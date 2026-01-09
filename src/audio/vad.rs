use std::path::Path;
use std::time::Duration;

use hound::WavReader;
use tracing::{debug, info};

use crate::error::{AutosubError, Result};

use super::SpeechRegion;

/// Configuration for Voice Activity Detection.
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// RMS energy threshold for speech detection (0.0 to 1.0).
    /// Lower values are more sensitive to quiet speech.
    pub energy_threshold: f32,

    /// Minimum duration of speech to be considered a valid region.
    pub min_speech_duration: Duration,

    /// Minimum duration of silence to split regions.
    pub min_silence_duration: Duration,

    /// Size of analysis window in samples.
    pub window_size: usize,

    /// Hop size between windows in samples.
    pub hop_size: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.01,
            min_speech_duration: Duration::from_millis(250),
            min_silence_duration: Duration::from_millis(500),
            window_size: 1600,
            hop_size: 800,
        }
    }
}

/// Calculate RMS (Root Mean Square) energy of a sample window.
fn calculate_rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f64 = samples
        .iter()
        .map(|&s| {
            let normalized = s as f64 / i16::MAX as f64;
            normalized * normalized
        })
        .sum();

    (sum_squares / samples.len() as f64).sqrt() as f32
}

/// Detect speech regions in a WAV audio file.
///
/// Returns a list of time regions where speech was detected.
pub fn detect_speech_regions(audio_path: &Path, config: &VadConfig) -> Result<Vec<SpeechRegion>> {
    let reader = WavReader::open(audio_path)
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to open WAV file: {e}")))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    info!(
        "Analyzing audio: {} Hz, {} channels, {} bits",
        sample_rate, spec.channels, spec.bits_per_sample
    );

    let samples: Vec<i16> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .into_samples::<i16>()
            .map(|s| s.unwrap_or(0))
            .collect(),
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|s| (s.unwrap_or(0.0) * i16::MAX as f32) as i16)
            .collect(),
    };

    if samples.is_empty() {
        return Ok(vec![]);
    }

    debug!("Total samples: {}", samples.len());

    let energy_values = compute_energy_profile(&samples, config.window_size, config.hop_size);

    let speech_frames = detect_speech_frames(&energy_values, config.energy_threshold);

    let regions = frames_to_regions(
        &speech_frames,
        sample_rate,
        config.hop_size,
        config.min_speech_duration,
        config.min_silence_duration,
    );

    let total_duration = Duration::from_secs_f64(samples.len() as f64 / sample_rate as f64);
    info!(
        "Detected {} speech regions in {:.2}s of audio",
        regions.len(),
        total_duration.as_secs_f64()
    );

    Ok(regions)
}

/// Compute energy profile using sliding window.
fn compute_energy_profile(samples: &[i16], window_size: usize, hop_size: usize) -> Vec<f32> {
    let mut energy_values = Vec::new();
    let mut pos = 0;

    while pos + window_size <= samples.len() {
        let window = &samples[pos..pos + window_size];
        let rms = calculate_rms(window);
        energy_values.push(rms);
        pos += hop_size;
    }

    energy_values
}

/// Classify frames as speech (true) or silence (false).
fn detect_speech_frames(energy_values: &[f32], threshold: f32) -> Vec<bool> {
    energy_values.iter().map(|&e| e >= threshold).collect()
}

