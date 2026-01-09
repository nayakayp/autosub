use crate::audio::AudioChunk;
use crate::error::{AutosubError, Result};
use crate::transcribe::{Transcriber, Transcript, TranscriptSegment};
use async_trait::async_trait;
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, warn};

/// Gemini API endpoint for content generation.
const GENERATE_CONTENT_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent";

/// Gemini Files API endpoint for uploading large files.
const FILES_UPLOAD_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";

/// Threshold for using Files API vs inline data (20 MB).
const INLINE_SIZE_THRESHOLD: usize = 20 * 1024 * 1024;

/// Maximum file size we'll handle (much larger than Whisper).
const MAX_FILE_SIZE: usize = 200 * 1024 * 1024;

/// Maximum retries for API calls.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds).
const BASE_DELAY_MS: u64 = 1000;

/// Google Gemini Audio API client.
pub struct GeminiClient {
    client: reqwest::Client,
    api_key: String,
    language: Option<String>,
    enable_diarization: bool,
}

impl GeminiClient {
    /// Create a new Gemini client with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            language: None,
            enable_diarization: false,
        }
    }

    /// Set the source language for transcription.
    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    /// Enable speaker diarization.
    pub fn with_diarization(mut self, enable: bool) -> Self {
        self.enable_diarization = enable;
        self
    }

    /// Get MIME type for audio file.
    fn get_mime_type(path: &Path) -> &'static str {
        match path.extension().and_then(|e| e.to_str()) {
            Some("wav") => "audio/wav",
            Some("mp3") => "audio/mpeg",
            Some("m4a") => "audio/mp4",
            Some("flac") => "audio/flac",
            Some("ogg") => "audio/ogg",
            Some("aac") => "audio/aac",
            Some("aiff") => "audio/aiff",
            _ => "audio/wav",
        }
    }

    /// Build the transcription prompt.
    fn build_prompt(&self) -> String {
        let mut prompt = String::new();

        prompt.push_str("Transcribe this audio with precise timestamps.\n\n");
        prompt.push_str("Format each line as:\n");
        prompt.push_str("[MM:SS] Text of what was said\n\n");

        if let Some(ref lang) = self.language {
            prompt.push_str(&format!("The audio is in {} language.\n", lang));
        }

        if self.enable_diarization {
            prompt.push_str(
                "Identify different speakers and label them as Speaker 1, Speaker 2, etc.\n",
            );
            prompt.push_str("Format: [MM:SS] Speaker N: Text\n");
        }

        prompt.push_str("\nProvide accurate timestamps for each segment of speech.");

        prompt
    }

    /// Transcribe using inline audio data (for files < 20MB).
    async fn transcribe_inline(&self, chunk: &AudioChunk) -> Result<Transcript> {
        let audio_bytes = fs::read(&chunk.path).await?;
        let base64_audio = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
        let mime_type = Self::get_mime_type(&chunk.path);

        let request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text {
                        text: self.build_prompt(),
                    },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: base64_audio,
                        },
                    },
                ],
            }],
            generation_config: Some(GenerationConfig {
                temperature: Some(0.0),
                max_output_tokens: Some(8192),
            }),
        };

        self.call_generate_content(request, chunk).await
    }

    /// Upload a file using the Files API (for files >= 20MB).
    async fn upload_file(&self, path: &Path) -> Result<String> {
        let file_bytes = fs::read(path).await?;
        let mime_type = Self::get_mime_type(path);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");

        let url = format!("{}?key={}", FILES_UPLOAD_URL, self.api_key);

        // Upload with resumable upload protocol
        let response = self
            .client
            .post(&url)
            .header("X-Goog-Upload-Protocol", "raw")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .header("Content-Type", mime_type)
            .header("X-Goog-Upload-File-Name", file_name)
            .body(file_bytes)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AutosubError::Api(format!(
                "Gemini file upload failed: {}",
                error_text
            )));
        }

        let upload_response: FileUploadResponse = response.json().await?;
        Ok(upload_response.file.uri)
    }

    /// Transcribe using uploaded file reference.
    async fn transcribe_file(&self, file_uri: &str, chunk: &AudioChunk) -> Result<Transcript> {
        let request = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text {
                        text: self.build_prompt(),
                    },
                    Part::FileData {
                        file_data: FileData {
                            mime_type: "audio/wav".to_string(),
                            file_uri: file_uri.to_string(),
                        },
                    },
                ],
            }],
            generation_config: Some(GenerationConfig {
                temperature: Some(0.0),
                max_output_tokens: Some(8192),
            }),
        };

        self.call_generate_content(request, chunk).await
    }

    /// Call the generateContent API endpoint.
    async fn call_generate_content(
        &self,
        request: GenerateContentRequest,
        chunk: &AudioChunk,
    ) -> Result<Transcript> {
        let url = format!("{}?key={}", GENERATE_CONTENT_URL, self.api_key);

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = BASE_DELAY_MS * 2u64.pow(attempt - 1);
                debug!("Retry attempt {} after {}ms delay", attempt, delay);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    debug!("Gemini API response status: {}", status);

                    if status.is_success() {
                        let body = resp.text().await?;
                        debug!("Gemini API response: {}", &body[..body.len().min(500)]);
                        let parsed: GenerateContentResponse = serde_json::from_str(&body)?;
                        return Ok(self.parse_response(parsed, chunk));
                    }

                    let error_body = resp.text().await.unwrap_or_default();

                    // Don't retry on client errors
                    if status.as_u16() >= 400 && status.as_u16() < 500 {
                        return Err(AutosubError::Api(format!(
                            "Gemini API error ({}): {}",
                            status, error_body
                        )));
                    }

                    warn!("Gemini API server error ({}): {}", status, error_body);
                    last_error = Some(AutosubError::Api(format!(
                        "Gemini API server error: {}",
                        status
                    )));
                }
                Err(e) => {
                    warn!("Gemini API request failed: {}", e);
                    last_error = Some(e.into());
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AutosubError::Api("Unknown error".to_string())))
    }

    /// Parse the Gemini response and extract transcript segments.
    fn parse_response(&self, response: GenerateContentResponse, chunk: &AudioChunk) -> Transcript {
        let text = response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| match p {
                ResponsePart::Text { text } => text.as_str(),
            })
            .unwrap_or("");

        debug!("Gemini raw response text: {}", text);

        let segments = self.parse_timestamped_text(text, chunk);

        Transcript {
            segments,
            language: self.language.clone(),
            duration: Some(chunk.duration()),
        }
    }

    /// Parse timestamped text like "[00:15] Hello world" into segments.
    fn parse_timestamped_text(&self, text: &str, chunk: &AudioChunk) -> Vec<TranscriptSegment> {
        let mut segments: Vec<TranscriptSegment> = Vec::new();

        // Regex to match [MM:SS] or [HH:MM:SS] timestamps at the start of lines or after newlines
        let timestamp_re =
            Regex::new(r"\[(\d{1,2}):(\d{2})(?::(\d{2}))?\]\s*([^\[]+)").expect("Invalid regex");

        for cap in timestamp_re.captures_iter(text) {
            let minutes: u64 = cap.get(1).unwrap().as_str().parse().unwrap_or(0);
            let seconds: u64 = cap.get(2).unwrap().as_str().parse().unwrap_or(0);

            // Handle optional hours field
            let (hours, mins, secs) = if let Some(s) = cap.get(3) {
                // Format was HH:MM:SS
                let hour_secs: u64 = s.as_str().parse().unwrap_or(0);
                (minutes, seconds, hour_secs)
            } else {
                // Format was MM:SS
                (0, minutes, seconds)
            };

            let timestamp_secs = hours * 3600 + mins * 60 + secs;
            let start = chunk.region.start + Duration::from_secs(timestamp_secs);

            let raw_text = cap.get(4).map(|m| m.as_str().trim()).unwrap_or("");

            // Parse speaker label if present (e.g., "Speaker 1: Hello")
            let (speaker, clean_text) = if raw_text.contains(':') {
                let parts: Vec<&str> = raw_text.splitn(2, ':').collect();
                if parts.len() == 2 && parts[0].to_lowercase().contains("speaker") {
                    (
                        Some(parts[0].trim().to_string()),
                        parts[1].trim().to_string(),
                    )
                } else {
                    (None, raw_text.to_string())
                }
            } else {
                (None, raw_text.to_string())
            };

            if !clean_text.is_empty() {
                // Update end time of previous segment
                if let Some(prev) = segments.last_mut() {
                    prev.end = start;
                }

                segments.push(TranscriptSegment {
                    text: clean_text,
                    start,
                    end: chunk.region.end, // Will be updated by next segment or left as chunk end
                    words: None,
                    confidence: None,
                    speaker,
                });
            }
        }

        // If no timestamps found, create a single segment with all text
        if segments.is_empty() && !text.trim().is_empty() {
            // Clean up the text (remove any bracketed content)
            let clean_text = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.trim())
                .collect::<Vec<_>>()
                .join(" ");

            segments.push(TranscriptSegment {
                text: clean_text,
                start: chunk.region.start,
                end: chunk.region.end,
                words: None,
                confidence: None,
                speaker: None,
            });
        }

        segments
    }
}

