// WebVTT subtitle format
use super::{SubtitleEntry, SubtitleFormatter};

pub struct VttFormatter;

impl SubtitleFormatter for VttFormatter {
    fn format(&self, entries: &[SubtitleEntry]) -> String {
        let mut output = String::from("WEBVTT\n\n");

        for entry in entries {
            output.push_str(&format!(
                "{} --> {}\n{}\n\n",
                format_timestamp(entry.start),
                format_timestamp(entry.end),
                entry.text
            ));
        }

        output
    }

    fn extension(&self) -> &'static str {
        "vtt"
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
    fn test_format_timestamp() {
        assert_eq!(
            format_timestamp(Duration::from_millis(1500)),
            "00:00:01.500"
        );
    }

    #[test]
    fn test_vtt_format() {
        let entries = vec![SubtitleEntry {
            index: 1,
            start: Duration::from_millis(1500),
            end: Duration::from_millis(4000),
            text: "Hello, world!".to_string(),
            speaker: None,
        }];

        let formatter = VttFormatter;
        let output = formatter.format(&entries);

        assert!(output.starts_with("WEBVTT\n\n"));
        assert!(output.contains("00:00:01.500 --> 00:00:04.000"));
    }
}
