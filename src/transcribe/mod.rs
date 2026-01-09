pub mod gemini;
pub mod whisper;

use crate::audio::AudioChunk;
use crate::error::Result;
use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub start: Duration,
    pub end: Duration,
    pub text: String,
    pub speaker: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Transcript {
    pub segments: Vec<TranscriptSegment>,
    pub language: Option<String>,
}

#[async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcribe(&self, audio: &AudioChunk) -> Result<Transcript>;
    fn name(&self) -> &'static str;
    fn max_chunk_duration(&self) -> Duration;
}