#[async_trait]
impl Transcriber for GeminiClient {
    async fn transcribe(&self, chunk: &AudioChunk) -> Result<Transcript> {
        debug!(
            "Transcribing chunk {} with Gemini: {:?}",
            chunk.index, chunk.path
        );

        let metadata = fs::metadata(&chunk.path).await?;
        let file_size = metadata.len() as usize;

        if file_size > MAX_FILE_SIZE {
            return Err(AutosubError::Transcription(format!(
                "File too large: {} bytes (max {} bytes)",
                file_size, MAX_FILE_SIZE
            )));
        }

        let transcript = if file_size < INLINE_SIZE_THRESHOLD {
            debug!("Using inline audio data ({} bytes)", file_size);
            self.transcribe_inline(chunk).await?
        } else {
            debug!("Uploading file to Files API ({} bytes)", file_size);
            let file_uri = self.upload_file(&chunk.path).await?;
            debug!("File uploaded: {}", file_uri);
            let result = self.transcribe_file(&file_uri, chunk).await?;
            // Note: In production, we should delete the uploaded file after use
            result
        };

        debug!(
            "Gemini returned {} segments for chunk {}",
            transcript.segments.len(),
            chunk.index
        );

        Ok(transcript)
    }

    fn name(&self) -> &'static str {
        "Google Gemini"
    }

    fn max_file_size(&self) -> usize {
        MAX_FILE_SIZE
    }

    fn supported_formats(&self) -> &[&str] {
        &["wav", "mp3", "aiff", "aac", "ogg", "flac"]
    }
}

