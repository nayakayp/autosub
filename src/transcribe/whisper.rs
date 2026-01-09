use crate::audio::AudioChunk;
use crate::error::{AutosubError, Result};
use crate::transcribe::{Transcript, TranscriptSegment, Transcriber, WordTimestamp};
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, warn};

/// OpenAI Whisper API endpoint.
const WHISPER_API_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

/// Maximum file size for Whisper API (25 MB).
const MAX_FILE_SIZE: usize = 25 * 1024 * 1024;

/// Maximum retries for API calls.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds).
const BASE_DELAY_MS: u64 = 1000;

/// Whisper model variants.
#[derive(Debug, Clone, Copy, Default)]
pub enum WhisperModel {
    #[default]
    Whisper1,
    Gpt4oTranscribe,
    Gpt4oMiniTranscribe,
}

impl WhisperModel {
    fn as_str(&self) -> &'static str {
        match self {
            WhisperModel::Whisper1 => "whisper-1",
            WhisperModel::Gpt4oTranscribe => "gpt-4o-transcribe",
            WhisperModel::Gpt4oMiniTranscribe => "gpt-4o-mini-transcribe",
        }
    }
}

/// OpenAI Whisper API client.
pub struct WhisperClient {
    client: reqwest::Client,
    api_key: String,
    model: WhisperModel,
    language: Option<String>,
    prompt: Option<String>,
}

impl WhisperClient {
    /// Create a new Whisper client with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: WhisperModel::default(),
            language: None,
            prompt: None,
        }
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: WhisperModel) -> Self {
        self.model = model;
        self
    }

    /// Set the source language (ISO 639-1 code).
    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    /// Set a prompt for vocabulary hints (max 224 tokens).
    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.prompt = Some(prompt);
        self
    }

    /// Build the multipart form for the API request.
    async fn build_form(&self, audio_path: &Path) -> Result<Form> {
        let file_bytes = fs::read(audio_path).await?;
        let file_name = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav")
            .to_string();

        let mime_type = match audio_path.extension().and_then(|e| e.to_str()) {
            Some("wav") => "audio/wav",
            Some("mp3") => "audio/mpeg",
            Some("m4a") => "audio/mp4",
            Some("flac") => "audio/flac",
            Some("ogg") => "audio/ogg",
            Some("webm") => "audio/webm",
            _ => "application/octet-stream",
        };

        let file_part = Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str(mime_type)?;

        let mut form = Form::new()
            .part("file", file_part)
            .text("model", self.model.as_str())
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "segment");

        if let Some(ref lang) = self.language {
            form = form.text("language", lang.clone());
        }

        if let Some(ref prompt) = self.prompt {
            form = form.text("prompt", prompt.clone());
        }

        Ok(form)
    }

    /// Make the API request (form is consumed, so no retries at this level).
    async fn call_api(&self, form: Form) -> Result<WhisperResponse> {
        let response = self
            .client
            .post(WHISPER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        debug!("Whisper API response status: {}", status);

        if status.is_success() {
            let body = response.text().await?;
            debug!("Whisper API response: {}", &body[..body.len().min(500)]);
            let parsed: WhisperResponse = serde_json::from_str(&body)?;
            return Ok(parsed);
        }

        // Handle error responses
        let error_body = response.text().await.unwrap_or_default();

        // Try to parse API error
        if let Ok(api_error) = serde_json::from_str::<ApiErrorResponse>(&error_body) {
            return Err(AutosubError::Api(format!(
                "Whisper API error: {} ({})",
                api_error.error.message, api_error.error.r#type
            )));
        }

        Err(AutosubError::Api(format!(
            "Whisper API error ({}): {}",
            status, error_body
        )))
    }

    /// Transcribe with retry logic - rebuilds form on each attempt.
    async fn transcribe_with_retry(&self, chunk: &AudioChunk) -> Result<WhisperResponse> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = BASE_DELAY_MS * 2u64.pow(attempt - 1);
                debug!("Retry attempt {} after {}ms delay", attempt, delay);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            let form = self.build_form(&chunk.path).await?;

            match self.call_api(form).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // Don't retry on client errors
                    let error_str = e.to_string();
                    if error_str.contains("API error (4") {
                        return Err(e);
                    }
                    warn!("Attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AutosubError::Api("Unknown error".to_string())))
    }

    /// Convert Whisper API response to our Transcript format.
    fn parse_response(&self, response: WhisperResponse, chunk: &AudioChunk) -> Transcript {
        let mut segments = Vec::new();

        if let Some(api_segments) = response.segments {
            for seg in api_segments {
                // Adjust timestamps relative to the chunk's position in the original audio
                let start = chunk.region.start + Duration::from_secs_f64(seg.start);
                let end = chunk.region.start + Duration::from_secs_f64(seg.end);

                segments.push(TranscriptSegment {
                    text: seg.text.trim().to_string(),
                    start,
                    end,
                    words: None, // Whisper segments don't include word-level by default
                    confidence: None,
                    speaker: None,
                });
            }
        } else {
            // Fallback: create a single segment with the full text
            segments.push(TranscriptSegment {
                text: response.text.trim().to_string(),
                start: chunk.region.start,
                end: chunk.region.end,
                words: None,
                confidence: None,
                speaker: None,
            });
        }

        // Parse word-level timestamps if available
        if let Some(words) = response.words {
            // If we have word timestamps, attach them to the appropriate segment
            let word_timestamps: Vec<WordTimestamp> = words
                .into_iter()
                .map(|w| WordTimestamp {
                    word: w.word,
                    start: chunk.region.start + Duration::from_secs_f64(w.start),
                    end: chunk.region.start + Duration::from_secs_f64(w.end),
                })
                .collect();

            // For simplicity, attach all words to the first segment
            if let Some(first_seg) = segments.first_mut() {
                first_seg.words = Some(word_timestamps);
            }
        }

        Transcript {
            segments,
            language: Some(response.language),
            duration: Some(Duration::from_secs_f64(response.duration)),
        }
    }
}

