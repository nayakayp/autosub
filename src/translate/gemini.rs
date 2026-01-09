//! Gemini-based translation using the Generative AI API.

use crate::error::{AutosubError, Result};
use crate::translate::Translator;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Translator using Google Gemini API.
pub struct GeminiTranslator {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiTranslator {
    /// Create a new Gemini translator with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "gemini-2.0-flash".to_string(),
        }
    }

    /// Set a different model (e.g., "gemini-1.5-pro").
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Build the translation prompt.
    fn build_prompt(&self, texts: &[&str], target_lang: &str) -> String {
        let lang_name = language_code_to_name(target_lang);

        if texts.len() == 1 {
            format!(
                r#"Translate the following text to {lang_name}. 
Return ONLY the translated text, nothing else. Preserve all formatting and line breaks.

Text to translate:
{}"#,
                texts[0]
            )
        } else {
            let numbered_texts: String = texts
                .iter()
                .enumerate()
                .map(|(i, t)| format!("[{}] {}", i + 1, t))
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"Translate each of the following numbered texts to {lang_name}.
Return ONLY the translations in the same numbered format. Preserve all formatting.

Texts to translate:
{numbered_texts}"#
            )
        }
    }

    /// Parse batch translation response.
    fn parse_batch_response(&self, response: &str, count: usize) -> Vec<String> {
        let mut results = Vec::with_capacity(count);

        // Try to parse numbered responses
        for i in 1..=count {
            let pattern = format!("[{}]", i);
            let next_pattern = format!("[{}]", i + 1);

            if let Some(start) = response.find(&pattern) {
                let text_start = start + pattern.len();
                let text_end = if i < count {
                    response[text_start..]
                        .find(&next_pattern)
                        .map(|p| text_start + p)
                        .unwrap_or(response.len())
                } else {
                    response.len()
                };

                let translated = response[text_start..text_end].trim().to_string();
                results.push(translated);
            }
        }

        // If parsing failed, split by newlines as fallback
        if results.len() != count {
            warn!(
                "Batch parse failed (got {} of {}), using line-based fallback",
                results.len(),
                count
            );
            results = response
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(count)
                .map(|l| l.trim().to_string())
                .collect();
        }

        // Pad with empty strings if still not enough
        while results.len() < count {
            results.push(String::new());
        }

        results
    }
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Deserialize, Debug)]
struct GeminiCandidate {
    content: Option<GeminiResponseContent>,
}

#[derive(Deserialize, Debug)]
struct GeminiResponseContent {
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize, Debug)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GeminiError {
    message: String,
}

#[async_trait]
impl Translator for GeminiTranslator {
    async fn translate(&self, text: &str, target_lang: &str) -> Result<String> {
        let texts = &[text];
        let results = self.translate_batch(texts, target_lang).await?;
        Ok(results.into_iter().next().unwrap_or_default())
    }

    async fn translate_batch(&self, texts: &[&str], target_lang: &str) -> Result<Vec<String>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        debug!("Translating {} text(s) to {}", texts.len(), target_lang);

        let prompt = self.build_prompt(texts, target_lang);

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AutosubError::Api(format!("Translation request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AutosubError::Api(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(AutosubError::Api(format!(
                "Translation API error ({}): {}",
                status, body
            )));
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&body).map_err(|e| {
            AutosubError::Api(format!("Failed to parse translation response: {}", e))
        })?;

        if let Some(error) = gemini_response.error {
            return Err(AutosubError::Api(format!(
                "Gemini error: {}",
                error.message
            )));
        }

        let translated_text = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .unwrap_or_default();

        if texts.len() == 1 {
            Ok(vec![translated_text.trim().to_string()])
        } else {
            Ok(self.parse_batch_response(&translated_text, texts.len()))
        }
    }

    fn supported_languages(&self) -> &[&str] {
        &SUPPORTED_LANGUAGES
    }

    fn name(&self) -> &'static str {
        "gemini"
    }
}

/// Convert language code to human-readable name for better prompting.
fn language_code_to_name(code: &str) -> &'static str {
    let lowercase = code.to_lowercase();
    match lowercase.as_str() {
        "en" => "English",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "pt" => "Portuguese",
        "ru" => "Russian",
        "ja" => "Japanese",
        "ko" => "Korean",
        "zh" => "Chinese",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "th" => "Thai",
        "vi" => "Vietnamese",
        "id" => "Indonesian",
        "ms" => "Malay",
        "tl" => "Tagalog",
        "nl" => "Dutch",
        "pl" => "Polish",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "cs" => "Czech",
        "sv" => "Swedish",
        "da" => "Danish",
        "fi" => "Finnish",
        "no" => "Norwegian",
        "el" => "Greek",
        "he" => "Hebrew",
        "hu" => "Hungarian",
        "ro" => "Romanian",
        "bg" => "Bulgarian",
        "hr" => "Croatian",
        "sk" => "Slovak",
        "sl" => "Slovenian",
        "lt" => "Lithuanian",
        "lv" => "Latvian",
        "et" => "Estonian",
        // For unknown codes, return a static fallback
        _ => "the target language",
    }
}

/// List of commonly supported language codes.
const SUPPORTED_LANGUAGES: [&str; 38] = [
    "en", "es", "fr", "de", "it", "pt", "ru", "ja", "ko", "zh", "ar", "hi", "th", "vi", "id", "ms",
    "tl", "nl", "pl", "tr", "uk", "cs", "sv", "da", "fi", "no", "el", "he", "hu", "ro", "bg", "hr",
    "sk", "sl", "lt", "lv", "et", "bn",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_translator_creation() {
        let translator = GeminiTranslator::new("test-key".to_string());
        assert_eq!(translator.name(), "gemini");
        assert_eq!(translator.model, "gemini-2.0-flash");
    }

    #[test]
    fn test_with_model() {
        let translator = GeminiTranslator::new("test-key".to_string()).with_model("gemini-1.5-pro");
        assert_eq!(translator.model, "gemini-1.5-pro");
    }

    #[test]
    fn test_supported_languages() {
        let translator = GeminiTranslator::new("test-key".to_string());
        let languages = translator.supported_languages();
        assert!(languages.contains(&"en"));
        assert!(languages.contains(&"ja"));
        assert!(languages.contains(&"es"));
    }

    #[test]
    fn test_build_prompt_single() {
        let translator = GeminiTranslator::new("test-key".to_string());
        let prompt = translator.build_prompt(&["Hello, world!"], "es");
        assert!(prompt.contains("Spanish"));
        assert!(prompt.contains("Hello, world!"));
    }

    #[test]
    fn test_build_prompt_batch() {
        let translator = GeminiTranslator::new("test-key".to_string());
        let prompt = translator.build_prompt(&["Hello", "Goodbye"], "ja");
        assert!(prompt.contains("Japanese"));
        assert!(prompt.contains("[1] Hello"));
        assert!(prompt.contains("[2] Goodbye"));
    }

    #[test]
    fn test_parse_batch_response() {
        let translator = GeminiTranslator::new("test-key".to_string());
        let response = "[1] Hola\n[2] Adiós";
        let results = translator.parse_batch_response(response, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "Hola");
        assert_eq!(results[1], "Adiós");
    }

    #[test]
    fn test_language_code_to_name() {
        assert_eq!(language_code_to_name("en"), "English");
        assert_eq!(language_code_to_name("ja"), "Japanese");
        assert_eq!(language_code_to_name("ES"), "Spanish"); // case insensitive
        assert_eq!(language_code_to_name("xyz"), "the target language"); // unknown returns fallback
    }
}
