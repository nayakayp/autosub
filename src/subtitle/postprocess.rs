use super::SubtitleEntry;
use std::time::Duration;

/// Configuration for post-processing subtitles.
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    /// Merge segments closer than this duration (default: 1 second).
    pub merge_threshold: Duration,
    /// Maximum characters per line (default: 42).
    pub max_line_length: usize,
    /// Minimum gap between subtitles (default: 100ms).
    pub min_gap: Duration,
    /// Minimum subtitle duration (default: 1 second).
    pub min_duration: Duration,
    /// Maximum subtitle duration (default: 7 seconds).
    pub max_duration: Duration,
    /// Remove filler words like "um", "uh", etc.
    pub remove_fillers: bool,
    /// Add punctuation if missing.
    pub add_punctuation: bool,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            merge_threshold: Duration::from_secs(1),
            max_line_length: 42,
            min_gap: Duration::from_millis(100),
            min_duration: Duration::from_secs(1),
            max_duration: Duration::from_secs(7),
            remove_fillers: false,
            add_punctuation: false,
        }
    }
}

/// Post-process subtitle entries to improve readability and timing.
pub fn post_process(entries: Vec<SubtitleEntry>, config: &PostProcessConfig) -> Vec<SubtitleEntry> {
    let mut result = entries;

    // Step 1: Remove filler words if enabled
    if config.remove_fillers {
        result = remove_filler_words(result);
    }

    // Step 2: Merge segments that are close together
    result = merge_close_segments(result, config.merge_threshold);

    // Step 3: Split long lines
    result = split_long_lines(result, config.max_line_length);

    // Step 4: Adjust timing (min gap, min/max duration)
    result = adjust_timing(result, config);

    // Step 5: Re-number entries sequentially
    result = renumber_entries(result);

    result
}

/// Merge segments that are closer than the threshold.
fn merge_close_segments(entries: Vec<SubtitleEntry>, threshold: Duration) -> Vec<SubtitleEntry> {
    if entries.is_empty() {
        return entries;
    }

    let mut result: Vec<SubtitleEntry> = Vec::new();

    for entry in entries {
        if let Some(last) = result.last_mut() {
            // Check if same speaker and close enough to merge
            let same_speaker = last.speaker == entry.speaker;
            let gap = entry.start.saturating_sub(last.end);

            if same_speaker && gap < threshold {
                // Merge: extend the last entry
                last.end = entry.end;
                last.text = format!("{} {}", last.text.trim(), entry.text.trim());
            } else {
                result.push(entry);
            }
        } else {
            result.push(entry);
        }
    }

    result
}

/// Split text that exceeds max line length at sentence boundaries when possible.
fn split_long_lines(entries: Vec<SubtitleEntry>, max_length: usize) -> Vec<SubtitleEntry> {
    let mut result = Vec::new();

    for entry in entries {
        if entry.text.len() <= max_length {
            result.push(entry);
            continue;
        }

        // Try to split at sentence boundaries or commas
        let split_text = smart_split(&entry.text, max_length);

        if split_text.len() == 1 {
            // Couldn't split meaningfully, keep original
            result.push(entry);
        } else {
            // Distribute time proportionally across splits
            let total_duration = entry.end.saturating_sub(entry.start);
            let total_chars: usize = split_text.iter().map(|s| s.len()).sum();
            let num_splits = split_text.len();
            let mut current_start = entry.start;

            for (i, text) in split_text.into_iter().enumerate() {
                let proportion = text.len() as f64 / total_chars as f64;
                let segment_duration =
                    Duration::from_secs_f64(total_duration.as_secs_f64() * proportion);
                let segment_end = if i == num_splits - 1 {
                    entry.end // Last segment gets exact end time
                } else {
                    current_start + segment_duration
                };

                result.push(SubtitleEntry {
                    index: 0, // Will be renumbered later
                    start: current_start,
                    end: segment_end,
                    text,
                    speaker: entry.speaker.clone(),
                });

                current_start = segment_end;
            }
        }
    }

    result
}

