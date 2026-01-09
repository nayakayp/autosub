pub mod convert;
pub mod json;
pub mod postprocess;
pub mod srt;
pub mod vtt;

pub use convert::{convert_to_subtitles, convert_with_defaults, quick_convert};
pub use postprocess::{post_process, PostProcessConfig};

use crate::config::OutputFormat;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub index: usize,
    pub start: Duration,
    pub end: Duration,
    pub text: String,
    pub speaker: Option<String>,
}

pub trait SubtitleFormatter {
    fn format(&self, entries: &[SubtitleEntry]) -> String;
    fn extension(&self) -> &'static str;
}

pub fn create_formatter(format: OutputFormat) -> Box<dyn SubtitleFormatter> {
    match format {
        OutputFormat::Srt => Box::new(srt::SrtFormatter),
        OutputFormat::Vtt => Box::new(vtt::VttFormatter),
        OutputFormat::Json => Box::new(json::JsonFormatter::default()),
    }
}