/// Convert speech frames to time regions with merging and filtering.
fn frames_to_regions(
    speech_frames: &[bool],
    sample_rate: u32,
    hop_size: usize,
    min_speech_duration: Duration,
    min_silence_duration: Duration,
) -> Vec<SpeechRegion> {
    if speech_frames.is_empty() {
        return vec![];
    }

    let frame_duration = hop_size as f64 / sample_rate as f64;
    let min_speech_frames = (min_speech_duration.as_secs_f64() / frame_duration).ceil() as usize;
    let min_silence_frames = (min_silence_duration.as_secs_f64() / frame_duration).ceil() as usize;

    let mut raw_regions: Vec<(usize, usize)> = Vec::new();
    let mut in_speech = false;
    let mut start_frame = 0;

    for (i, &is_speech) in speech_frames.iter().enumerate() {
        if is_speech && !in_speech {
            in_speech = true;
            start_frame = i;
        } else if !is_speech && in_speech {
            in_speech = false;
            raw_regions.push((start_frame, i));
        }
    }

    if in_speech {
        raw_regions.push((start_frame, speech_frames.len()));
    }

    let mut merged_regions: Vec<(usize, usize)> = Vec::new();
    for (start, end) in raw_regions {
        if let Some((_last_start, last_end)) = merged_regions.last_mut() {
            if start.saturating_sub(*last_end) < min_silence_frames {
                *last_end = end;
                continue;
            }
        }
        merged_regions.push((start, end));
    }

    merged_regions
        .into_iter()
        .filter(|(start, end)| end - start >= min_speech_frames)
        .map(|(start, end)| {
            let start_time = start as f64 * frame_duration;
            let end_time = end as f64 * frame_duration;
            SpeechRegion {
                start: Duration::from_secs_f64(start_time),
                end: Duration::from_secs_f64(end_time),
            }
        })
        .collect()
}

/// Detect if audio file has any speech content.
pub fn has_speech(audio_path: &Path, config: &VadConfig) -> Result<bool> {
    let regions = detect_speech_regions(audio_path, config)?;
    Ok(!regions.is_empty())
}

/// Get total speech duration from detected regions.
pub fn total_speech_duration(regions: &[SpeechRegion]) -> Duration {
    regions.iter().map(|r| r.end.saturating_sub(r.start)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rms_silence() {
        let samples = vec![0i16; 100];
        assert_eq!(calculate_rms(&samples), 0.0);
    }

    #[test]
    fn test_calculate_rms_loud() {
        let samples = vec![i16::MAX; 100];
        let rms = calculate_rms(&samples);
        assert!((rms - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_rms_mixed() {
        let samples: Vec<i16> = (0..100)
            .map(|i| if i % 2 == 0 { 1000 } else { -1000 })
            .collect();
        let rms = calculate_rms(&samples);
        assert!(rms > 0.0);
        assert!(rms < 1.0);
    }

    #[test]
    fn test_detect_speech_frames() {
        let energy = vec![0.001, 0.02, 0.03, 0.005, 0.001];
        let threshold = 0.01;
        let frames = detect_speech_frames(&energy, threshold);
        assert_eq!(frames, vec![false, true, true, false, false]);
    }

    #[test]
    fn test_frames_to_regions_basic() {
        let frames = vec![false, true, true, true, false, false, true, true, false];
        let regions = frames_to_regions(
            &frames,
            16000,
            800,
            Duration::from_millis(100),
            Duration::from_millis(100),
        );
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn test_frames_to_regions_merge_short_silence() {
        let frames = vec![true, true, false, true, true];
        let regions = frames_to_regions(
            &frames,
            16000,
            800,
            Duration::from_millis(50),
            Duration::from_millis(500),
        );
        assert_eq!(regions.len(), 1);
    }

    #[test]
    fn test_total_speech_duration() {
        let regions = vec![
            SpeechRegion {
                start: Duration::from_secs(0),
                end: Duration::from_secs(5),
            },
            SpeechRegion {
                start: Duration::from_secs(10),
                end: Duration::from_secs(15),
            },
        ];
        let total = total_speech_duration(&regions);
        assert_eq!(total, Duration::from_secs(10));
    }

    #[test]
    fn test_vad_config_default() {
        let config = VadConfig::default();
        assert!(config.energy_threshold > 0.0);
        assert!(config.min_speech_duration > Duration::ZERO);
        assert!(config.min_silence_duration > Duration::ZERO);
    }
}
