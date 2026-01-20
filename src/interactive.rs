use crate::config::{Config, OutputFormat};
use crate::pipeline::PipelineConfig;
use console::style;
use dialoguer::{Confirm, Input, Select};
use std::fs;
use std::path::PathBuf;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", // Video
    "mp3", "wav", "flac", "m4a", "ogg", "aac", // Audio
];

const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("ja", "Japanese"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("zh", "Chinese"),
    ("ko", "Korean"),
    ("pt", "Portuguese"),
    ("it", "Italian"),
    ("ru", "Russian"),
    ("ar", "Arabic"),
    ("hi", "Hindi"),
    ("nl", "Dutch"),
    ("pl", "Polish"),
    ("tr", "Turkish"),
];

pub struct InteractiveResult {
    pub input: PathBuf,
    pub output: PathBuf,
    pub config: Config,
    pub pipeline_config: PipelineConfig,
}

pub fn run_interactive_wizard() -> anyhow::Result<InteractiveResult> {
    print_header();

    // Step 1: Check/Setup API Key
    let config = setup_api_key()?;

    // Step 2: Select source file
    let input = select_source_file()?;

    // Step 3: Select source language
    let language = select_language("Select source language:", 0)?;

    // Step 4: Translation (optional)
    let translate_to = setup_translation(&language)?;

    // Step 5: Select output format
    let format = select_output_format()?;

    // Derive output path
    let output = derive_output_path(&input, &format);

    // Step 6: Confirm
    print_summary(&input, &output, &language, &translate_to, &format);

    if !Confirm::new()
        .with_prompt("Proceed with these settings?")
        .default(true)
        .interact()?
    {
        anyhow::bail!("Cancelled by user");
    }

    println!();

    let pipeline_config = PipelineConfig {
        format,
        language,
        translate_to,
        concurrency: config.concurrency,
        post_process: Some(crate::subtitle::PostProcessConfig::default()),
        show_progress: true,
    };

    Ok(InteractiveResult {
        input,
        output,
        config,
        pipeline_config,
    })
}

fn print_header() {
    println!();
    println!(
        "{}",
        style("╔═══════════════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║           autosub - AI Subtitle Generator         ║").cyan()
    );
    println!(
        "{}",
        style("╚═══════════════════════════════════════════════════╝").cyan()
    );
    println!();
}

fn setup_api_key() -> anyhow::Result<Config> {
    let mut config = Config::load().unwrap_or_default();

    if config.gemini_api_key.is_some() {
        println!(
            "{} API key configured",
            style("✓").green()
        );
        return Ok(config);
    }

    println!(
        "{} Gemini API key not found",
        style("!").yellow()
    );
    println!("  Get one at: https://aistudio.google.com/apikey\n");

    let api_key: String = Input::new()
        .with_prompt("Enter your Gemini API key")
        .interact_text()?;

    if api_key.trim().is_empty() {
        anyhow::bail!("API key is required");
    }

    config.gemini_api_key = Some(api_key.trim().to_string());

    // Offer to save
    if Confirm::new()
        .with_prompt("Save API key to config file?")
        .default(true)
        .interact()?
    {
        save_config(&config)?;
        println!("{} API key saved to config\n", style("✓").green());
    }

    Ok(config)
}

fn save_config(config: &Config) -> anyhow::Result<()> {
    if let Some(config_dir) = dirs::config_dir() {
        let autosub_dir = config_dir.join("autosub");
        fs::create_dir_all(&autosub_dir)?;

        let config_path = autosub_dir.join("config.toml");
        let toml_content = toml::to_string_pretty(config)?;
        fs::write(config_path, toml_content)?;
    }
    Ok(())
}

