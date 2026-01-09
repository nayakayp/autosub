use super::postprocess::PostProcessConfig;
use super::SubtitleEntry;
use crate::transcribe::TranscriptSegment;

/// Convert transcript segments to subtitle entries with optional post-processing.
pub fn convert_to_subtitles(
    segments: Vec<TranscriptSegment>,
    config: Option<PostProcessConfig>,
) -> Vec<SubtitleEntry> {
    // Convert segments to entries
    let entries: Vec<SubtitleEntry> = segments
        .into_iter()
        .enumerate()
        .map(|(i, segment)| {
            let text = format_text_with_speaker(&segment.text, segment.speaker.as_deref());

            SubtitleEntry {
                index: i + 1,
                start: segment.start,
                end: segment.end,
                text,
                speaker: segment.speaker,
            }
        })
        .collect();

    // Validate no overlapping timestamps
    let entries = fix_overlapping_timestamps(entries);

    // Apply post-processing if config provided
    if let Some(config) = config {
        super::postprocess::post_process(entries, &config)
    } else {
        entries
    }
}

/// Format text with optional speaker label prefix.
fn format_text_with_speaker(text: &str, speaker: Option<&str>) -> String {
    match speaker {
        Some(s) if !s.is_empty() => format!("[{}] {}", s, text.trim()),
        _ => text.trim().to_string(),
    }
}

/// Fix overlapping timestamps by adjusting end times.
fn fix_overlapping_timestamps(entries: Vec<SubtitleEntry>) -> Vec<SubtitleEntry> {
    if entries.is_empty() {
        return entries;
    }

    let mut result: Vec<SubtitleEntry> = Vec::new();

    for entry in entries {
        if let Some(last) = result.last_mut() {
            // If current entry starts before previous ends, adjust previous end
            if entry.start < last.end {
                last.end = entry.start;
            }
        }
        result.push(entry);
    }

    result
}

/// Quick conversion without post-processing.
pub fn quick_convert(segments: Vec<TranscriptSegment>) -> Vec<SubtitleEntry> {
    convert_to_subtitles(segments, None)
}

/// Conversion with default post-processing.
pub fn convert_with_defaults(segments: Vec<TranscriptSegment>) -> Vec<SubtitleEntry> {
    convert_to_subtitles(segments, Some(PostProcessConfig::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn segment(start_ms: u64, end_ms: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            text: text.to_string(),
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            words: None,
            confidence: None,
            speaker: None,
        }
    }

    fn segment_with_speaker(
        start_ms: u64,
        end_ms: u64,
        text: &str,
        speaker: &str,
    ) -> TranscriptSegment {
        TranscriptSegment {
            text: text.to_string(),
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            words: None,
            confidence: None,
            speaker: Some(speaker.to_string()),
        }
    }

    #[test]
    fn test_quick_convert() {
        let segments = vec![
            segment(0, 2000, "Hello world"),
            segment(2500, 5000, "This is a test"),
        ];

        let entries = quick_convert(segments);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[0].text, "Hello world");
        assert_eq!(entries[1].index, 2);
        assert_eq!(entries[1].text, "This is a test");
    }

    #[test]
    fn test_convert_with_speaker() {
        let segments = vec![
            segment_with_speaker(0, 2000, "Hello", "Alice"),
            segment_with_speaker(2500, 5000, "Hi there", "Bob"),
        ];

        let entries = quick_convert(segments);

        assert_eq!(entries[0].text, "[Alice] Hello");
        assert_eq!(entries[1].text, "[Bob] Hi there");
    }

    #[test]
    fn test_fix_overlapping_timestamps() {
        let entries = vec![
            SubtitleEntry {
                index: 1,
                start: Duration::from_millis(0),
                end: Duration::from_millis(3000), // Overlaps with next
                text: "First".to_string(),
                speaker: None,
            },
            SubtitleEntry {
                index: 2,
                start: Duration::from_millis(2500), // Starts before previous ends
                end: Duration::from_millis(5000),
                text: "Second".to_string(),
                speaker: None,
            },
        ];

        let result = fix_overlapping_timestamps(entries);

        // First entry's end should be adjusted to second's start
        assert_eq!(result[0].end, Duration::from_millis(2500));
        assert_eq!(result[1].start, Duration::from_millis(2500));
    }

    #[test]
    fn test_convert_empty() {
        let segments: Vec<TranscriptSegment> = vec![];
        let entries = quick_convert(segments);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_convert_trims_whitespace() {
        let segments = vec![segment(0, 2000, "  Hello world  ")];
        let entries = quick_convert(segments);
        assert_eq!(entries[0].text, "Hello world");
    }

    #[test]
    fn test_convert_with_defaults() {
        let segments = vec![
            segment(0, 1000, "Hello"),
            segment(1200, 2000, "world"), // Close enough to merge
        ];

        let entries = convert_with_defaults(segments);

        // Should be merged
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Hello world");
    }
}
