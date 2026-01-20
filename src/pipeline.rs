use crate::audio::{
    check_ffmpeg, cleanup_chunks, create_chunks, extract_audio, get_audio_duration, plan_chunks,
    AudioChunk, ChunkConfig,
};
use crate::config::{Config, OutputFormat};
use crate::error::{AutosubError, Result};
use crate::subtitle::{convert_with_defaults, create_formatter, PostProcessConfig, SubtitleEntry};
use crate::transcribe::{GeminiClient, Transcriber, TranscriptionOrchestrator};
use crate::translate::create_translator;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tracing::{debug, info, warn};

/// Configuration for the subtitle generation pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Output subtitle format.
    pub format: OutputFormat,
    /// Source language code.
    pub language: String,
    /// Target language for translation (optional).
    pub translate_to: Option<String>,
    /// Number of concurrent API requests.
    pub concurrency: usize,
    /// Post-processing configuration.
    pub post_process: Option<PostProcessConfig>,
    /// Show progress bars.
    pub show_progress: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::default(),
            language: "en".to_string(),
            translate_to: None,
            concurrency: 4,
            post_process: Some(PostProcessConfig::default()),
            show_progress: true,
        }
    }
}

/// Statistics from the subtitle generation process.
#[derive(Debug, Clone)]
pub struct PipelineStats {
    /// Total time taken for the entire pipeline.
    pub total_time: Duration,
    /// Time taken for audio extraction.
    pub extraction_time: Duration,
    /// Time taken for transcription.
    pub transcription_time: Duration,
    /// Time taken for translation (if performed).
    pub translation_time: Option<Duration>,
    /// Number of audio chunks processed.
    pub chunks_processed: usize,
    /// Number of subtitle entries generated.
    pub subtitle_entries: usize,
    /// Total audio duration.
    pub audio_duration: Duration,
    /// Provider used for transcription.
    pub provider: String,
    /// Target language for translation (if performed).
    pub translated_to: Option<String>,
}

/// Result of the subtitle generation pipeline.
#[derive(Debug)]
pub struct PipelineResult {
    /// Path to the output subtitle file.
    pub output_path: PathBuf,
    /// Generated subtitle entries.
    pub entries: Vec<SubtitleEntry>,
    /// Pipeline statistics.
    pub stats: PipelineStats,
    /// Detected language (if different from specified).
    pub detected_language: Option<String>,
}

