// JSON subtitle format
use super::{SubtitleEntry, SubtitleFormatter};
use serde::Serialize;

#[derive(Default)]
pub struct JsonFormatter {
    pub source_file: Option<String>,
    pub language: Option<String>,
    pub provider: Option<String>,
}

#[derive(Serialize)]
struct JsonOutput {
    metadata: JsonMetadata,
    subtitles: Vec<JsonSubtitle>,
}

#[derive(Serialize)]
struct JsonMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    subtitle_count: usize,
}

#[derive(Serialize)]
struct JsonSubtitle {
    index: usize,
    start: f64,
    end: f64,
    start_formatted: String,
    end_formatted: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker: Option<String>,
}

impl SubtitleFormatter for JsonFormatter {
    fn format(&self, entries: &[SubtitleEntry]) -> String {
        let output = JsonOutput {
            metadata: JsonMetadata {
                source_file: self.source_file.clone(),
                language: self.language.clone(),
                provider: self.provider.clone(),
                subtitle_count: entries.len(),
            },
            subtitles: entries
                .iter()
                .map(|e| JsonSubtitle {
                    index: e.index,
                    start: e.start.as_secs_f64(),
                    end: e.end.as_secs_f64(),
                    start_formatted: format_timestamp(e.start),
                    end_formatted: format_timestamp(e.end),
                    text: e.text.clone(),
                    speaker: e.speaker.clone(),
                })
                .collect(),
        };

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }

    fn extension(&self) -> &'static str {
        "json"
    }
}

fn format_timestamp(d: std::time::Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = d.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_json_format() {
        let entries = vec![SubtitleEntry {
            index: 1,
            start: Duration::from_millis(1500),
            end: Duration::from_millis(4000),
            text: "Hello, world!".to_string(),
            speaker: None,
        }];

        let formatter = JsonFormatter::default();
        let output = formatter.format(&entries);

        assert!(output.contains("\"subtitle_count\": 1"));
        assert!(output.contains("\"text\": \"Hello, world!\""));
        assert!(output.contains("\"start\": 1.5"));
    }
}
