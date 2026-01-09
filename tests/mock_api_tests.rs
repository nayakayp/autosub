//! Mock API tests for transcription providers
//!
//! These tests validate client creation and configuration without hitting real endpoints.

use autosub::audio::{AudioChunk, SpeechRegion};
use autosub::transcribe::{GeminiClient, Transcriber, TranscriptionOrchestrator, WhisperClient};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Whisper API Mock Tests
// ============================================================================

mod whisper_tests {
    use super::*;

    fn create_test_chunk() -> AudioChunk {
        AudioChunk {
            region: SpeechRegion {
                start: Duration::from_secs(0),
                end: Duration::from_secs(5),
            },
            path: PathBuf::from("/tmp/nonexistent_test.wav"),
            index: 0,
        }
    }

    #[tokio::test]
    async fn test_whisper_client_creation() {
        let client = WhisperClient::new("test-api-key".to_string());
        assert_eq!(client.name(), "OpenAI Whisper");
    }

    #[tokio::test]
    async fn test_whisper_max_file_size() {
        let client = WhisperClient::new("test-api-key".to_string());
        assert_eq!(client.max_file_size(), 25 * 1024 * 1024); // 25MB
    }

    #[tokio::test]
    async fn test_whisper_client_with_language() {
        let client = WhisperClient::new("test-api-key".to_string()).with_language("ja".to_string());
        assert_eq!(client.name(), "OpenAI Whisper");
    }

    #[tokio::test]
    async fn test_whisper_client_with_prompt() {
        let client = WhisperClient::new("test-api-key".to_string())
            .with_prompt("Custom vocabulary: AI, ML".to_string());
        assert_eq!(client.name(), "OpenAI Whisper");
    }

    #[tokio::test]
    async fn test_whisper_handles_missing_file() {
        let client = WhisperClient::new("test-api-key".to_string());
        let chunk = create_test_chunk();

        let result = client.transcribe(&chunk).await;

        // Should fail because the file doesn't exist
        assert!(result.is_err());
    }
}

// ============================================================================
// Gemini API Mock Tests
// ============================================================================

mod gemini_tests {
    use super::*;

    fn create_test_chunk() -> AudioChunk {
        AudioChunk {
            region: SpeechRegion {
                start: Duration::from_secs(0),
                end: Duration::from_secs(5),
            },
            path: PathBuf::from("/tmp/nonexistent_test.wav"),
            index: 0,
        }
    }

    #[tokio::test]
    async fn test_gemini_client_creation() {
        let client = GeminiClient::new("test-api-key".to_string());
        assert_eq!(client.name(), "Google Gemini");
    }

    #[tokio::test]
    async fn test_gemini_max_file_size() {
        let client = GeminiClient::new("test-api-key".to_string());
        // 200MB limit for Gemini (per their Files API)
        assert_eq!(client.max_file_size(), 200 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_gemini_client_with_language() {
        let client = GeminiClient::new("test-api-key".to_string()).with_language("en".to_string());
        assert_eq!(client.name(), "Google Gemini");
    }

    #[tokio::test]
    async fn test_gemini_client_with_diarization() {
        let client = GeminiClient::new("test-api-key".to_string()).with_diarization(true);
        assert_eq!(client.name(), "Google Gemini");
    }

    #[tokio::test]
    async fn test_gemini_handles_missing_file() {
        let client = GeminiClient::new("test-api-key".to_string());
        let chunk = create_test_chunk();

        let result = client.transcribe(&chunk).await;

        // Should fail because the file doesn't exist
        assert!(result.is_err());
    }
}

// ============================================================================
// Transcription Orchestrator Tests
// ============================================================================

mod orchestrator_tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let client: Box<dyn Transcriber> = Box::new(WhisperClient::new("test-api-key".to_string()));
        let _orchestrator = TranscriptionOrchestrator::new(client, 4);

        // Just verify it compiles and creates successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_orchestrator_empty_chunks() {
        let client: Box<dyn Transcriber> = Box::new(WhisperClient::new("test-api-key".to_string()));
        let orchestrator = TranscriptionOrchestrator::new(client, 4);

        let chunks: Vec<AudioChunk> = vec![];
        let result = orchestrator.process_chunks(chunks).await;

        assert!(result.is_ok());
        let (transcription_result, _stats) = result.unwrap();
        assert!(transcription_result.segments.is_empty());
    }

