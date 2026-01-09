# autosub

> Automatic subtitle generation using AI, written in Rust.

A blazingly fast CLI tool for generating subtitles from video and audio files using:
- **OpenAI Whisper API** — Fast, accurate transcription
- **Google Gemini Audio** — Transcription, translation, speaker diarization

## Installation

```bash
# Build from source
cargo build --release

# The binary will be at ./target/release/autosub
```

## Setup

Set your API keys as environment variables:

```bash
# For OpenAI Whisper
export OPENAI_API_KEY="sk-..."

# For Google Gemini
export GEMINI_API_KEY="..."
```

## Usage

```bash
# Basic usage with Whisper (default)
autosub video.mp4 -o subtitles.srt

# Use Google Gemini
autosub video.mp4 -o subtitles.srt --provider gemini

# Output WebVTT format
autosub video.mp4 -o subtitles.vtt --format vtt

# Specify source language (Japanese)
autosub anime.mkv -o subs.srt --language ja

# Enable verbose logging
autosub video.mp4 -o subs.srt -v
```

## CLI Options

```
autosub <INPUT> [OPTIONS]

Arguments:
  <INPUT>  Input video/audio file

Options:
  -o, --output <FILE>       Output subtitle file
  -f, --format <FORMAT>     Output format: srt, vtt, json [default: srt]
  -p, --provider <NAME>     Transcription: gemini, whisper [default: whisper]
  -l, --language <CODE>     Source language [default: en]
      --translate <CODE>    Translate to language (optional)
  -c, --concurrency <N>     Concurrent API requests [default: 4]
  -v, --verbose             Enable verbose logging
  -h, --help                Print help
  -V, --version             Print version
```

## Supported Formats

**Input:** MP4, MKV, AVI, MOV, MP3, WAV, and other FFmpeg-supported formats

**Output:**
- **SRT** — Standard SubRip format (most compatible)
- **VTT** — WebVTT format (for web use)
- **JSON** — Machine-readable with timestamps

## Requirements

- [FFmpeg](https://ffmpeg.org/) must be installed and available in PATH
- API key for your chosen provider

## License

MIT
