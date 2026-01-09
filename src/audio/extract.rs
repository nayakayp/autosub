use std::path::Path;
use std::process::Command;
use std::time::Duration;

use tracing::{debug, info};

use crate::error::{AutosubError, Result};

use super::AudioMetadata;

/// Check if FFmpeg is installed and accessible.
pub fn check_ffmpeg() -> Result<()> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|e| {
            AutosubError::AudioExtraction(format!(
                "FFmpeg not found. Please install FFmpeg and ensure it's in your PATH. Error: {e}"
            ))
        })?;

    if !output.status.success() {
        return Err(AutosubError::AudioExtraction(
            "FFmpeg check failed".to_string(),
        ));
    }

    debug!("FFmpeg is available");
    Ok(())
}

/// Check if FFprobe is installed and accessible.
pub fn check_ffprobe() -> Result<()> {
    let output = Command::new("ffprobe")
        .arg("-version")
        .output()
        .map_err(|e| {
            AutosubError::AudioExtraction(format!(
                "FFprobe not found. Please install FFmpeg (includes FFprobe). Error: {e}"
            ))
        })?;

    if !output.status.success() {
        return Err(AutosubError::AudioExtraction(
            "FFprobe check failed".to_string(),
        ));
    }

    debug!("FFprobe is available");
    Ok(())
}

/// Get audio duration using FFprobe.
pub fn get_audio_duration(input: &Path) -> Result<Duration> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to run FFprobe: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AutosubError::AudioExtraction(format!(
            "FFprobe failed: {stderr}"
        )));
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    let duration_secs: f64 = duration_str.trim().parse().map_err(|e| {
        AutosubError::AudioExtraction(format!(
            "Failed to parse duration '{}': {e}",
            duration_str.trim()
        ))
    })?;

    Ok(Duration::from_secs_f64(duration_secs))
}

/// Get audio metadata (sample rate, channels) using FFprobe.
pub fn get_audio_info(input: &Path) -> Result<(u32, u16)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=sample_rate,channels",
            "-of",
            "csv=s=,:p=0",
        ])
        .arg(input)
        .output()
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to run FFprobe: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AutosubError::AudioExtraction(format!(
            "FFprobe failed: {stderr}"
        )));
    }

    let info_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = info_str.trim().split(',').collect();

    if parts.len() < 2 {
        return Err(AutosubError::AudioExtraction(format!(
            "Failed to parse audio info: {}",
            info_str.trim()
        )));
    }

    let sample_rate: u32 = parts[0].parse().map_err(|e| {
        AutosubError::AudioExtraction(format!("Failed to parse sample rate: {e}"))
    })?;

    let channels: u16 = parts[1]
        .parse()
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to parse channels: {e}")))?;

    Ok((sample_rate, channels))
}

/// Extract audio from a video/audio file and convert to WAV format.
///
/// The output is mono 16-bit PCM at 16kHz, which is optimal for speech recognition.
pub async fn extract_audio(input: &Path, output: &Path) -> Result<AudioMetadata> {
    check_ffmpeg()?;
    check_ffprobe()?;

    if !input.exists() {
        return Err(AutosubError::FileNotFound(
            input.display().to_string(),
        ));
    }

    info!("Extracting audio from {}", input.display());

    let duration = get_audio_duration(input)?;
    debug!("Input duration: {:?}", duration);

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
        ])
        .arg(input)
        .args([
            "-vn",
            "-acodec",
            "pcm_s16le",
            "-ar",
            "16000",
            "-ac",
            "1",
        ])
        .arg(output)
        .status()
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to run FFmpeg: {e}")))?;

    if !status.success() {
        return Err(AutosubError::AudioExtraction(
            "FFmpeg audio extraction failed".to_string(),
        ));
    }

    if !output.exists() {
        return Err(AutosubError::AudioExtraction(
            "Output file was not created".to_string(),
        ));
    }

    info!("Audio extracted to {}", output.display());

    Ok(AudioMetadata {
        duration,
        sample_rate: 16000,
        channels: 1,
    })
}

