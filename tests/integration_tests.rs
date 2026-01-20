//! Integration tests for autosub
//!
//! These tests validate the integration between components without requiring
//! external API keys.

use autosub::audio::{AudioMetadata, ChunkConfig, SpeechRegion};
use autosub::config::{Config, OutputFormat};
use autosub::pipeline::PipelineConfig;
use autosub::subtitle::{
    convert_to_subtitles, convert_with_defaults, create_formatter, json::JsonFormatter,
    quick_convert, srt::SrtFormatter, vtt::VttFormatter, PostProcessConfig, SubtitleEntry,
    SubtitleFormatter,
};
use autosub::transcribe::{Transcript, TranscriptSegment};

use std::time::Duration;

// ============================================================================
// Config Integration Tests
// ============================================================================

mod config_tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert_eq!(config.default_format, OutputFormat::Srt);
        assert_eq!(config.concurrency, 4);
    }

    #[test]
    fn test_config_gemini_validation() {
        let mut config = Config::default();
        config.gemini_api_key = None;

        let result = config.validate();
        assert!(result.is_err());

        config.gemini_api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_output_format_extensions() {
        assert_eq!(OutputFormat::Srt.extension(), "srt");
        assert_eq!(OutputFormat::Vtt.extension(), "vtt");
        assert_eq!(OutputFormat::Json.extension(), "json");
    }
}

// ============================================================================
// Subtitle Formatter Integration Tests
// ============================================================================

mod subtitle_formatter_tests {
    use super::*;

    fn sample_entries() -> Vec<SubtitleEntry> {
        vec![
            SubtitleEntry {
                index: 1,
                start: Duration::from_millis(1500),
                end: Duration::from_millis(4000),
                text: "Hello, welcome to this video.".to_string(),
                speaker: None,
            },
            SubtitleEntry {
                index: 2,
                start: Duration::from_millis(4500),
                end: Duration::from_millis(7000),
                text: "Today we're going to learn.".to_string(),
                speaker: None,
            },
        ]
    }

    #[test]
    fn test_srt_formatter_integration() {
        let formatter = SrtFormatter;
        let entries = sample_entries();
        let output = formatter.format(&entries);

        assert!(output.contains("1\n"));
        assert!(output.contains("00:00:01,500 --> 00:00:04,000"));
        assert!(output.contains("Hello, welcome to this video."));
        assert!(output.contains("2\n"));
        assert_eq!(formatter.extension(), "srt");
    }

    #[test]
    fn test_vtt_formatter_integration() {
        let formatter = VttFormatter;
        let entries = sample_entries();
        let output = formatter.format(&entries);

        assert!(output.starts_with("WEBVTT\n"));
        assert!(output.contains("00:00:01.500 --> 00:00:04.000"));
        assert!(output.contains("Hello, welcome to this video."));
        assert_eq!(formatter.extension(), "vtt");
    }

    #[test]
    fn test_json_formatter_integration() {
        let formatter = JsonFormatter {
            source_file: Some("test.mp4".to_string()),
            language: Some("en".to_string()),
            provider: Some("gemini".to_string()),
        };
        let entries = sample_entries();
        let output = formatter.format(&entries);

        assert!(output.contains("\"metadata\""));
        assert!(output.contains("\"subtitles\""));
        assert!(output.contains("\"source_file\": \"test.mp4\""));
        assert!(output.contains("Hello, welcome to this video."));
        assert_eq!(formatter.extension(), "json");
    }

    #[test]
    fn test_create_formatter_factory() {
        let srt = create_formatter(OutputFormat::Srt);
        assert_eq!(srt.extension(), "srt");

        let vtt = create_formatter(OutputFormat::Vtt);
        assert_eq!(vtt.extension(), "vtt");

        let json = create_formatter(OutputFormat::Json);
        assert_eq!(json.extension(), "json");
    }

    #[test]
    fn test_multiline_subtitle_formatting() {
        let entries = vec![SubtitleEntry {
            index: 1,
            start: Duration::from_secs(0),
            end: Duration::from_secs(5),
            text: "This is line one.\nThis is line two.".to_string(),
            speaker: None,
        }];

        let formatter = SrtFormatter;
        let output = formatter.format(&entries);

        assert!(output.contains("This is line one.\nThis is line two."));
    }
}

