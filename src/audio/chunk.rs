use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::{debug, info};

use crate::error::{AutosubError, Result};

use super::extract::extract_audio_segment;
use super::{AudioChunk, SpeechRegion};

/// Configuration for audio chunking.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum chunk duration (API limit).
    pub max_duration: Duration,

    /// Maximum chunk file size in bytes (for Whisper API: 25MB).
    pub max_file_size: usize,

    /// Target chunk duration for optimal processing.
    pub target_duration: Duration,

    /// Padding to add around speech regions.
    pub padding: Duration,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self::gemini()
    }
}

impl ChunkConfig {
    /// Configuration optimized for Gemini API.
    pub fn gemini() -> Self {
        Self {
            max_duration: Duration::from_secs(60),
            max_file_size: 20 * 1024 * 1024,
            target_duration: Duration::from_secs(30),
            padding: Duration::from_millis(200),
        }
    }
}

/// Plan chunks based on speech regions.
///
/// This merges close regions and splits long ones to respect API limits.
pub fn plan_chunks(
    regions: &[SpeechRegion],
    total_duration: Duration,
    config: &ChunkConfig,
) -> Vec<SpeechRegion> {
    if regions.is_empty() {
        return plan_fixed_chunks(total_duration, config.target_duration);
    }

    let mut result = Vec::new();
    let mut current_start: Option<Duration> = None;
    let mut current_end = Duration::ZERO;

    for region in regions {
        let padded_start = region.start.saturating_sub(config.padding);
        let padded_end = (region.end + config.padding).min(total_duration);

        if current_start.is_none() {
            current_start = Some(padded_start);
            current_end = padded_end;
            continue;
        }

        let start = current_start.unwrap();
        let potential_duration = padded_end.saturating_sub(start);

        if potential_duration > config.max_duration {
            result.push(SpeechRegion {
                start,
                end: current_end,
            });
            current_start = Some(padded_start);
            current_end = padded_end;
        } else {
            current_end = padded_end;
        }
    }

    if let Some(start) = current_start {
        result.push(SpeechRegion {
            start,
            end: current_end,
        });
    }

    let mut final_chunks = Vec::new();
    for chunk in result {
        let duration = chunk.end.saturating_sub(chunk.start);
        if duration > config.max_duration {
            final_chunks.extend(split_long_region(&chunk, config.max_duration));
        } else {
            final_chunks.push(chunk);
        }
    }

    final_chunks
}

/// Plan fixed-duration chunks when no VAD regions available.
fn plan_fixed_chunks(total_duration: Duration, chunk_duration: Duration) -> Vec<SpeechRegion> {
    let mut chunks = Vec::new();
    let mut current = Duration::ZERO;

    while current < total_duration {
        let end = (current + chunk_duration).min(total_duration);
        chunks.push(SpeechRegion {
            start: current,
            end,
        });
        current = end;
    }

    chunks
}

/// Split a long region into smaller chunks.
fn split_long_region(region: &SpeechRegion, max_duration: Duration) -> Vec<SpeechRegion> {
    let mut chunks = Vec::new();
    let mut current = region.start;

    while current < region.end {
        let end = (current + max_duration).min(region.end);
        chunks.push(SpeechRegion {
            start: current,
            end,
        });
        current = end;
    }

    chunks
}

/// Create audio chunk files from planned regions.
pub async fn create_chunks(
    source_audio: &Path,
    regions: &[SpeechRegion],
    output_dir: &Path,
) -> Result<Vec<AudioChunk>> {
    if !source_audio.exists() {
        return Err(AutosubError::FileNotFound(
            source_audio.display().to_string(),
        ));
    }

    std::fs::create_dir_all(output_dir).map_err(|e| {
        AutosubError::AudioExtraction(format!("Failed to create output directory: {e}"))
    })?;

    info!(
        "Creating {} audio chunks in {}",
        regions.len(),
        output_dir.display()
    );

    let mut chunks = Vec::new();

    for (index, region) in regions.iter().enumerate() {
        let chunk_path = output_dir.join(format!("chunk_{:04}.wav", index));

        debug!(
            "Creating chunk {}: {:?} to {:?}",
            index, region.start, region.end
        );

        let _metadata =
            extract_audio_segment(source_audio, &chunk_path, region.start, region.end).await?;

        chunks.push(AudioChunk {
            region: region.clone(),
            path: chunk_path,
            index,
        });
    }

    info!("Created {} audio chunks", chunks.len());
    Ok(chunks)
}

