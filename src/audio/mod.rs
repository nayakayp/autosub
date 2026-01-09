pub mod chunk;
pub mod extract;
pub mod vad;

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub duration: Duration,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub struct SpeechRegion {
    pub start: Duration,
    pub end: Duration,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub region: SpeechRegion,
    pub path: PathBuf,
    pub index: usize,
}