// ============================================================================
// Transcript to Subtitle Conversion Tests
// ============================================================================

mod conversion_tests {
    use super::*;

    fn sample_segments() -> Vec<TranscriptSegment> {
        vec![
            TranscriptSegment {
                start: Duration::from_millis(0),
                end: Duration::from_millis(2500),
                text: "First segment here.".to_string(),
                speaker: None,
                confidence: Some(0.95),
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_millis(3000),
                end: Duration::from_millis(5500),
                text: "Second segment here.".to_string(),
                speaker: Some("Speaker A".to_string()),
                confidence: Some(0.90),
                words: None,
            },
        ]
    }

    #[test]
    fn test_quick_convert_no_processing() {
        let segments = sample_segments();
        let entries = quick_convert(segments);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 1);
        assert_eq!(entries[1].index, 2);
        assert_eq!(entries[0].text, "First segment here.");
    }

    #[test]
    fn test_convert_with_speaker_labels() {
        let segments = sample_segments();
        let entries = quick_convert(segments);

        // Second entry should have speaker label
        assert_eq!(entries[1].text, "[Speaker A] Second segment here.");
    }

    #[test]
    fn test_convert_with_default_processing() {
        let segments = vec![TranscriptSegment {
            start: Duration::from_millis(0),
            end: Duration::from_millis(200), // Very short - should extend
            text: "Short.".to_string(),
            speaker: None,
            confidence: None,
            words: None,
        }];

        let entries = convert_with_defaults(segments);

        // Should be extended to minimum 1s
        assert!(entries[0].end >= Duration::from_secs(1));
    }

    #[test]
    fn test_convert_removes_filler_words() {
        let segments = vec![TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(3),
            text: "So, um, you know, this is like important.".to_string(),
            speaker: None,
            confidence: None,
            words: None,
        }];

        let config = PostProcessConfig {
            remove_fillers: true,
            ..Default::default()
        };

        let entries = convert_to_subtitles(segments, Some(config));

        // Filler words should be removed
        assert!(!entries[0].text.contains(" um "));
        assert!(!entries[0].text.contains("you know"));
    }

    #[test]
    fn test_convert_merges_close_segments() {
        let segments = vec![
            TranscriptSegment {
                start: Duration::from_millis(0),
                end: Duration::from_millis(1000),
                text: "First.".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_millis(1050), // Only 50ms gap
                end: Duration::from_millis(2000),
                text: "Second.".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
        ];

        let config = PostProcessConfig {
            merge_threshold: Duration::from_secs(1),
            ..Default::default()
        };

        let entries = convert_to_subtitles(segments, Some(config));

        // Should merge since gap < threshold
        assert_eq!(entries.len(), 1);
        assert!(entries[0].text.contains("First.") && entries[0].text.contains("Second."));
    }
}

// ============================================================================
// Audio Module Integration Tests
// ============================================================================

mod audio_tests {
    use super::*;
    use autosub::audio::chunk::plan_chunks;
    use autosub::audio::vad::VadConfig;

    #[test]
    fn test_audio_metadata_struct() {
        let metadata = AudioMetadata {
            duration: Duration::from_secs(120),
            sample_rate: 16000,
            channels: 1,
        };

        assert_eq!(metadata.duration, Duration::from_secs(120));
        assert_eq!(metadata.sample_rate, 16000);
        assert_eq!(metadata.channels, 1);
    }

    #[test]
    fn test_speech_region_struct() {
        let region = SpeechRegion {
            start: Duration::from_secs(10),
            end: Duration::from_secs(25),
        };

        assert_eq!(region.start, Duration::from_secs(10));
        assert_eq!(region.end, Duration::from_secs(25));
    }

    #[test]
    fn test_chunk_config_gemini_defaults() {
        let config = ChunkConfig::gemini();

        assert_eq!(config.max_duration, Duration::from_secs(60));
        assert_eq!(config.max_file_size, 20 * 1024 * 1024);
    }

