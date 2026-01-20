# autosub

> Automatic subtitle generation using AI, written in Rust.

A blazingly fast CLI tool for generating subtitles from video and audio files using **Google Gemini Audio** for transcription, translation, and speaker diarization.

## Features

- 🚀 Fast concurrent processing with configurable parallelism
- 🎯 Multiple output formats: SRT, VTT, JSON
- 🔊 Smart voice activity detection (VAD)
- 📝 Post-processing: segment merging, line splitting, filler word removal
- ⏸️ Graceful Ctrl+C handling with cleanup
- 🔍 Dry-run mode for validation

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/autosub.git
cd autosub

# Build release binary
cargo build --release

# The binary will be at ./target/release/autosub
# Optionally, copy to your PATH:
cp target/release/autosub /usr/local/bin/
```

### Requirements

- **Rust 1.70+** (for building)
- **[FFmpeg](https://ffmpeg.org/)** must be installed and available in PATH
- Google Gemini API key

## Setup

### API Key

Set your Gemini API key as an environment variable:

```bash
export GEMINI_API_KEY="..."
```

You can get an API key from: https://aistudio.google.com/apikey

### Optional: Config File

Create `~/.config/autosub/config.toml` for persistent settings:

```toml
default_format = "srt"        # or "vtt", "json"
concurrency = 4
```

## Usage

### Basic Examples

```bash
# Basic usage
autosub video.mp4 -o subtitles.srt

# Output WebVTT format
autosub video.mp4 -o subtitles.vtt --format vtt

# Specify source language (Japanese)
autosub anime.mkv -o subs.srt --language ja

# High concurrency for faster processing
autosub long-video.mp4 -o subs.srt --concurrency 8

# Enable verbose logging
autosub video.mp4 -o subs.srt -v
```

### Validation and Safety

```bash
# Validate without processing (dry-run)
autosub video.mp4 --dry-run

# Force overwrite existing output file
autosub video.mp4 -o existing.srt --force
```

## CLI Reference

```
autosub <INPUT> [OPTIONS]

Arguments:
  <INPUT>  Input video/audio file

Options:
  -o, --output <FILE>       Output subtitle file (auto-derived if not specified)
  -f, --format <FORMAT>     Output format: srt, vtt, json [default: srt]
  -l, --language <CODE>     Source language code [default: en]
      --translate <CODE>    Translate to language (optional)
  -c, --concurrency <N>     Concurrent API requests [default: 4]
      --dry-run             Validate inputs without processing
      --force               Overwrite existing output file
  -v, --verbose             Enable verbose logging
  -h, --help                Print help
  -V, --version             Print version
```

## Supported Formats

### Input Formats

Any format supported by FFmpeg, including:
- **Video:** MP4, MKV, AVI, MOV, WebM
- **Audio:** MP3, WAV, FLAC, AAC, OGG, M4A

### Output Formats

| Format | Extension | Best For |
|--------|-----------|----------|
| **SRT** | `.srt` | Maximum compatibility (VLC, YouTube, etc.) |
| **VTT** | `.vtt` | Web use (HTML5 video) |
| **JSON** | `.json` | Programmatic access, further processing |

## How It Works

1. **Audio Extraction** — Extracts audio from video using FFmpeg
2. **Voice Activity Detection** — Identifies speech regions
3. **Chunking** — Splits audio for API limits (20MB for Gemini)
4. **Transcription** — Sends chunks to Gemini API in parallel
5. **Post-Processing** — Merges segments, splits long lines, adjusts timing
6. **Formatting** — Outputs in chosen subtitle format

## Troubleshooting

### FFmpeg not found

Install FFmpeg:
```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# Windows (with Chocolatey)
choco install ffmpeg
```

### API Key errors

Ensure your API key is exported in the current shell:
```bash
echo $GEMINI_API_KEY  # Should show your key
export GEMINI_API_KEY="..."  # Set if empty
```

### Rate limiting

Reduce concurrency if you hit rate limits:
```bash
autosub video.mp4 -o subs.srt --concurrency 2
```

## API Pricing

| Provider | Pricing |
|----------|---------|
| Google Gemini | Based on token usage ([details](https://ai.google.dev/gemini-api/docs/pricing)) |

## License

MIT

## Credits

Inspired by [agermanidis/autosub](https://github.com/agermanidis/autosub).
