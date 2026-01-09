# Autosub-RS Implementation Plan

> A blazingly fast CLI tool for automatic subtitle generation, written in Rust.

## Overview

Rewrite of [agermanidis/autosub](https://github.com/agermanidis/autosub) in Rust with modern API support:
- **Google Gemini Audio Understanding** — transcription, translation, speaker diarization
- **OpenAI Whisper API** — fast, accurate transcription
- **Future: Local Whisper (whisper.cpp)** — offline processing

Users bring their own API keys (BYOK model).

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI (clap)                              │
│   autosub input.mp4 -o output.srt --provider whisper            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Pipeline Orchestrator                        │
│              (manages workflow & error handling)                │
└─────────────────────────────────────────────────────────────────┘
        │                     │                      │
        ▼                     ▼                      ▼
┌───────────────┐   ┌─────────────────┐   ┌──────────────────┐
│    Audio      │   │   Transcription │   │    Subtitle      │
│   Processor   │   │    Provider     │   │    Formatter     │
│               │   │   (trait-based) │   │                  │
│ • Extract     │   │ • Google STT    │   │ • SRT            │
│ • VAD         │   │ • Whisper API   │   │ • VTT            │
│ • Chunk       │   │ • Local (future)│   │ • JSON           │
└───────────────┘   └─────────────────┘   └──────────────────┘
```

---

## Project Structure

```
autosub/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, CLI parsing
│   ├── lib.rs                  # Library exports
│   ├── config.rs               # Configuration & API keys
│   ├── error.rs                # Custom error types
│   │
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── extract.rs          # FFmpeg audio extraction
│   │   ├── vad.rs              # Voice Activity Detection
│   │   └── chunk.rs            # Audio chunking for API
│   │
│   ├── transcribe/
│   │   ├── mod.rs              # Transcriber trait
│   │   ├── gemini.rs           # Google Gemini Audio
│   │   ├── whisper.rs          # OpenAI Whisper API
│   │   └── local.rs            # Local Whisper (future)
│   │
│   ├── translate/
│   │   ├── mod.rs              # Translator trait
│   │   └── google.rs           # Google Translate API
│   │
│   └── subtitle/
│       ├── mod.rs
│       ├── srt.rs              # SRT format
│       ├── vtt.rs              # WebVTT format
│       └── json.rs             # JSON format
│
├── tests/
│   ├── integration/
│   └── fixtures/               # Sample audio files
│
└── README.md
```

---

## Phase 1: Foundation (Week 1)

### 1.1 Project Setup
- [ ] Initialize Cargo project
- [ ] Setup dependencies in `Cargo.toml`
- [ ] Configure error handling with `thiserror` + `anyhow`
- [ ] Setup logging with `tracing`

### 1.2 CLI Interface
- [ ] Define CLI arguments with `clap`
  ```
  autosub <INPUT> [OPTIONS]
  
  Options:
    -o, --output <FILE>       Output subtitle file
    -f, --format <FORMAT>     Output format: srt, vtt, json [default: srt]
    -p, --provider <NAME>     Transcription: gemini, whisper [default: whisper]
    -l, --language <CODE>     Source language [default: en]
    --translate <CODE>        Translate to language (optional)
    -c, --concurrency <N>     Concurrent API requests [default: 4]
    -v, --verbose             Enable verbose logging
  ```

### 1.3 Configuration
- [ ] Environment variable support for API keys
  - `GEMINI_API_KEY`
  - `OPENAI_API_KEY`
- [ ] Optional config file (`~/.config/autosub/config.toml`)
- [ ] Config validation on startup

---

## Phase 2: Audio Processing (Week 2)

### 2.1 Audio Extraction
- [ ] FFmpeg wrapper for audio extraction
- [ ] Convert to WAV (mono, 16kHz)
- [ ] Support common formats: MP4, MKV, AVI, MOV, MP3, WAV

### 2.2 Voice Activity Detection (VAD)
- [ ] Read WAV with `hound`
- [ ] Calculate RMS energy per chunk
- [ ] Detect speech regions (start, end timestamps)
- [ ] Configurable thresholds

### 2.3 Audio Chunking
- [ ] Split audio by speech regions
- [ ] Respect API limits:
  - Google: 60s per request (sync), 480 min (async)
  - Whisper: 25MB file size limit
- [ ] Export chunks as temporary files

---

## Phase 3: Transcription Providers (Week 3)

### 3.1 Provider Trait
```rust
#[async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcribe(&self, audio: &AudioChunk) -> Result<Transcript>;
    fn name(&self) -> &'static str;
    fn max_chunk_duration(&self) -> Duration;
}
```

### 3.2 OpenAI Whisper API
- [ ] Implement `Transcriber` trait
- [ ] Support multiple models: `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`
- [ ] Handle file upload (multipart/form-data, max 25MB)
- [ ] Chunk large files and reassemble transcripts
- [ ] Parse `verbose_json` response with timestamps
- [ ] Support `timestamp_granularities[]` for word/segment timestamps
- [ ] Prompting support for domain-specific vocabulary (224 token limit)
- [ ] Retry logic with exponential backoff
- [ ] Rate limiting
- [ ] Optional: `gpt-4o-transcribe-diarize` for speaker labels

### 3.3 Google Gemini Audio
- [ ] Implement `Transcriber` trait
- [ ] Use Files API for uploads (>20MB) or inline for smaller files
- [ ] Use `generateContent` endpoint with audio MIME type
- [ ] Parse response with timestamps (MM:SS format)
- [ ] Support speaker diarization via prompt
- [ ] Support emotion detection via prompt
- [ ] Retry logic with exponential backoff

### 3.4 Concurrent Processing
- [ ] Process chunks concurrently with `tokio`
- [ ] Configurable concurrency limit
- [ ] Progress bar with `indicatif`
- [ ] Aggregate results in order

---

## Phase 4: Subtitle Generation (Week 4)

### 4.1 Subtitle Formatter Trait
```rust
pub trait SubtitleFormatter {
    fn format(&self, entries: &[SubtitleEntry]) -> String;
    fn extension(&self) -> &'static str;
}
```

### 4.2 Format Implementations
- [ ] **SRT**: Standard SubRip format
- [ ] **VTT**: WebVTT format
- [ ] **JSON**: Machine-readable format

### 4.3 Post-Processing
- [ ] Merge short segments
- [ ] Split long lines
- [ ] Timing adjustments

---

## Phase 5: Translation (Week 5)

### 5.1 Translation Support
- [ ] Google Translate API integration
- [ ] Batch translation for efficiency
- [ ] Preserve timing from transcription

### 5.2 Language Detection
- [ ] Auto-detect source language (optional)
- [ ] Validate language codes

---

## Phase 6: Polish & Release (Week 6)

### 6.1 Testing
- [ ] Unit tests for each module
- [ ] Integration tests with sample files
- [ ] Mock API responses for CI

### 6.2 Documentation
- [ ] README with usage examples
- [ ] API key setup guide
- [ ] Troubleshooting guide

### 6.3 Distribution
- [ ] GitHub releases
- [ ] Pre-built binaries (Linux, macOS, Windows)
- [ ] Homebrew formula (optional)
- [ ] Cargo publish (optional)

---

## Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# CLI
clap = { version = "4", features = ["derive"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "multipart", "stream"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Audio
hound = "3.5"                    # WAV reading

# Error handling
anyhow = "1"
thiserror = "2"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Progress
indicatif = "0.17"

# Config
toml = "0.8"
dirs = "5"

[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"                 # Mock HTTP for tests
```

