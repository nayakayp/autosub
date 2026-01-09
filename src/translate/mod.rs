pub mod gemini;

use crate::error::Result;
use async_trait::async_trait;

pub use gemini::GeminiTranslator;

/// Trait for translation providers.
#[async_trait]
pub trait Translator: Send + Sync {
    /// Translate a single text to the target language.
    async fn translate(&self, text: &str, target_lang: &str) -> Result<String>;

    /// Translate multiple texts to the target language.
    /// More efficient than calling translate() multiple times.
    async fn translate_batch(&self, texts: &[&str], target_lang: &str) -> Result<Vec<String>>;

    /// Get list of supported language codes.
    fn supported_languages(&self) -> &[&str];

    /// Get the name of the translator.
    fn name(&self) -> &'static str;
}

/// Create a translator using the available API key.
pub fn create_translator(gemini_api_key: Option<&str>) -> Result<Box<dyn Translator>> {
    if let Some(key) = gemini_api_key {
        return Ok(Box::new(GeminiTranslator::new(key.to_string())));
    }

    Err(crate::error::AutosubError::Config(
        "No API key available for translation. Set GEMINI_API_KEY.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_translator_with_gemini_key() {
        let translator = create_translator(Some("test-key"));
        assert!(translator.is_ok());
        assert_eq!(translator.unwrap().name(), "gemini");
    }

    #[test]
    fn test_create_translator_no_key() {
        let translator = create_translator(None);
        assert!(translator.is_err());
    }
}