// Request/Response types

#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
    FileData { file_data: FileData },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct FileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ResponsePart {
    Text { text: String },
}

#[derive(Deserialize)]
struct FileUploadResponse {
    file: UploadedFile,
}

#[derive(Deserialize)]
struct UploadedFile {
    uri: String,
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
                end: Duration::from_secs(30),
            },
            path: PathBuf::from("/tmp/test.wav"),
            index: 0,
        }
    }

    #[test]
    fn test_parse_timestamped_text() {
        let client = GeminiClient::new("test-key".to_string());
        let chunk = create_test_chunk();

        let text = "[00:00] Hello world.\n[00:05] How are you doing today?";
        let segments = client.parse_timestamped_text(text, &chunk);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello world.");
        assert_eq!(segments[0].start, Duration::from_secs(10)); // chunk start + 0
        assert_eq!(segments[1].text, "How are you doing today?");
        assert_eq!(segments[1].start, Duration::from_secs(15)); // chunk start + 5
    }

    #[test]
    fn test_parse_with_speaker_labels() {
        let client = GeminiClient::new("test-key".to_string());
        let chunk = create_test_chunk();

        let text = "[00:00] Speaker 1: Hello.\n[00:03] Speaker 2: Hi there!";
        let segments = client.parse_timestamped_text(text, &chunk);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].speaker, Some("Speaker 1".to_string()));
        assert_eq!(segments[0].text, "Hello.");
        assert_eq!(segments[1].speaker, Some("Speaker 2".to_string()));
        assert_eq!(segments[1].text, "Hi there!");
    }

    #[test]
    fn test_parse_no_timestamps() {
        let client = GeminiClient::new("test-key".to_string());
        let chunk = create_test_chunk();

        let text = "This is just plain text without any timestamps.";
        let segments = client.parse_timestamped_text(text, &chunk);

        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0].text,
            "This is just plain text without any timestamps."
        );
        assert_eq!(segments[0].start, chunk.region.start);
    }

    #[test]
    fn test_build_prompt_basic() {
        let client = GeminiClient::new("test-key".to_string());
        let prompt = client.build_prompt();
        assert!(prompt.contains("Transcribe this audio"));
        assert!(prompt.contains("[MM:SS]"));
    }

    #[test]
    fn test_build_prompt_with_diarization() {
        let client = GeminiClient::new("test-key".to_string()).with_diarization(true);
        let prompt = client.build_prompt();
        assert!(prompt.contains("Speaker 1"));
        assert!(prompt.contains("Speaker 2"));
    }

    #[test]
    fn test_get_mime_type() {
        assert_eq!(
            GeminiClient::get_mime_type(Path::new("test.wav")),
            "audio/wav"
        );
        assert_eq!(
            GeminiClient::get_mime_type(Path::new("test.mp3")),
            "audio/mpeg"
        );
        assert_eq!(
            GeminiClient::get_mime_type(Path::new("test.flac")),
            "audio/flac"
        );
    }
}
