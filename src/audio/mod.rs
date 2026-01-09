pub mod chunk;
pub mod extract;
pub mod vad;

pub use chunk::{
    cleanup_chunks, create_chunks, estimate_wav_size, get_temp_chunk_dir, plan_chunks, ChunkConfig,
};
pub use extract::{
    check_ffmpeg, check_ffprobe, extract_audio, extract_audio_segment, extract_audio_with_progress,
    get_audio_duration, get_audio_info,
};
pub use vad::{detect_speech_regions, has_speech, total_speech_duration, VadConfig};

use std::path::PathBuf;
use std::time::Duration;

/// Metadata about an audio file.
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub duration: Duration,
    pub sample_rate: u32,
    pub channels: u16,
}

/// A region of speech detected in audio.
#[derive(Debug, Clone)]
pub struct SpeechRegion {
    pub start: Duration,
    pub end: Duration,
}

impl SpeechRegion {
    /// Get the duration of this speech region.
    pub fn duration(&self) -> Duration {
        self.end.saturating_sub(self.start)
    }
}

/// A chunk of audio ready for transcription.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub region: SpeechRegion,
    pub path: PathBuf,
    pub index: usize,
}

impl AudioChunk {
    /// Get the duration of this audio chunk.
    pub fn duration(&self) -> Duration {
        self.region.duration()
    }
}