/// Clean up chunk files.
pub fn cleanup_chunks(chunks: &[AudioChunk]) -> Result<()> {
    for chunk in chunks {
        if chunk.path.exists() {
            std::fs::remove_file(&chunk.path).map_err(|e| {
                AutosubError::AudioExtraction(format!(
                    "Failed to remove chunk file {}: {e}",
                    chunk.path.display()
                ))
            })?;
        }
    }
    Ok(())
}

/// Get temporary directory for chunk storage.
pub fn get_temp_chunk_dir() -> PathBuf {
    std::env::temp_dir().join("autosub_chunks")
}

/// Estimate file size for a WAV chunk (16-bit mono 16kHz).
pub fn estimate_wav_size(duration: Duration) -> usize {
    const SAMPLE_RATE: usize = 16000;
    const BYTES_PER_SAMPLE: usize = 2;
    const CHANNELS: usize = 1;
    const WAV_HEADER_SIZE: usize = 44;

    let samples = (duration.as_secs_f64() * SAMPLE_RATE as f64) as usize;
    WAV_HEADER_SIZE + (samples * BYTES_PER_SAMPLE * CHANNELS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert!(config.max_duration > Duration::ZERO);
        assert!(config.max_file_size > 0);
    }

    #[test]
    fn test_chunk_config_gemini() {
        let config = ChunkConfig::gemini();
        assert_eq!(config.max_file_size, 20 * 1024 * 1024);
    }

    #[test]
    fn test_plan_fixed_chunks() {
        let total = Duration::from_secs(100);
        let chunk_size = Duration::from_secs(30);
        let chunks = plan_fixed_chunks(total, chunk_size);

        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].start, Duration::ZERO);
        assert_eq!(chunks[0].end, Duration::from_secs(30));
        assert_eq!(chunks[3].start, Duration::from_secs(90));
        assert_eq!(chunks[3].end, Duration::from_secs(100));
    }

    #[test]
    fn test_split_long_region() {
        let region = SpeechRegion {
            start: Duration::from_secs(0),
            end: Duration::from_secs(150),
        };
        let max = Duration::from_secs(60);
        let chunks = split_long_region(&region, max);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].end - chunks[0].start, Duration::from_secs(60));
        assert_eq!(chunks[1].end - chunks[1].start, Duration::from_secs(60));
        assert_eq!(chunks[2].end - chunks[2].start, Duration::from_secs(30));
    }

    #[test]
    fn test_plan_chunks_empty_regions() {
        let config = ChunkConfig::default();
        let chunks = plan_chunks(&[], Duration::from_secs(60), &config);

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_plan_chunks_merges_close_regions() {
        let config = ChunkConfig {
            max_duration: Duration::from_secs(60),
            target_duration: Duration::from_secs(30),
            padding: Duration::from_millis(100),
            ..Default::default()
        };

        let regions = vec![
            SpeechRegion {
                start: Duration::from_secs(1),
                end: Duration::from_secs(5),
            },
            SpeechRegion {
                start: Duration::from_secs(6),
                end: Duration::from_secs(10),
            },
        ];

        let chunks = plan_chunks(&regions, Duration::from_secs(60), &config);

        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_plan_chunks_splits_long_regions() {
        let config = ChunkConfig {
            max_duration: Duration::from_secs(30),
            ..Default::default()
        };

        let regions = vec![SpeechRegion {
            start: Duration::from_secs(0),
            end: Duration::from_secs(90),
        }];

        let chunks = plan_chunks(&regions, Duration::from_secs(100), &config);

        assert!(chunks.len() >= 3);
    }

    #[test]
    fn test_estimate_wav_size() {
        let duration = Duration::from_secs(60);
        let size = estimate_wav_size(duration);

        let expected = 44 + (60 * 16000 * 2);
        assert_eq!(size, expected);
    }

    #[test]
    fn test_get_temp_chunk_dir() {
        let dir = get_temp_chunk_dir();
        assert!(dir.to_string_lossy().contains("autosub_chunks"));
    }
}
