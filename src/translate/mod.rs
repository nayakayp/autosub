use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Translator: Send + Sync {
    async fn translate(&self, text: &str, target_lang: &str) -> Result<String>;
    async fn translate_batch(&self, texts: &[&str], target_lang: &str) -> Result<Vec<String>>;
    fn supported_languages(&self) -> &[&str];
}
