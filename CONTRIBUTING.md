# Contributing to autosub

Thanks for your interest in contributing! This document provides guidelines and instructions.

## Development Setup

### Prerequisites

- Rust 1.70+ (`rustup update stable`)
- FFmpeg installed and in PATH
- API keys for testing (optional, tests can run without them)

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/autosub.git
cd autosub

# Build debug
cargo build

# Build release
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_srt_formatter
```

### Linting

```bash
# Run clippy (must pass with no warnings)
cargo clippy

# Check formatting
cargo fmt --check

# Format code
cargo fmt
```

## Code Style

- Follow Rust idioms and conventions
- Use `rustfmt` for consistent formatting
- No clippy warnings in PRs
- Write tests for new functionality
- Use meaningful commit messages

## Project Structure

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library exports
├── config.rs         # Configuration handling
├── error.rs          # Error types
├── pipeline.rs       # Main orchestration
├── audio/            # Audio processing
│   ├── extract.rs    # FFmpeg extraction
│   ├── vad.rs        # Voice activity detection
│   └── chunk.rs      # Audio chunking
├── transcribe/       # Transcription providers
│   ├── whisper.rs    # OpenAI Whisper
│   ├── gemini.rs     # Google Gemini
│   └── orchestrator.rs
├── subtitle/         # Output formatting
│   ├── srt.rs
│   ├── vtt.rs
│   └── json.rs
└── translate/        # Translation (future)
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Ensure tests pass (`cargo test`)
5. Ensure no clippy warnings (`cargo clippy`)
6. Commit with a clear message
7. Push to your fork
8. Open a Pull Request

## Commit Message Format

Use conventional commits:

```
feat: add speaker diarization support
fix: handle empty audio files gracefully
docs: update API key setup instructions
refactor: simplify chunk processing logic
test: add integration tests for VTT output
chore: update dependencies
```

## Adding a New Transcription Provider

1. Create `src/transcribe/newprovider.rs`
2. Implement the `Transcriber` trait
3. Add to `create_transcriber()` factory in `src/transcribe/mod.rs`
4. Add provider variant to `config.rs`
5. Update CLI argument validation in `main.rs`
6. Add tests
7. Update documentation

## Testing with Real APIs

For testing with real APIs, set environment variables:

```bash
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="..."
```

Integration tests that require API keys are marked and will skip gracefully if keys aren't available.

## Questions?

Open an issue for questions, bugs, or feature requests.