/// Smart split text at sentence boundaries, commas, or word boundaries.
fn smart_split(text: &str, max_length: usize) -> Vec<String> {
    if text.len() <= max_length {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() {
        // Count characters, not bytes, for proper UTF-8 handling
        let char_count = remaining.chars().count();

        if char_count <= max_length {
            result.push(remaining.trim().to_string());
            break;
        }

        // Find byte index for max_length characters
        let byte_limit = remaining
            .char_indices()
            .nth(max_length)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());

        // Find best split point within max_length characters
        let search_range = &remaining[..byte_limit];

        // Priority: sentence end (. ! ?) > comma > space
        let split_pos = find_best_split(search_range);

        if let Some(pos) = split_pos {
            // Find the byte position after the split character
            let next_char_start = remaining[pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| pos + i)
                .unwrap_or(remaining.len());
            result.push(remaining[..=pos].trim().to_string());
            remaining = remaining[next_char_start..].to_string();
        } else {
            // No good split point, force split at max_length characters
            result.push(remaining[..byte_limit].trim().to_string());
            remaining = remaining[byte_limit..].to_string();
        }
    }

    result.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Find the best position to split text.
fn find_best_split(text: &str) -> Option<usize> {
    // First try sentence endings
    let sentence_ends: Vec<usize> = text
        .char_indices()
        .filter(|(_, c)| *c == '.' || *c == '!' || *c == '?')
        .map(|(i, _)| i)
        .collect();

    if let Some(&pos) = sentence_ends.last() {
        return Some(pos);
    }

    // Then try commas
    let commas: Vec<usize> = text
        .char_indices()
        .filter(|(_, c)| *c == ',')
        .map(|(i, _)| i)
        .collect();

    if let Some(&pos) = commas.last() {
        return Some(pos);
    }

    // Finally, try last space
    text.rfind(' ')
}

/// Adjust timing to ensure minimum gaps and durations.
fn adjust_timing(entries: Vec<SubtitleEntry>, config: &PostProcessConfig) -> Vec<SubtitleEntry> {
    if entries.is_empty() {
        return entries;
    }

    let mut result: Vec<SubtitleEntry> = Vec::new();

    for mut entry in entries {
        // Ensure minimum duration
        let duration = entry.end.saturating_sub(entry.start);
        if duration < config.min_duration {
            entry.end = entry.start + config.min_duration;
        }

        // Ensure maximum duration (truncate if too long)
        let duration = entry.end.saturating_sub(entry.start);
        if duration > config.max_duration {
            entry.end = entry.start + config.max_duration;
        }

        // Ensure minimum gap from previous entry
        if let Some(last) = result.last_mut() {
            let gap = entry.start.saturating_sub(last.end);
            if gap < config.min_gap {
                // Adjust the previous entry's end time to create the gap
                let overlap = config.min_gap - gap;
                last.end = last.end.saturating_sub(overlap);

                // If that makes the previous entry too short, shift the current entry instead
                let prev_duration = last.end.saturating_sub(last.start);
                if prev_duration < config.min_duration {
                    last.end = last.start + config.min_duration;
                    entry.start = last.end + config.min_gap;
                }
            }
        }

        // Ensure no overlap with previous
        if let Some(last) = result.last() {
            if entry.start < last.end {
                entry.start = last.end + config.min_gap;
            }
        }

        result.push(entry);
    }

    result
}

/// Remove common filler words from subtitle text.
fn remove_filler_words(entries: Vec<SubtitleEntry>) -> Vec<SubtitleEntry> {
    const FILLERS: &[&str] = &[
        " um ",
        " uh ",
        " um,",
        " uh,",
        " um.",
        " uh.",
        " er ",
        " er,",
        " er.",
        " like ",
        " like, ",
        " you know ",
        " you know, ",
        " I mean ",
        " I mean, ",
    ];

    entries
        .into_iter()
        .map(|mut entry| {
            let mut text = format!(" {} ", entry.text);
            for filler in FILLERS {
                text = text.replace(filler, " ");
            }
            entry.text = text.trim().to_string();

            // Clean up multiple spaces
            while entry.text.contains("  ") {
                entry.text = entry.text.replace("  ", " ");
            }

            entry
        })
        .filter(|e| !e.text.is_empty())
        .collect()
}

/// Re-number entries sequentially starting from 1.
fn renumber_entries(entries: Vec<SubtitleEntry>) -> Vec<SubtitleEntry> {
    entries
        .into_iter()
        .enumerate()
        .map(|(i, mut entry)| {
            entry.index = i + 1;
            entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize, start_ms: u64, end_ms: u64, text: &str) -> SubtitleEntry {
        SubtitleEntry {
            index,
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            text: text.to_string(),
            speaker: None,
        }
    }

    #[test]
    fn test_merge_close_segments() {
        let entries = vec![
            entry(1, 0, 1000, "Hello"),
            entry(2, 1200, 2000, "world"),
            entry(3, 5000, 6000, "Separate"),
        ];

        let result = merge_close_segments(entries, Duration::from_secs(1));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Hello world");
        assert_eq!(result[1].text, "Separate");
    }

    #[test]
    fn test_merge_respects_speakers() {
        let mut entries = vec![entry(1, 0, 1000, "Hello"), entry(2, 1200, 2000, "world")];
        entries[0].speaker = Some("A".to_string());
        entries[1].speaker = Some("B".to_string());

        let result = merge_close_segments(entries, Duration::from_secs(1));

        // Should not merge because different speakers
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_smart_split_short_text() {
        let result = smart_split("Hello world", 42);
        assert_eq!(result, vec!["Hello world"]);
    }

    #[test]
    fn test_smart_split_at_sentence() {
        let text = "This is a sentence. This is another sentence.";
        let result = smart_split(text, 30);

        assert_eq!(result.len(), 2);
        assert!(result[0].ends_with('.'));
    }

    #[test]
    fn test_remove_filler_words() {
        let entries = vec![entry(1, 0, 1000, "So um I was like thinking")];

        let result = remove_filler_words(entries);

        assert_eq!(result[0].text, "So I was thinking");
    }

    #[test]
    fn test_renumber_entries() {
        let entries = vec![
            entry(5, 0, 1000, "First"),
            entry(10, 2000, 3000, "Second"),
            entry(15, 4000, 5000, "Third"),
        ];

        let result = renumber_entries(entries);

        assert_eq!(result[0].index, 1);
        assert_eq!(result[1].index, 2);
        assert_eq!(result[2].index, 3);
    }

    #[test]
    fn test_adjust_timing_min_duration() {
        let entries = vec![entry(1, 0, 100, "Short")]; // Only 100ms

        let config = PostProcessConfig {
            min_duration: Duration::from_secs(1),
            ..Default::default()
        };

        let result = adjust_timing(entries, &config);

        assert_eq!(result[0].end, Duration::from_secs(1));
    }

    #[test]
    fn test_adjust_timing_max_duration() {
        let entries = vec![entry(1, 0, 10000, "Long subtitle")]; // 10 seconds

        let config = PostProcessConfig {
            max_duration: Duration::from_secs(7),
            ..Default::default()
        };

        let result = adjust_timing(entries, &config);

        assert_eq!(result[0].end, Duration::from_secs(7));
    }

    #[test]
    fn test_post_process_integration() {
        let entries = vec![
            entry(1, 0, 1000, "Hello"),
            entry(2, 1200, 2000, "world"),
            entry(3, 5000, 5100, "Short"), // Too short
        ];

        let config = PostProcessConfig::default();
        let result = post_process(entries, &config);

        // Should merge first two and extend the short one
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Hello world");
        assert!(result[1].end - result[1].start >= config.min_duration);

        // Should be renumbered
        assert_eq!(result[0].index, 1);
        assert_eq!(result[1].index, 2);
    }
}