fn select_source_file() -> anyhow::Result<PathBuf> {
    println!("\n{}", style("Select source file:").bold());

    let files = scan_media_files(".")?;

    if files.is_empty() {
        println!("  No media files found in current directory.\n");
        let path: String = Input::new()
            .with_prompt("Enter file path")
            .interact_text()?;
        let path = PathBuf::from(path);
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }
        return Ok(path);
    }

    let display_items: Vec<String> = files
        .iter()
        .map(|f| {
            let size = fs::metadata(f)
                .map(|m| format_size(m.len()))
                .unwrap_or_else(|_| "?".to_string());
            format!("{} ({})", f.display(), size)
        })
        .collect();

    let mut items = display_items.clone();
    items.push("Enter custom path...".to_string());

    let selection = Select::new()
        .with_prompt("Choose a file")
        .items(&items)
        .default(0)
        .interact()?;

    if selection == files.len() {
        // Custom path
        let path: String = Input::new()
            .with_prompt("Enter file path")
            .interact_text()?;
        let path = PathBuf::from(path);
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }
        Ok(path)
    } else {
        Ok(files[selection].clone())
    }
}

fn scan_media_files(dir: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        }
    }

    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn select_language(prompt: &str, default: usize) -> anyhow::Result<String> {
    let items: Vec<String> = LANGUAGES
        .iter()
        .map(|(code, name)| format!("{} ({})", name, code))
        .collect();

    let mut options = items.clone();
    options.push("Other (enter code)...".to_string());

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&options)
        .default(default)
        .interact()?;

    if selection == LANGUAGES.len() {
        let code: String = Input::new()
            .with_prompt("Enter language code (e.g., 'vi' for Vietnamese)")
            .interact_text()?;
        Ok(code.trim().to_lowercase())
    } else {
        Ok(LANGUAGES[selection].0.to_string())
    }
}

fn setup_translation(source_lang: &str) -> anyhow::Result<Option<String>> {
    if !Confirm::new()
        .with_prompt("Translate subtitles to another language?")
        .default(false)
        .interact()?
    {
        return Ok(None);
    }

    // Default to English if source is not English, otherwise Spanish
    let default_idx = if source_lang == "en" { 2 } else { 0 };

    let target = select_language("Select target language:", default_idx)?;

    if target == source_lang {
        println!(
            "{} Target language is same as source, skipping translation",
            style("!").yellow()
        );
        return Ok(None);
    }

    Ok(Some(target))
}

fn select_output_format() -> anyhow::Result<OutputFormat> {
    let formats = vec![
        ("SRT", "Most compatible (VLC, YouTube, etc.)", OutputFormat::Srt),
        ("VTT", "Web/HTML5 video", OutputFormat::Vtt),
        ("JSON", "Programmatic access", OutputFormat::Json),
    ];

    let items: Vec<String> = formats
        .iter()
        .map(|(name, desc, _)| format!("{} - {}", name, desc))
        .collect();

    let selection = Select::new()
        .with_prompt("Select output format")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(formats[selection].2)
}

fn derive_output_path(input: &PathBuf, format: &OutputFormat) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default();
    let mut output = input.clone();
    output.set_file_name(format!("{}.{}", stem.to_string_lossy(), format.extension()));
    output
}

fn print_summary(
    input: &PathBuf,
    output: &PathBuf,
    language: &str,
    translate_to: &Option<String>,
    format: &OutputFormat,
) {
    println!("\n{}", style("═══ Summary ═══").bold());
    println!("  Input:     {}", style(input.display()).cyan());
    println!("  Output:    {}", style(output.display()).cyan());
    println!("  Language:  {}", get_language_name(language));
    if let Some(target) = translate_to {
        println!("  Translate: → {}", get_language_name(target));
    }
    println!("  Format:    {}", format.extension().to_uppercase());
    println!();
}

fn get_language_name(code: &str) -> String {
    LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(c, n)| format!("{} ({})", n, c))
        .unwrap_or_else(|| code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_get_language_name() {
        assert_eq!(get_language_name("en"), "English (en)");
        assert_eq!(get_language_name("ja"), "Japanese (ja)");
        assert_eq!(get_language_name("unknown"), "unknown");
    }

    #[test]
    fn test_derive_output_path() {
        let input = PathBuf::from("/path/to/video.mp4");
        
        let srt = derive_output_path(&input, &OutputFormat::Srt);
        assert_eq!(srt, PathBuf::from("/path/to/video.srt"));
        
        let vtt = derive_output_path(&input, &OutputFormat::Vtt);
        assert_eq!(vtt, PathBuf::from("/path/to/video.vtt"));
    }
}