    #[test]
    fn test_vad_config_defaults() {
        let config = VadConfig::default();

        assert!(config.energy_threshold > 0.0);
        assert!(config.min_speech_duration > Duration::ZERO);
        assert!(config.min_silence_duration > Duration::ZERO);
    }

    #[test]
    fn test_plan_chunks_with_short_regions() {
        // Use regions that together exceed max_duration so they won't be merged
        let regions = vec![
            SpeechRegion {
                start: Duration::from_secs(0),
                end: Duration::from_secs(40),
            },
            SpeechRegion {
                start: Duration::from_secs(50),
                end: Duration::from_secs(90),
            },
        ];

        let config = ChunkConfig {
            max_duration: Duration::from_secs(60),
            ..Default::default()
        };
        let total_duration = Duration::from_secs(100);
        let chunks = plan_chunks(&regions, total_duration, &config);

        // Two regions that together exceed max_duration should stay as two chunks
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn test_plan_chunks_splits_long_region() {
        let regions = vec![SpeechRegion {
            start: Duration::from_secs(0),
            end: Duration::from_secs(120), // 2 minutes
        }];

        let config = ChunkConfig {
            max_duration: Duration::from_secs(60),
            ..Default::default()
        };

        let total_duration = Duration::from_secs(120);
        let chunks = plan_chunks(&regions, total_duration, &config);

        // Should split into 2 chunks
        assert_eq!(chunks.len(), 2);
    }
}

// ============================================================================
// Pipeline Config Tests
// ============================================================================

mod pipeline_config_tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();

        assert_eq!(config.format, OutputFormat::Srt);
        assert_eq!(config.language, "en");
        assert!(config.concurrency > 0);
    }

    #[test]
    fn test_pipeline_config_custom() {
        let config = PipelineConfig {
            format: OutputFormat::Vtt,
            language: "ja".to_string(),
            translate_to: Some("en".to_string()),
            concurrency: 8,
            post_process: Some(PostProcessConfig::default()),
            show_progress: true,
        };

        assert_eq!(config.format, OutputFormat::Vtt);
        assert_eq!(config.language, "ja");
        assert_eq!(config.translate_to, Some("en".to_string()));
        assert_eq!(config.concurrency, 8);
    }
}

// ============================================================================
// End-to-End Formatting Tests
// ============================================================================

mod e2e_formatting_tests {
    use super::*;

    #[test]
    fn test_full_srt_workflow() {
        // Simulate a complete workflow: segments -> entries -> SRT output
        let segments = vec![
            TranscriptSegment {
                start: Duration::from_millis(500),
                end: Duration::from_millis(3000),
                text: "Welcome to the tutorial.".to_string(),
                speaker: None,
                confidence: Some(0.99),
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_millis(3500),
                end: Duration::from_millis(6000),
                text: "Let's get started.".to_string(),
                speaker: None,
                confidence: Some(0.98),
                words: None,
            },
        ];

        // Convert to subtitle entries
        let entries = quick_convert(segments);

        // Format as SRT
        let formatter = create_formatter(OutputFormat::Srt);
        let srt_output = formatter.format(&entries);

        // Verify output
        assert!(srt_output.contains("1\n"));
        assert!(srt_output.contains("00:00:00,500 --> 00:00:03,000"));
        assert!(srt_output.contains("Welcome to the tutorial."));
        assert!(srt_output.contains("2\n"));
        assert!(srt_output.contains("00:00:03,500 --> 00:00:06,000"));
        assert!(srt_output.contains("Let's get started."));
    }

    #[test]
    fn test_full_vtt_workflow() {
        let segments = vec![TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(5),
            text: "Hello World".to_string(),
            speaker: None,
            confidence: None,
            words: None,
        }];

        let entries = quick_convert(segments);
        let formatter = create_formatter(OutputFormat::Vtt);
        let vtt_output = formatter.format(&entries);

