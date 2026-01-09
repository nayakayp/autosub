use anyhow::{Context, Result};
use autosub::config::{Config, OutputFormat, Provider};
use autosub::{print_summary, PipelineConfig};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "autosub")]
#[command(version, about = "Automatic subtitle generation using AI")]
#[command(
    long_about = "Generate subtitles from video/audio files using OpenAI Whisper or Google Gemini APIs."
)]
struct Cli {
    /// Input video/audio file
    input: PathBuf,

    /// Output subtitle file (defaults to input name with appropriate extension)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format: srt, vtt, json
    #[arg(short, long, default_value = "srt")]
    format: String,

    /// Transcription provider: whisper, gemini
    #[arg(short, long, default_value = "whisper")]
    provider: String,

    /// Source language code (e.g., en, ja, es)
    #[arg(short, long, default_value = "en")]
    language: String,

    /// Translate to target language (e.g., en, es, fr)
    #[arg(long)]
    translate: Option<String>,

    /// Number of concurrent API requests
    #[arg(short, long, default_value = "4")]
    concurrency: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Suppress progress bars and output
    #[arg(short, long)]
    quiet: bool,

    /// Validate input without processing (check dependencies, API keys, etc.)
    #[arg(long)]
    dry_run: bool,

    /// Overwrite output file if it already exists
    #[arg(long)]
    force: bool,
}

fn init_logging(verbose: bool) {
    let level = if verbose { Level::DEBUG } else { Level::INFO };

    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();
}

fn derive_output_path(input: &Path, format: &OutputFormat) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default();
    let mut output = input.to_path_buf();
    output.set_file_name(format!("{}.{}", stem.to_string_lossy(), format.extension()));
    output
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_logging(cli.verbose);

    // Validate input file exists
    if !cli.input.exists() {
        anyhow::bail!("Input file not found: {}", cli.input.display());
    }

    // Parse format
    let format: OutputFormat = cli.format.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    // Parse provider
    let provider: Provider = cli
        .provider
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    // Derive output path if not specified
    let output = cli
        .output
        .unwrap_or_else(|| derive_output_path(&cli.input, &format));

    // Check if output file exists and --force not specified
    if output.exists() && !cli.force && !cli.dry_run {
        anyhow::bail!(
            "Output file already exists: {}\nUse --force to overwrite.",
            output.display()
        );
    }

    // Load and validate configuration
    let config = Config::load().context("Failed to load configuration")?;
    config
        .validate(provider)
        .context("Configuration validation failed")?;

    // Check FFmpeg availability
    autosub::audio::check_ffmpeg()
        .context("FFmpeg not found. Install it with: brew install ffmpeg (macOS) or apt install ffmpeg (Linux)")?;

    if !cli.quiet {
        info!("Input:    {}", cli.input.display());
        info!("Output:   {}", output.display());
        info!("Format:   {}", format);
        info!("Provider: {}", provider);
        info!("Language: {}", cli.language);
        if let Some(ref target) = cli.translate {
            info!("Translate to: {}", target);
        }
    }

    // Dry run mode - validate everything but don't process
    if cli.dry_run {
        println!();
        println!("✓ Dry run validation successful:");
        println!("  Input file:    {} (exists)", cli.input.display());
        println!("  Output file:   {}", output.display());
        println!("  Format:        {}", format);
        println!("  Provider:      {} (API key set)", provider);
        println!("  Language:      {}", cli.language);
        println!("  Concurrency:   {}", cli.concurrency);
        println!("  FFmpeg:        available");
        if output.exists() {
            println!("  ⚠ Output file exists (will be overwritten with --force)");
        }
        println!();
        println!("Run without --dry-run to process the file.");
        return Ok(());
    }

    // Setup Ctrl+C handler for graceful cancellation
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();

    ctrlc::set_handler(move || {
        if cancelled_clone.load(Ordering::Relaxed) {
            // Second Ctrl+C, force exit
            std::process::exit(1);
        }
        eprintln!("\nReceived Ctrl+C, cancelling... (press again to force quit)");
        cancelled_clone.store(true, Ordering::Relaxed);
    })
    .ok();

    // Build pipeline configuration
    let pipeline_config = PipelineConfig {
        provider,
        format,
        language: cli.language.clone(),
        translate_to: cli.translate.clone(),
        concurrency: cli.concurrency,
        post_process: Some(autosub::subtitle::PostProcessConfig::default()),
        show_progress: !cli.quiet,
    };

    // Run the pipeline
    match autosub::pipeline::generate_subtitles_with_cancel(
        &cli.input,
        &output,
        &config,
        pipeline_config,
        cancelled,
    )
    .await
    {
        Ok(result) => {
            if !cli.quiet {
                print_summary(&result);
            }
            Ok(())
        }
        Err(e) => {
            error!("Pipeline failed: {}", e);
            Err(anyhow::anyhow!("{}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_output_path() {
        let input = PathBuf::from("/path/to/video.mp4");

        let srt_output = derive_output_path(&input, &OutputFormat::Srt);
        assert_eq!(srt_output, PathBuf::from("/path/to/video.srt"));

        let vtt_output = derive_output_path(&input, &OutputFormat::Vtt);
        assert_eq!(vtt_output, PathBuf::from("/path/to/video.vtt"));

        let json_output = derive_output_path(&input, &OutputFormat::Json);
        assert_eq!(json_output, PathBuf::from("/path/to/video.json"));
    }
}