/// Extract audio with progress callback.
///
/// This version spawns FFmpeg and monitors its progress output.
pub async fn extract_audio_with_progress<F>(
    input: &Path,
    output: &Path,
    mut progress_callback: F,
) -> Result<AudioMetadata>
where
    F: FnMut(f64),
{
    check_ffmpeg()?;
    check_ffprobe()?;

    if !input.exists() {
        return Err(AutosubError::FileNotFound(
            input.display().to_string(),
        ));
    }

    info!("Extracting audio from {}", input.display());

    let duration = get_audio_duration(input)?;
    let duration_secs = duration.as_secs_f64();
    debug!("Input duration: {:.2}s", duration_secs);

    let mut child = std::process::Command::new("ffmpeg")
        .args(["-y", "-progress", "pipe:1", "-i"])
        .arg(input)
        .args(["-vn", "-acodec", "pcm_s16le", "-ar", "16000", "-ac", "1"])
        .arg(output)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to spawn FFmpeg: {e}")))?;

    if let Some(stdout) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);

        for line in reader.lines().map_while(|l| l.ok()) {
            if line.starts_with("out_time_us=") {
                if let Ok(time_us) = line.trim_start_matches("out_time_us=").parse::<i64>() {
                    if time_us > 0 {
                        let current_secs = time_us as f64 / 1_000_000.0;
                        let progress = (current_secs / duration_secs).min(1.0);
                        progress_callback(progress);
                    }
                }
            }
        }
    }

    let status = child.wait().map_err(|e| {
        AutosubError::AudioExtraction(format!("Failed to wait for FFmpeg: {e}"))
    })?;

    if !status.success() {
        return Err(AutosubError::AudioExtraction(
            "FFmpeg audio extraction failed".to_string(),
        ));
    }

    progress_callback(1.0);

    if !output.exists() {
        return Err(AutosubError::AudioExtraction(
            "Output file was not created".to_string(),
        ));
    }

    info!("Audio extracted to {}", output.display());

    Ok(AudioMetadata {
        duration,
        sample_rate: 16000,
        channels: 1,
    })
}

/// Extract a segment of audio between start and end times.
pub async fn extract_audio_segment(
    input: &Path,
    output: &Path,
    start: Duration,
    end: Duration,
) -> Result<AudioMetadata> {
    check_ffmpeg()?;

    if !input.exists() {
        return Err(AutosubError::FileNotFound(
            input.display().to_string(),
        ));
    }

    let duration = end.saturating_sub(start);
    if duration.is_zero() {
        return Err(AutosubError::AudioExtraction(
            "Segment duration is zero".to_string(),
        ));
    }

    let start_secs = format!("{:.3}", start.as_secs_f64());
    let duration_secs = format!("{:.3}", duration.as_secs_f64());

    debug!(
        "Extracting segment: start={}, duration={}",
        start_secs, duration_secs
    );

    let status = Command::new("ffmpeg")
        .args(["-y", "-ss"])
        .arg(&start_secs)
        .args(["-t"])
        .arg(&duration_secs)
        .args(["-i"])
        .arg(input)
        .args(["-vn", "-acodec", "pcm_s16le", "-ar", "16000", "-ac", "1"])
        .arg(output)
        .status()
        .map_err(|e| AutosubError::AudioExtraction(format!("Failed to run FFmpeg: {e}")))?;

    if !status.success() {
        return Err(AutosubError::AudioExtraction(
            "FFmpeg segment extraction failed".to_string(),
        ));
    }

    Ok(AudioMetadata {
        duration,
        sample_rate: 16000,
        channels: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ffmpeg_available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_check_ffmpeg() {
        let result = check_ffmpeg();
        if !ffmpeg_available() {
            eprintln!("Skipping test: FFmpeg not available or broken");
            return;
        }
        assert!(result.is_ok(), "FFmpeg check failed: {:?}", result.err());
    }

    #[test]
    fn test_check_ffprobe() {
        let result = check_ffprobe();
        if !Command::new("ffprobe")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            eprintln!("Skipping test: FFprobe not available or broken");
            return;
        }
        assert!(result.is_ok(), "FFprobe check failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_extract_audio_file_not_found() {
        if !ffmpeg_available() {
            eprintln!("Skipping test: FFmpeg not available");
            return;
        }

        let result =
            extract_audio(Path::new("/nonexistent/file.mp4"), Path::new("/tmp/out.wav")).await;
        assert!(result.is_err());
        match &result {
            Err(AutosubError::FileNotFound(path)) => {
                assert!(path.contains("nonexistent"));
            }
            Err(other) => {
                panic!("Expected FileNotFound error, got: {other}");
            }
            Ok(_) => {
                panic!("Expected error but got Ok");
            }
        }
    }
}