/// Cleanup guard that removes temp directory when dropped.
struct TempCleanupGuard {
    temp_dir: Option<TempDir>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for TempCleanupGuard {
    fn drop(&mut self) {
        if let Some(temp_dir) = self.temp_dir.take() {
            let path = temp_dir.path().to_path_buf();
            if self.cancelled.load(Ordering::Relaxed) {
                warn!("Pipeline cancelled, cleaning up temp files: {:?}", path);
            } else {
                debug!("Cleaning up temp directory: {:?}", path);
            }
            // TempDir automatically deletes on drop
        }
    }
}

/// Generate subtitles from a video or audio file.
///
/// This is the main entry point for the autosub pipeline. It:
/// 1. Extracts audio from the input file
/// 2. Chunks the audio for API processing
/// 3. Transcribes using the selected provider
/// 4. Optionally translates to target language
/// 5. Post-processes and formats as subtitles
/// 6. Writes the output file
pub async fn generate_subtitles(
    input: &Path,
    output: &Path,
    config: &Config,
    pipeline_config: PipelineConfig,
) -> Result<PipelineResult> {
    let cancelled = Arc::new(AtomicBool::new(false));
    generate_subtitles_with_cancel(input, output, config, pipeline_config, cancelled).await
}

/// Generate subtitles with cancellation support.
pub async fn generate_subtitles_with_cancel(
    input: &Path,
    output: &Path,
    config: &Config,
    pipeline_config: PipelineConfig,
    cancelled: Arc<AtomicBool>,
) -> Result<PipelineResult> {
    let start_time = Instant::now();

    // Validate input file exists
    if !input.exists() {
        return Err(AutosubError::FileNotFound(input.display().to_string()));
    }

    // Check FFmpeg is available
    check_ffmpeg().map_err(|_| {
        AutosubError::AudioExtraction(
            "FFmpeg not found. Install it with: brew install ffmpeg (macOS) or apt install ffmpeg (Linux)".to_string()
        )
    })?;

    // Create temp directory for intermediate files
    let temp_dir = TempDir::new().map_err(|e| {
        AutosubError::Io(std::io::Error::other(format!(
            "Failed to create temp directory: {}",
            e
        )))
    })?;

    let _cleanup_guard = TempCleanupGuard {
        temp_dir: Some(TempDir::new().unwrap_or_else(|_| {
            // Fallback: just use the existing temp_dir path
            TempDir::new().expect("Failed to create temp dir")
        })),
        cancelled: cancelled.clone(),
    };

    let temp_path = temp_dir.path();
    debug!("Using temp directory: {:?}", temp_path);

    // Setup progress bars if enabled
    let multi_progress = if pipeline_config.show_progress {
        Some(MultiProgress::new())
    } else {
        None
    };

    // Check for cancellation
    if cancelled.load(Ordering::Relaxed) {
        return Err(AutosubError::Transcription(
            "Pipeline cancelled".to_string(),
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Stage 1: Audio Extraction
    // ═══════════════════════════════════════════════════════════════════════
    info!("Stage 1/4: Extracting audio from {:?}", input);
    let extraction_start = Instant::now();

    let extraction_pb = multi_progress.as_ref().map(|mp| {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Extracting audio...");
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    });

    let audio_path = temp_path.join("audio.wav");
    let audio_metadata = extract_audio(input, &audio_path).await?;

    if let Some(pb) = extraction_pb {
        pb.finish_with_message(format!(
            "✓ Audio extracted ({:.1}s)",
            audio_metadata.duration.as_secs_f64()
        ));
    }

    let extraction_time = extraction_start.elapsed();
    info!(
        "Audio extraction complete: {:.1}s duration in {:.2}s",
        audio_metadata.duration.as_secs_f64(),
        extraction_time.as_secs_f64()
    );

    // Check for cancellation
    if cancelled.load(Ordering::Relaxed) {
        return Err(AutosubError::Transcription(
            "Pipeline cancelled".to_string(),
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Stage 2: Audio Chunking
    // ═══════════════════════════════════════════════════════════════════════
    info!("Stage 2/4: Chunking audio for API");

    let chunking_pb = multi_progress.as_ref().map(|mp| {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Planning audio chunks...");
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    });

    // Get chunk config for Gemini
    let chunk_config = ChunkConfig::gemini();

    // Get audio duration
    let audio_duration = get_audio_duration(&audio_path).unwrap_or(audio_metadata.duration);

    // Plan chunks (use empty regions for fixed-duration chunking)
    let empty_regions: Vec<crate::audio::SpeechRegion> = Vec::new();
    let planned_chunks = plan_chunks(&empty_regions, audio_duration, &chunk_config);

    if let Some(pb) = &chunking_pb {
        pb.set_message(format!("Creating {} chunks...", planned_chunks.len()));
    }

    // Create actual chunk files
    let chunks: Vec<AudioChunk> = create_chunks(&audio_path, &planned_chunks, temp_path).await?;

    if let Some(pb) = chunking_pb {
        pb.finish_with_message(format!("✓ Created {} audio chunks", chunks.len()));
    }

    info!("Created {} audio chunks", chunks.len());

    // Check for cancellation
    if cancelled.load(Ordering::Relaxed) {
        let _ = cleanup_chunks(&chunks);
        return Err(AutosubError::Transcription(
            "Pipeline cancelled".to_string(),
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Stage 3: Transcription
    // ═══════════════════════════════════════════════════════════════════════
    info!(
        "Stage 3/4: Transcribing with Gemini (concurrency: {})",
        pipeline_config.concurrency
    );
    let transcription_start = Instant::now();

    // Create transcriber with language set
    let api_key = config.gemini_api_key.as_ref().ok_or_else(|| {
        AutosubError::Config(
            "Gemini API key not set. Set GEMINI_API_KEY environment variable.".to_string(),
        )
    })?;
    let transcriber: Box<dyn Transcriber> = Box::new(
        GeminiClient::new(api_key.clone()).with_language(pipeline_config.language.clone()),
    );

    // Create orchestrator
    let orchestrator = TranscriptionOrchestrator::new(transcriber, pipeline_config.concurrency)
        .with_progress(pipeline_config.show_progress);

    // Process chunks
    let (transcription_result, transcription_stats) =
        orchestrator.process_chunks(chunks.clone()).await?;

    let transcription_time = transcription_start.elapsed();
    info!(
        "Transcription complete: {} segments in {:.2}s",
        transcription_result.segments.len(),
        transcription_time.as_secs_f64()
    );

    // Cleanup chunk files after transcription
    let _ = cleanup_chunks(&chunks);

    // Check for cancellation
    if cancelled.load(Ordering::Relaxed) {
        return Err(AutosubError::Transcription(
            "Pipeline cancelled".to_string(),
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Stage 4: Translation (Optional)
    // ═══════════════════════════════════════════════════════════════════════
    let mut translation_time: Option<Duration> = None;
    let mut translated_to: Option<String> = None;

    let mut segments = transcription_result.segments.clone();

    if let Some(ref target_lang) = pipeline_config.translate_to {
        info!("Stage 4/5: Translating to {}", target_lang);
        let translation_start = Instant::now();

        let translation_pb = multi_progress.as_ref().map(|mp| {
            let pb = mp.add(ProgressBar::new(segments.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb.set_message("Translating...");
            pb
        });

        // Create translator using Gemini API key
        let translator = create_translator(config.gemini_api_key.as_deref())?;

        // Translate in batches for efficiency
        let batch_size = 10;
        let mut translated_segments = Vec::with_capacity(segments.len());

        for batch in segments.chunks(batch_size) {
            // Check for cancellation
            if cancelled.load(Ordering::Relaxed) {
                return Err(AutosubError::Transcription(
                    "Pipeline cancelled during translation".to_string(),
                ));
            }

            let texts: Vec<&str> = batch.iter().map(|s| s.text.as_str()).collect();
            let translations = translator.translate_batch(&texts, target_lang).await?;

            for (segment, translated_text) in batch.iter().zip(translations.into_iter()) {
                let mut new_segment = segment.clone();
                new_segment.text = translated_text;
                translated_segments.push(new_segment);

                if let Some(ref pb) = translation_pb {
                    pb.inc(1);
                }
            }
        }

        segments = translated_segments;
        translation_time = Some(translation_start.elapsed());
        translated_to = Some(target_lang.clone());

        if let Some(pb) = translation_pb {
            pb.finish_with_message(format!("✓ Translated to {}", target_lang));
        }

        info!(
            "Translation complete: {} segments in {:.2}s",
            segments.len(),
            translation_time.unwrap().as_secs_f64()
        );
    }

    // Check for cancellation
    if cancelled.load(Ordering::Relaxed) {
        return Err(AutosubError::Transcription(
            "Pipeline cancelled".to_string(),
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Stage 5: Subtitle Generation
    // ═══════════════════════════════════════════════════════════════════════
    let stage_num = if pipeline_config.translate_to.is_some() {
        "5/5"
    } else {
        "4/4"
    };
    info!(
        "Stage {}: Generating {} subtitles",
        stage_num, pipeline_config.format
    );

    let subtitle_pb = multi_progress.as_ref().map(|mp| {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Formatting subtitles...");
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    });

    // Convert transcript to subtitle entries with post-processing
    let subtitle_entries = if pipeline_config.post_process.is_some() {
        convert_with_defaults(segments)
    } else {
        crate::subtitle::quick_convert(segments)
    };

    // Format subtitles
    let formatter = create_formatter(pipeline_config.format);
    let subtitle_content = formatter.format(&subtitle_entries);

    // Write output file
    fs::write(output, &subtitle_content)?;

    if let Some(pb) = subtitle_pb {
        pb.finish_with_message(format!(
            "✓ Generated {} subtitle entries",
            subtitle_entries.len()
        ));
    }

    info!("Wrote {} entries to {:?}", subtitle_entries.len(), output);

    // Build result
    let total_time = start_time.elapsed();

    let stats = PipelineStats {
        total_time,
        extraction_time,
        transcription_time,
        translation_time,
        chunks_processed: transcription_stats.successful_chunks,
        subtitle_entries: subtitle_entries.len(),
        audio_duration,
        provider: "gemini".to_string(),
        translated_to,
    };

    let detected_language = if transcription_result.language != pipeline_config.language
        && transcription_result.language != "unknown"
    {
        Some(transcription_result.language)
    } else {
        None
    };

    Ok(PipelineResult {
        output_path: output.to_path_buf(),
        entries: subtitle_entries,
        stats,
        detected_language,
    })
}

/// Print a summary of the pipeline results.
pub fn print_summary(result: &PipelineResult) {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("                      Subtitle Generation Complete              ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("  Output:     {}", result.output_path.display());
    println!("  Entries:    {}", result.stats.subtitle_entries);
    println!("  Provider:   {}", result.stats.provider);
    if let Some(ref target_lang) = result.stats.translated_to {
        println!("  Translated: {}", target_lang);
    }
    println!(
        "  Duration:   {:.1}s audio",
        result.stats.audio_duration.as_secs_f64()
    );
    println!();
    println!("  Timing:");
    println!(
        "    Extract:     {:.2}s",
        result.stats.extraction_time.as_secs_f64()
    );
    println!(
        "    Transcribe:  {:.2}s ({} chunks)",
        result.stats.transcription_time.as_secs_f64(),
        result.stats.chunks_processed
    );
    if let Some(translation_time) = result.stats.translation_time {
        println!("    Translate:   {:.2}s", translation_time.as_secs_f64());
    }
    println!(
        "    Total:       {:.2}s",
        result.stats.total_time.as_secs_f64()
    );
    if let Some(ref lang) = result.detected_language {
        println!();
        println!(
            "  Note: Detected language '{}' differs from specified",
            lang
        );
    }
    println!();
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.format, OutputFormat::Srt);
        assert_eq!(config.language, "en");
        assert_eq!(config.concurrency, 4);
        assert!(config.post_process.is_some());
        assert!(config.show_progress);
    }

    #[test]
    fn test_pipeline_stats_display() {
        let stats = PipelineStats {
            total_time: Duration::from_secs(30),
            extraction_time: Duration::from_secs(5),
            transcription_time: Duration::from_secs(20),
            translation_time: None,
            chunks_processed: 5,
            subtitle_entries: 50,
            audio_duration: Duration::from_secs(300),
            provider: "gemini".to_string(),
            translated_to: None,
        };

        assert_eq!(stats.chunks_processed, 5);
        assert_eq!(stats.subtitle_entries, 50);
    }

    #[test]
    fn test_pipeline_stats_with_translation() {
        let stats = PipelineStats {
            total_time: Duration::from_secs(35),
            extraction_time: Duration::from_secs(5),
            transcription_time: Duration::from_secs(20),
            translation_time: Some(Duration::from_secs(5)),
            chunks_processed: 5,
            subtitle_entries: 50,
            audio_duration: Duration::from_secs(300),
            provider: "gemini".to_string(),
            translated_to: Some("es".to_string()),
        };

        assert!(stats.translation_time.is_some());
        assert_eq!(stats.translated_to, Some("es".to_string()));
    }
}