#[async_trait]
impl Transcriber for WhisperClient {
    async fn transcribe(&self, chunk: &AudioChunk) -> Result<Transcript> {
        debug!(
            "Transcribing chunk {} with Whisper: {:?}",
            chunk.index, chunk.path
        );

        // Check file size
        let metadata = fs::metadata(&chunk.path).await?;
        if metadata.len() as usize > MAX_FILE_SIZE {
            return Err(AutosubError::Transcription(format!(
                "File too large for Whisper API: {} bytes (max {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }

        let response = self.transcribe_with_retry(chunk).await?;
        let transcript = self.parse_response(response, chunk);

        debug!(
            "Whisper returned {} segments for chunk {}",
            transcript.segments.len(),
            chunk.index
        );

        Ok(transcript)
    }

    fn name(&self) -> &'static str {
        "OpenAI Whisper"
    }

    fn max_file_size(&self) -> usize {
        MAX_FILE_SIZE
    }

    fn supported_formats(&self) -> &[&str] {
        &["mp3", "mp4", "mpeg", "mpga", "m4a", "wav", "webm"]
    }
}

// API response types

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
    #[serde(default)]
    segments: Option<Vec<WhisperSegment>>,
    #[serde(default)]
    words: Option<Vec<WhisperWord>>,
    language: String,
    duration: f64,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Debug, Deserialize)]
struct WhisperWord {
    word: String,
    start: f64,
    end: f64,
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
    r#type: String,
    #[allow(dead_code)]
    code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SpeechRegion;
    use std::path::PathBuf;

    fn create_test_chunk() -> AudioChunk {
        AudioChunk {
            region: SpeechRegion {
                start: Duration::from_secs(10),
                end: Duration::from_secs(20),
            },
            path: PathBuf::from("/tmp/test.wav"),
            index: 0,
        }
    }

    #[test]
    fn test_whisper_model_str() {
        assert_eq!(WhisperModel::Whisper1.as_str(), "whisper-1");
        assert_eq!(WhisperModel::Gpt4oTranscribe.as_str(), "gpt-4o-transcribe");
    }

    #[test]
    fn test_parse_response_with_segments() {
        let client = WhisperClient::new("test-key".to_string());
        let chunk = create_test_chunk();

        let response = WhisperResponse {
            text: "Hello world. How are you?".to_string(),
            segments: Some(vec![
                WhisperSegment {
                    start: 0.0,
                    end: 2.0,
                    text: "Hello world.".to_string(),
                },
                WhisperSegment {
                    start: 2.5,
                    end: 4.0,
                    text: "How are you?".to_string(),
                },
            ]),
            words: None,
            language: "en".to_string(),
            duration: 4.0,
        };

        let transcript = client.parse_response(response, &chunk);
        assert_eq!(transcript.segments.len(), 2);
        assert_eq!(transcript.segments[0].text, "Hello world.");
        // Start should be chunk.region.start + segment.start
        assert_eq!(transcript.segments[0].start, Duration::from_secs(10));
        assert_eq!(transcript.segments[1].start, Duration::from_millis(12500));
    }

    #[test]
    fn test_parse_response_without_segments() {
        let client = WhisperClient::new("test-key".to_string());
        let chunk = create_test_chunk();

        let response = WhisperResponse {
            text: "Hello world".to_string(),
            segments: None,
            words: None,
            language: "en".to_string(),
            duration: 2.0,
        };

        let transcript = client.parse_response(response, &chunk);
        assert_eq!(transcript.segments.len(), 1);
        assert_eq!(transcript.segments[0].text, "Hello world");
        assert_eq!(transcript.segments[0].start, Duration::from_secs(10));
        assert_eq!(transcript.segments[0].end, Duration::from_secs(20));
    }
}
