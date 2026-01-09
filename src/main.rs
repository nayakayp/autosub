use anyhow::{Context, Result};
use autosub::config::{Config, OutputFormat, Provider};
use clap::Parser;
use std::path::{Path, PathBuf};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "autosub")]
#[command(version, about = "Automatic subtitle generation using AI")]
#[command(long_about = "Generate subtitles from video/audio files using OpenAI Whisper or Google Gemini APIs.")]
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
    let format: OutputFormat = cli
        .format
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    // Parse provider
    let provider: Provider = cli
        .provider
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    // Derive output path if not specified
    let output = cli.output.unwrap_or_else(|| derive_output_path(&cli.input, &format));

    // Load and validate configuration
    let config = Config::load().context("Failed to load configuration")?;
    config
        .validate(provider)
        .context("Configuration validation failed")?;

    info!("Input:    {}", cli.input.display());
    info!("Output:   {}", output.display());
    info!("Format:   {}", format);
    info!("Provider: {}", provider);
    info!("Language: {}", cli.language);
    if let Some(ref target) = cli.translate {
        info!("Translate to: {}", target);
    }

    // TODO: Implement pipeline stages
    // 1. Extract audio from input file
    // 2. Perform VAD and chunking
    // 3. Transcribe chunks using provider
    // 4. Optionally translate
    // 5. Format and write subtitles

    info!("Pipeline not yet implemented. Foundation complete!");
    info!(
        "Next steps: Implement audio extraction, transcription, and subtitle generation."
    );

    Ok(())
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