    #[tokio::test]
    async fn test_orchestrator_with_progress_disabled() {
        let client: Box<dyn Transcriber> = Box::new(GeminiClient::new("test-api-key".to_string()));
        let _orchestrator = TranscriptionOrchestrator::new(client, 4).with_progress(false);

        assert!(true);
    }
}

// ============================================================================
// Response Parsing Tests
// ============================================================================

mod response_parsing_tests {
    use autosub::transcribe::TranscriptSegment;
    use std::time::Duration;

    #[test]
    fn test_transcript_segment_creation() {
        let segment = TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(5),
            text: "Hello world".to_string(),
            speaker: Some("Speaker 1".to_string()),
            confidence: Some(0.95),
            words: None,
        };

        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.speaker, Some("Speaker 1".to_string()));
        assert_eq!(segment.confidence, Some(0.95));
    }

    #[test]
    fn test_transcript_segment_without_optional_fields() {
        let segment = TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(3),
            text: "Simple text".to_string(),
            speaker: None,
            confidence: None,
            words: None,
        };

        assert_eq!(segment.text, "Simple text");
        assert!(segment.speaker.is_none());
        assert!(segment.confidence.is_none());
        assert!(segment.words.is_none());
    }
}

// ============================================================================
// Create Transcriber Factory Tests
// ============================================================================

mod factory_tests {
    use autosub::config::{Config, Provider};
    use autosub::transcribe::create_transcriber;

    #[test]
    fn test_create_whisper_transcriber() {
        let mut config = Config::default();
        config.openai_api_key = Some("test-key".to_string());

        let transcriber = create_transcriber(Provider::Whisper, &config).unwrap();
        assert_eq!(transcriber.name(), "OpenAI Whisper");
    }

    #[test]
    fn test_create_gemini_transcriber() {
        let mut config = Config::default();
        config.gemini_api_key = Some("test-key".to_string());

        let transcriber = create_transcriber(Provider::Gemini, &config).unwrap();
        assert_eq!(transcriber.name(), "Google Gemini");
    }

    #[test]
    fn test_create_transcriber_missing_whisper_key() {
        let mut config = Config::default();
        config.openai_api_key = None;

        let result = create_transcriber(Provider::Whisper, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_transcriber_missing_gemini_key() {
        let mut config = Config::default();
        config.gemini_api_key = None;

        let result = create_transcriber(Provider::Gemini, &config);
        assert!(result.is_err());
    }
}

// ============================================================================
// Transcription Result Tests
// ============================================================================

mod result_tests {
    use autosub::transcribe::{TranscriptSegment, TranscriptionResult};
    use std::time::Duration;

    #[test]
    fn test_transcription_result_empty() {
        let result = TranscriptionResult {
            segments: vec![],
            language: "en".to_string(),
            duration: Duration::ZERO,
        };

        assert!(result.segments.is_empty());
        assert_eq!(result.language, "en");
    }

    #[test]
    fn test_transcription_result_with_segments() {
        let segments = vec![
            TranscriptSegment {
                start: Duration::from_secs(0),
                end: Duration::from_secs(5),
                text: "First".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_secs(5),
                end: Duration::from_secs(10),
                text: "Second".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
        ];

        let result = TranscriptionResult {
            segments,
            language: "ja".to_string(),
            duration: Duration::from_secs(10),
        };

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.language, "ja");
        assert_eq!(result.duration, Duration::from_secs(10));
    }
}
