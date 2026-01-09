use thiserror::Error;

#[derive(Error, Debug)]
pub enum AutosubError {
    #[error("Audio extraction failed: {0}")]
    AudioExtraction(String),

    #[error("Transcription failed: {0}")]
    Transcription(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AutosubError>;
