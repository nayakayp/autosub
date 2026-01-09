// SRT subtitle format
use super::{SubtitleEntry, SubtitleFormatter};

pub struct SrtFormatter;

impl SubtitleFormatter for SrtFormatter {
    fn format(&self, entries: &[SubtitleEntry]) -> String {
        entries
            .iter()
            .map(|entry| {
                format!(
                    "{}\n{} --> {}\n{}\n",
                    entry.index,
                    format_timestamp(entry.start),
                    format_timestamp(entry.end),
                    entry.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn extension(&self) -> &'static str {
        "srt"
    }
}

fn format_timestamp(d: std::time::Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = d.subsec_millis();
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(
            format_timestamp(Duration::from_millis(1500)),
            "00:00:01,500"
        );
        assert_eq!(
            format_timestamp(Duration::from_secs(3661) + Duration::from_millis(123)),
            "01:01:01,123"
        );
    }

    #[test]
    fn test_srt_format() {
        let entries = vec![
            SubtitleEntry {
                index: 1,
                start: Duration::from_millis(1500),
                end: Duration::from_millis(4000),
                text: "Hello, world!".to_string(),
                speaker: None,
            },
            SubtitleEntry {
                index: 2,
                start: Duration::from_millis(4500),
                end: Duration::from_millis(7000),
                text: "This is a test.".to_string(),
                speaker: None,
            },
        ];

        let formatter = SrtFormatter;
        let output = formatter.format(&entries);

        assert!(output.contains("1\n00:00:01,500 --> 00:00:04,000\nHello, world!"));
        assert!(output.contains("2\n00:00:04,500 --> 00:00:07,000\nThis is a test."));
    }
}
