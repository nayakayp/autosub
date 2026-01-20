use crate::error::{AutosubError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;



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
    pub gemini_api_key: Option<String>,
    pub default_format: OutputFormat,
    pub concurrency: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gemini_api_key: None,
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
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            config.gemini_api_key = Some(key);
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

    pub fn validate(&self) -> Result<()> {
        if self.gemini_api_key.is_none() {
            return Err(AutosubError::Config(
                "GEMINI_API_KEY not set. Get one at https://aistudio.google.com/apikey"
                    .to_string(),
            ));
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
        assert_eq!(config.default_format, OutputFormat::Srt);
        assert_eq!(config.concurrency, 4);
    }

    #[test]
    fn test_validate_missing_api_key() {
        let config = Config::default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_with_api_key() {
        let mut config = Config::default();
        config.gemini_api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok());
    }
}
