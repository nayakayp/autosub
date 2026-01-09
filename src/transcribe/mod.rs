pub mod gemini;
pub mod orchestrator;
pub mod whisper;

pub use gemini::GeminiClient;
pub use orchestrator::TranscriptionOrchestrator;
pub use whisper::WhisperClient;

use crate::audio::AudioChunk;
use crate::config::{Config, Provider};
use crate::error::Result;
use async_trait::async_trait;
use std::time::Duration;

/// A word with its timestamp information.
#[derive(Debug, Clone)]
pub struct WordTimestamp {
    pub word: String,
    pub start: Duration,
    pub end: Duration,
}

/// A single segment of transcribed audio.
#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub text: String,
    pub start: Duration,
    pub end: Duration,
    pub words: Option<Vec<WordTimestamp>>,
    pub confidence: Option<f64>,
    pub speaker: Option<String>,
}

/// Complete transcription result from processing an audio chunk.
#[derive(Debug, Clone)]
pub struct Transcript {
    pub segments: Vec<TranscriptSegment>,
    pub language: Option<String>,
    pub duration: Option<Duration>,
}

impl Transcript {
    /// Create an empty transcript.
    pub fn empty() -> Self {
        Self {
            segments: Vec::new(),
            language: None,
            duration: None,
        }
    }

    /// Create a transcript with a single segment.
    pub fn single(segment: TranscriptSegment) -> Self {
        Self {
            segments: vec![segment],
            language: None,
            duration: None,
        }
    }
}

/// Complete result of transcribing all audio chunks.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptSegment>,
    pub language: String,
    pub duration: Duration,
}

/// Trait for transcription providers (Whisper, Gemini, etc.).
#[async_trait]
pub trait Transcriber: Send + Sync {
    /// Transcribe an audio chunk and return the transcript.
    async fn transcribe(&self, chunk: &AudioChunk) -> Result<Transcript>;

    /// Get the provider name for display.
    fn name(&self) -> &'static str;

    /// Maximum file size supported by this provider (in bytes).
    fn max_file_size(&self) -> usize;

    /// Supported audio formats.
    fn supported_formats(&self) -> &[&str];
}

/// Factory function to create a transcriber based on the provider.
pub fn create_transcriber(provider: Provider, config: &Config) -> Result<Box<dyn Transcriber>> {
    match provider {
        Provider::Whisper => {
            let api_key = config
                .openai_api_key
                .as_ref()
                .ok_or_else(|| crate::error::AutosubError::Config(
                    "OpenAI API key not set. Set OPENAI_API_KEY environment variable.".to_string(),
                ))?;
            Ok(Box::new(WhisperClient::new(api_key.clone())))
        }
        Provider::Gemini => {
            let api_key = config
                .gemini_api_key
                .as_ref()
                .ok_or_else(|| crate::error::AutosubError::Config(
                    "Gemini API key not set. Set GEMINI_API_KEY environment variable.".to_string(),
                ))?;
            Ok(Box::new(GeminiClient::new(api_key.clone())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_empty() {
        let t = Transcript::empty();
        assert!(t.segments.is_empty());
        assert!(t.language.is_none());
    }

    #[test]
    fn test_transcript_single() {
        let segment = TranscriptSegment {
            text: "Hello world".to_string(),
            start: Duration::from_secs(0),
            end: Duration::from_secs(2),
            words: None,
            confidence: Some(0.95),
            speaker: None,
        };
        let t = Transcript::single(segment.clone());
        assert_eq!(t.segments.len(), 1);
        assert_eq!(t.segments[0].text, "Hello world");
    }
}