        assert!(vtt_output.starts_with("WEBVTT\n"));
        assert!(vtt_output.contains("00:00:00.000 --> 00:00:05.000"));
        assert!(vtt_output.contains("Hello World"));
    }

    #[test]
    fn test_workflow_with_speaker_diarization() {
        let segments = vec![
            TranscriptSegment {
                start: Duration::from_secs(0),
                end: Duration::from_secs(3),
                text: "How are you?".to_string(),
                speaker: Some("Alice".to_string()),
                confidence: None,
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_secs(4),
                end: Duration::from_secs(7),
                text: "I'm doing great!".to_string(),
                speaker: Some("Bob".to_string()),
                confidence: None,
                words: None,
            },
        ];

        let entries = quick_convert(segments);
        let formatter = SrtFormatter;
        let output = formatter.format(&entries);

        assert!(output.contains("[Alice] How are you?"));
        assert!(output.contains("[Bob] I'm doing great!"));
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_segments() {
        let segments: Vec<TranscriptSegment> = vec![];
        let entries = quick_convert(segments);

        assert!(entries.is_empty());

        let formatter = SrtFormatter;
        let output = formatter.format(&entries);
        assert!(output.is_empty());
    }

    #[test]
    fn test_single_very_short_segment() {
        let segments = vec![TranscriptSegment {
            start: Duration::from_millis(0),
            end: Duration::from_millis(100), // 100ms
            text: "Hi".to_string(),
            speaker: None,
            confidence: None,
            words: None,
        }];

        let entries = convert_with_defaults(segments);

        // Should be extended to minimum duration
        assert!(entries[0].end >= Duration::from_secs(1));
    }

    #[test]
    fn test_overlapping_segments_fixed() {
        let segments = vec![
            TranscriptSegment {
                start: Duration::from_secs(0),
                end: Duration::from_secs(5),
                text: "First".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_secs(4), // Overlaps!
                end: Duration::from_secs(8),
                text: "Second".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
        ];

        let entries = quick_convert(segments);

        // First entry's end should be adjusted to not overlap
        assert!(entries[0].end <= entries[1].start);
    }

    #[test]
    fn test_whitespace_handling() {
        let segments = vec![TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(3),
            text: "  trimmed text  ".to_string(),
            speaker: None,
            confidence: None,
            words: None,
        }];

        let entries = quick_convert(segments);

        // Should trim whitespace
        assert_eq!(entries[0].text, "trimmed text");
    }

    #[test]
    fn test_unicode_text() {
        let segments = vec![
            TranscriptSegment {
                start: Duration::from_secs(0),
                end: Duration::from_secs(3),
                text: "日本語テスト".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
            TranscriptSegment {
                start: Duration::from_secs(4),
                end: Duration::from_secs(7),
                text: "🎬 Emoji support".to_string(),
                speaker: None,
                confidence: None,
                words: None,
            },
        ];

        let entries = quick_convert(segments);
        let formatter = SrtFormatter;
        let output = formatter.format(&entries);

        assert!(output.contains("日本語テスト"));
        assert!(output.contains("🎬 Emoji support"));
    }

    #[test]
    fn test_very_long_text_splitting() {
        let long_text = "This is a very long sentence that should probably be split into multiple lines because it exceeds the typical maximum character limit for subtitle display which is usually around 42 characters per line.";

        let segments = vec![TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(10),
            text: long_text.to_string(),
            speaker: None,
            confidence: None,
            words: None,
        }];

        let config = PostProcessConfig {
            max_line_length: 42,
            ..Default::default()
        };

        let entries = convert_to_subtitles(segments, Some(config));

        // Should have split into multiple entries
        assert!(entries.len() > 1);
    }
}

// ============================================================================
// Transcript Aggregation Tests
// ============================================================================

mod transcript_tests {
    use super::*;

    #[test]
    fn test_transcript_single() {
        let segment = TranscriptSegment {
            start: Duration::from_secs(0),
            end: Duration::from_secs(5),
            text: "Test segment.".to_string(),
            speaker: None,
            confidence: Some(0.95),
            words: None,
        };

        let transcript = Transcript::single(segment);

        assert_eq!(transcript.segments.len(), 1);
        assert_eq!(transcript.segments[0].text, "Test segment.");
    }

    #[test]
    fn test_empty_transcript() {
        let transcript = Transcript::empty();

        assert!(transcript.segments.is_empty());
        assert!(transcript.language.is_none());
        assert!(transcript.duration.is_none());
    }
}
