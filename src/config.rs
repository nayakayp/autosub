use crate::error::{AutosubError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Whisper,
    Gemini,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Whisper => write!(f, "whisper"),
            Provider::Gemini => write!(f, "gemini"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "whisper" => Ok(Provider::Whisper),
            "gemini" => Ok(Provider::Gemini),
            _ => Err(format!("Unknown provider: {}. Use 'whisper' or 'gemini'", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Srt,
    Vtt,
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Srt => write!(f, "srt"),
            OutputFormat::Vtt => write!(f, "vtt"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "srt" => Ok(OutputFormat::Srt),
            "vtt" => Ok(OutputFormat::Vtt),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!(
                "Unknown format: {}. Use 'srt', 'vtt', or 'json'",
                s
            )),
        }
    }
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Srt => "srt",
            OutputFormat::Vtt => "vtt",
            OutputFormat::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub default_provider: Provider,
    pub default_format: OutputFormat,
    pub concurrency: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            gemini_api_key: None,
            default_provider: Provider::default(),
            default_format: OutputFormat::default(),
            concurrency: 4,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Load from config file if it exists
        if let Some(config_path) = Self::config_file_path() {
            if config_path.exists() {
                let contents = std::fs::read_to_string(&config_path)?;
                if let Ok(file_config) = toml::from_str::<Config>(&contents) {
                    config = file_config;
                }
            }
        }

        // Override with environment variables
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.openai_api_key = Some(key);
        }
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            config.gemini_api_key = Some(key);
        }
        if let Ok(provider) = std::env::var("AUTOSUB_DEFAULT_PROVIDER") {
            if let Ok(p) = provider.parse() {
                config.default_provider = p;
            }
        }
        if let Ok(format) = std::env::var("AUTOSUB_DEFAULT_FORMAT") {
            if let Ok(f) = format.parse() {
                config.default_format = f;
            }
        }
        if let Ok(concurrency) = std::env::var("AUTOSUB_CONCURRENCY") {
            if let Ok(c) = concurrency.parse() {
                config.concurrency = c;
            }
        }

        Ok(config)
    }

    pub fn validate(&self, provider: Provider) -> Result<()> {
        match provider {
            Provider::Whisper => {
                if self.openai_api_key.is_none() {
                    return Err(AutosubError::Config(
                        "OPENAI_API_KEY not set. Export it with: export OPENAI_API_KEY=sk-..."
                            .to_string(),
                    ));
                }
            }
            Provider::Gemini => {
                if self.gemini_api_key.is_none() {
                    return Err(AutosubError::Config(
                        "GEMINI_API_KEY not set. Get one at https://aistudio.google.com/apikey"
                            .to_string(),
                    ));
                }
            }
        }

        if self.concurrency == 0 {
            return Err(AutosubError::Config(
                "Concurrency must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("autosub").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_parsing() {
        assert_eq!("whisper".parse::<Provider>().unwrap(), Provider::Whisper);
        assert_eq!("gemini".parse::<Provider>().unwrap(), Provider::Gemini);
        assert_eq!("WHISPER".parse::<Provider>().unwrap(), Provider::Whisper);
        assert!("unknown".parse::<Provider>().is_err());
    }

    #[test]
    fn test_format_parsing() {
        assert_eq!("srt".parse::<OutputFormat>().unwrap(), OutputFormat::Srt);
        assert_eq!("vtt".parse::<OutputFormat>().unwrap(), OutputFormat::Vtt);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert!("txt".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(OutputFormat::Srt.extension(), "srt");
        assert_eq!(OutputFormat::Vtt.extension(), "vtt");
        assert_eq!(OutputFormat::Json.extension(), "json");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.default_provider, Provider::Whisper);
        assert_eq!(config.default_format, OutputFormat::Srt);
        assert_eq!(config.concurrency, 4);
    }

    #[test]
    fn test_validate_missing_api_key() {
        let config = Config::default();
        assert!(config.validate(Provider::Whisper).is_err());
        assert!(config.validate(Provider::Gemini).is_err());
    }

    #[test]
    fn test_validate_with_api_key() {
        let mut config = Config::default();
        config.openai_api_key = Some("sk-test".to_string());
        assert!(config.validate(Provider::Whisper).is_ok());

        config.gemini_api_key = Some("test-key".to_string());
        assert!(config.validate(Provider::Gemini).is_ok());
    }
}