---

## API Reference

### OpenAI Whisper API
- **Transcription Endpoint**: `POST https://api.openai.com/v1/audio/transcriptions`
- **Translation Endpoint**: `POST https://api.openai.com/v1/audio/translations` (to English only)
- **Models**:
  - `whisper-1` — Original Whisper, supports all output formats
  - `gpt-4o-transcribe` — Higher quality, json/text output only
  - `gpt-4o-mini-transcribe` — Faster, json/text output only
  - `gpt-4o-transcribe-diarize` — Speaker diarization support
- **Max file size**: 25 MB (chunk larger files)
- **Supported input formats**: mp3, mp4, mpeg, mpga, m4a, wav, webm
- **Output formats** (`whisper-1`): json, text, srt, vtt, verbose_json
- **Timestamps**: Word-level and segment-level via `timestamp_granularities[]` (whisper-1 only)
- **Streaming**: Supported for gpt-4o models (not whisper-1)
- **Prompting**: Up to 224 tokens for context/spelling hints
- **Supported languages**: 50+ languages (Afrikaans to Welsh)
- **Pricing**: $0.006 / minute

### Google Gemini Audio Understanding
- **Endpoint**: `POST https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent`
- **Files API**: `POST https://generativelanguage.googleapis.com/upload/v1beta/files` (for >20MB)
- **Max audio length**: 9.5 hours per prompt
- **Token rate**: 32 tokens/second of audio (~1,920 tokens/minute)
- **Supported formats**: WAV, MP3, AIFF, AAC, OGG, FLAC
- **Capabilities**: Transcription, translation, speaker diarization, emotion detection
- **Pricing**: Input tokens based on audio length (see [pricing](https://ai.google.dev/gemini-api/docs/pricing))

---

## CLI Usage Examples

```bash
# Basic usage with Whisper
autosub video.mp4 -o subtitles.srt

# Use Google Gemini Audio
autosub video.mp4 -o subtitles.srt --provider gemini

# Output WebVTT format
autosub video.mp4 -o subtitles.vtt --format vtt

# Transcribe and translate to Spanish
autosub video.mp4 -o subtitles.srt --translate es

# Specify source language (Japanese)
autosub anime.mkv -o subs.srt --language ja

# High concurrency for faster processing
autosub long-video.mp4 -o subs.srt --concurrency 8
```

---

## Environment Variables

```bash
# Required (one of these based on provider)
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="..."        # Get from https://aistudio.google.com/apikey

# Optional
export AUTOSUB_DEFAULT_PROVIDER="whisper"
export AUTOSUB_DEFAULT_FORMAT="srt"
export AUTOSUB_CONCURRENCY="4"
```

---

## Future Enhancements

- [ ] **Local Whisper**: Integrate `whisper-rs` for offline processing
- [ ] **Streaming**: Real-time transcription for live audio
- [ ] **GPU acceleration**: CUDA support for local Whisper
- [ ] **Speaker diarization**: Identify different speakers
- [ ] **Custom vocabulary**: Domain-specific terms
- [ ] **Batch processing**: Process multiple files
- [ ] **Watch mode**: Monitor folder for new files

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Cold start time | < 100ms |
| 1-hour video processing | < 5 min (API-bound) |
| Memory usage | < 100MB |
| Binary size | < 10MB |
| Test coverage | > 80% |

---

## References

- [Original autosub](https://github.com/agermanidis/autosub)
- [OpenAI Whisper API](https://platform.openai.com/docs/guides/speech-to-text)
- [Google Speech-to-Text](https://cloud.google.com/speech-to-text/docs)
- [whisper-rs](https://github.com/tazz4843/whisper-rs)
- [SRT Format Spec](https://www.matroska.org/technical/subtitles.html)
