# Autosub-RS Development Changelog

> Detailed task breakdown for building autosub in Rust.
> Check off items as completed. Update dates as you progress.

---

## Phase 1: Foundation

**Target: Week 1**
**Status:** 🟢 Complete

### 1.1 Project Initialization

- [x] **1.1.1** Run `cargo new autosub --bin`
- [x] **1.1.2** Create directory structure:
  ```
  src/
  ├── main.rs
  ├── lib.rs
  ├── config.rs
  ├── error.rs
  ├── audio/
  ├── transcribe/
  ├── translate/
  └── subtitle/
  ```
- [x] **1.1.3** Create empty `mod.rs` files in each subdirectory
- [x] **1.1.4** Setup `.gitignore` for Rust project
- [x] **1.1.5** Create initial `README.md`

### 1.2 Dependencies Setup (Cargo.toml)

- [x] **1.2.1** Add async runtime:
  ```toml
  tokio = { version = "1", features = ["full"] }
  ```
- [x] **1.2.2** Add CLI framework:
  ```toml
  clap = { version = "4", features = ["derive"] }
  ```
- [x] **1.2.3** Add HTTP client:
  ```toml
  reqwest = { version = "0.12", features = ["json", "multipart", "stream"] }
  ```
- [x] **1.2.4** Add serialization:
  ```toml
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  ```
- [x] **1.2.5** Add audio processing:
  ```toml
  hound = "3.5"
  ```
- [x] **1.2.6** Add error handling:
  ```toml
  anyhow = "1"
  thiserror = "2"
  ```
- [x] **1.2.7** Add logging:
  ```toml
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  ```
- [x] **1.2.8** Add progress bars:
  ```toml
  indicatif = "0.17"
  ```
- [x] **1.2.9** Add config file support:
  ```toml
  toml = "0.8"
  dirs = "5"
  ```
- [x] **1.2.10** Add dev dependencies:
  ```toml
  [dev-dependencies]
  tokio-test = "0.4"
  wiremock = "0.6"
  ```
- [x] **1.2.11** Run `cargo build` to verify dependencies resolve

### 1.3 Error Handling Module (`src/error.rs`)

- [x] **1.3.1** Define custom error enum with `thiserror`:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum AutosubError {
      #[error("Audio extraction failed: {0}")]
      AudioExtraction(String),
      #[error("Transcription failed: {0}")]
      Transcription(String),
      #[error("API error: {0}")]
      Api(String),
      #[error("File not found: {0}")]
      FileNotFound(String),
      #[error("Invalid configuration: {0}")]
      Config(String),
      #[error("IO error: {0}")]
      Io(#[from] std::io::Error),
  }
  ```
- [x] **1.3.2** Create type alias: `pub type Result<T> = std::result::Result<T, AutosubError>;`
- [x] **1.3.3** Export from `lib.rs`

### 1.4 Configuration Module (`src/config.rs`)

- [x] **1.4.1** Define `Config` struct:
  ```rust
  pub struct Config {
      pub openai_api_key: Option<String>,
      pub gemini_api_key: Option<String>,
      pub default_provider: Provider,
      pub default_format: OutputFormat,
      pub concurrency: usize,
  }
  ```
- [x] **1.4.2** Define `Provider` enum: `Whisper`, `Gemini`
- [x] **1.4.3** Define `OutputFormat` enum: `Srt`, `Vtt`, `Json`
- [x] **1.4.4** Implement `Config::load()`:
  - [x] Read from environment variables first
  - [x] Fall back to config file `~/.config/autosub/config.toml`
  - [x] Apply defaults for missing values
- [x] **1.4.5** Implement `Config::validate()`:
  - [x] Check API key exists for selected provider
  - [x] Validate concurrency > 0
- [x] **1.4.6** Add unit tests for config loading

### 1.5 CLI Interface (`src/main.rs`)

- [x] **1.5.1** Define CLI args struct with clap derive:
  ```rust
  #[derive(Parser)]
  #[command(name = "autosub")]
  #[command(about = "Automatic subtitle generation")]
  struct Cli {
      /// Input video/audio file
      input: PathBuf,
      
      /// Output subtitle file
      #[arg(short, long)]
      output: Option<PathBuf>,
      
      /// Output format
      #[arg(short, long, default_value = "srt")]
      format: String,
      
      /// Transcription provider
      #[arg(short, long, default_value = "whisper")]
      provider: String,
      
      /// Source language
      #[arg(short, long, default_value = "en")]
      language: String,
      
      /// Translate to language
      #[arg(long)]
      translate: Option<String>,
      
      /// Concurrent API requests
      #[arg(short, long, default_value = "4")]
      concurrency: usize,
      
      /// Verbose output
      #[arg(short, long)]
      verbose: bool,
  }
  ```
- [x] **1.5.2** Implement argument validation:
  - [x] Check input file exists
  - [x] Validate format is one of: srt, vtt, json
  - [x] Validate provider is one of: whisper, gemini
  - [ ] Validate language codes (deferred to Phase 5)
- [x] **1.5.3** Setup tracing subscriber based on `--verbose`
- [x] **1.5.4** Derive output path from input if not specified (e.g., `video.mp4` → `video.srt`)
- [x] **1.5.5** Create async main with tokio:
  ```rust
  #[tokio::main]
  async fn main() -> anyhow::Result<()> {
      let cli = Cli::parse();
      // ...
  }
  ```
- [x] **1.5.6** Test CLI parsing with various argument combinations

### 1.6 Logging Setup

- [x] **1.6.1** Create `init_logging(verbose: bool)` function
- [x] **1.6.2** Configure tracing-subscriber with:
  - [x] RUST_LOG env var support
  - [ ] JSON format option for structured logs (deferred)
  - [x] Timestamp formatting
- [x] **1.6.3** Add colored output for terminal
- [x] **1.6.4** Test logging at different levels

---

## Phase 2: Audio Processing

**Target: Week 2**
**Status:** 🟢 Complete

### 2.1 Audio Module Structure (`src/audio/mod.rs`)

- [x] **2.1.1** Create module exports:
  ```rust
  pub mod extract;
  pub mod vad;
  pub mod chunk;
  
  pub use extract::*;
  pub use vad::*;
  pub use chunk::*;
  ```
- [x] **2.1.2** Define common types:
  ```rust
  pub struct AudioMetadata {
      pub duration: Duration,
      pub sample_rate: u32,
      pub channels: u16,
  }
  
  pub struct SpeechRegion {
      pub start: Duration,
      pub end: Duration,
  }
  
  pub struct AudioChunk {
      pub region: SpeechRegion,
      pub path: PathBuf,
      pub index: usize,
  }
  ```

### 2.2 FFmpeg Audio Extraction (`src/audio/extract.rs`)

- [x] **2.2.1** Check ffmpeg is installed:
  ```rust
  pub fn check_ffmpeg() -> Result<()> {
      Command::new("ffmpeg").arg("-version").output()?;
      Ok(())
  }
  ```
- [x] **2.2.2** Implement `extract_audio()`:
  ```rust
  pub async fn extract_audio(input: &Path, output: &Path) -> Result<AudioMetadata>
  ```
- [x] **2.2.3** FFmpeg command construction:
  ```
  ffmpeg -i input.mp4 -vn -acodec pcm_s16le -ar 16000 -ac 1 output.wav
  ```
  - `-vn`: No video
  - `-acodec pcm_s16le`: 16-bit PCM
  - `-ar 16000`: 16kHz sample rate
  - `-ac 1`: Mono
- [x] **2.2.4** Handle ffmpeg stderr for progress/errors
- [x] **2.2.5** Parse output duration from ffmpeg
- [x] **2.2.6** Create temp directory for extracted audio
- [x] **2.2.7** Add tests with sample video files
- [x] **2.2.8** Handle different input formats: MP4, MKV, AVI, MOV, MP3, WAV

### 2.3 Voice Activity Detection (`src/audio/vad.rs`)

- [x] **2.3.1** Implement WAV file reading with `hound`:
  ```rust
  pub fn read_wav(path: &Path) -> Result<(Vec<i16>, WavSpec)>
  ```
- [x] **2.3.2** Implement RMS energy calculation:
  ```rust
  fn calculate_rms(samples: &[i16]) -> f64 {
      let sum: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
      (sum / samples.len() as f64).sqrt()
  }
  ```
- [x] **2.3.3** Implement energy-based VAD:
  ```rust
  pub fn detect_speech_regions(
      samples: &[i16],
      sample_rate: u32,
      config: VadConfig,
  ) -> Vec<SpeechRegion>
  ```
- [x] **2.3.4** Define `VadConfig`:
  ```rust
  pub struct VadConfig {
      pub frame_duration_ms: u32,      // Default: 30ms
      pub min_speech_duration_ms: u32, // Default: 500ms
      pub max_speech_duration_ms: u32, // Default: 6000ms
      pub energy_threshold: f64,       // Default: calculated from percentile
      pub padding_ms: u32,             // Default: 200ms
  }
  ```
- [x] **2.3.5** Implement percentile-based threshold calculation:
  - Calculate RMS for all frames
  - Use 20th percentile as silence threshold
- [x] **2.3.6** Implement region merging:
  - Merge regions closer than `padding_ms`
  - Split regions longer than `max_speech_duration_ms`
- [x] **2.3.7** Add debug logging for VAD decisions
- [x] **2.3.8** Add unit tests with sample audio

### 2.4 Audio Chunking (`src/audio/chunk.rs`)

- [x] **2.4.1** Define chunk configuration:
  ```rust
  pub struct ChunkConfig {
      pub max_duration: Duration,    // API limit
      pub max_size_bytes: usize,     // 25MB for Whisper
      pub output_format: AudioFormat, // WAV, FLAC, MP3
  }
  ```
- [x] **2.4.2** Implement `chunk_audio()`:
  ```rust
  pub async fn chunk_audio(
      source: &Path,
      regions: &[SpeechRegion],
      config: ChunkConfig,
  ) -> Result<Vec<AudioChunk>>
  ```
- [x] **2.4.3** Extract each region using ffmpeg:
  ```
  ffmpeg -i input.wav -ss START -to END -c copy chunk_N.wav
  ```
- [x] **2.4.4** Handle Whisper API limits:
  - Split chunks > 25MB
  - Track chunk boundaries for reassembly
- [x] **2.4.5** Handle Gemini API limits:
  - Inline for < 20MB
  - Files API for >= 20MB
- [x] **2.4.6** Implement parallel chunk extraction with tokio
- [x] **2.4.7** Add progress bar for chunking
- [x] **2.4.8** Cleanup temporary files on error/completion
- [x] **2.4.9** Add tests for edge cases:
  - Very short audio
  - Very long audio
  - Silent audio

---

## Phase 3: Transcription Providers

**Target: Week 3**
**Status:** 🟢 Complete

### 3.1 Transcriber Trait (`src/transcribe/mod.rs`)

- [x] **3.1.1** Define core trait:
  ```rust
  #[async_trait]
  pub trait Transcriber: Send + Sync {
      async fn transcribe(&self, chunk: &AudioChunk) -> Result<TranscriptSegment>;
      fn name(&self) -> &'static str;
      fn max_file_size(&self) -> usize;
      fn supported_formats(&self) -> &[&str];
  }
  ```
- [x] **3.1.2** Define `TranscriptSegment`:
  ```rust
  pub struct TranscriptSegment {
      pub text: String,
      pub start: Duration,
      pub end: Duration,
      pub words: Option<Vec<WordTimestamp>>,
      pub confidence: Option<f64>,
      pub speaker: Option<String>,
  }
  
  pub struct WordTimestamp {
      pub word: String,
      pub start: Duration,
      pub end: Duration,
  }
  ```
- [x] **3.1.3** Define `TranscriptionResult`:
  ```rust
  pub struct TranscriptionResult {
      pub segments: Vec<TranscriptSegment>,
      pub language: String,
      pub duration: Duration,
  }
  ```
- [x] **3.1.4** Create factory function:
  ```rust
  pub fn create_transcriber(provider: Provider, config: &Config) -> Box<dyn Transcriber>
  ```

### 3.2 OpenAI Whisper Provider (`src/transcribe/whisper.rs`)

#### 3.2.1 Basic Setup
- [x] **3.2.1.1** Define `WhisperClient` struct:
  ```rust
  pub struct WhisperClient {
      client: reqwest::Client,
      api_key: String,
      model: WhisperModel,
      base_url: String,
  }
  ```
- [x] **3.2.1.2** Define `WhisperModel` enum:
  ```rust
  pub enum WhisperModel {
      Whisper1,           // whisper-1
      Gpt4oTranscribe,    // gpt-4o-transcribe
      Gpt4oMiniTranscribe, // gpt-4o-mini-transcribe
      Gpt4oDiarize,       // gpt-4o-transcribe-diarize
  }
  ```
- [x] **3.2.1.3** Implement `WhisperClient::new()` with config validation

#### 3.2.2 API Request Building
- [x] **3.2.2.1** Build multipart form data:
  ```rust
  async fn build_request(&self, chunk: &AudioChunk) -> Result<reqwest::multipart::Form>
  ```
- [x] **3.2.2.2** Add file field with correct MIME type
- [x] **3.2.2.3** Add model field
- [x] **3.2.2.4** Add response_format field (`verbose_json` for timestamps)
- [x] **3.2.2.5** Add language field (optional)
- [x] **3.2.2.6** Add prompt field for vocabulary hints (max 224 tokens)
- [x] **3.2.2.7** Add timestamp_granularities field (`word`, `segment`)

#### 3.2.3 API Response Parsing
- [x] **3.2.3.1** Define response structs:
  ```rust
  #[derive(Deserialize)]
  struct WhisperResponse {
      text: String,
      segments: Option<Vec<WhisperSegment>>,
      words: Option<Vec<WhisperWord>>,
      language: String,
      duration: f64,
  }
  
  #[derive(Deserialize)]
  struct WhisperSegment {
      start: f64,
      end: f64,
      text: String,
  }
  
  #[derive(Deserialize)]
  struct WhisperWord {
      word: String,
      start: f64,
      end: f64,
  }
  ```
- [x] **3.2.3.2** Parse `verbose_json` response
- [x] **3.2.3.3** Convert to `TranscriptSegment`
- [x] **3.2.3.4** Handle API error responses:
  ```rust
  #[derive(Deserialize)]
  struct ApiError {
      error: ApiErrorDetail,
  }
  
  #[derive(Deserialize)]
  struct ApiErrorDetail {
      message: String,
      r#type: String,
      code: Option<String>,
  }
  ```

#### 3.2.4 Retry Logic
- [x] **3.2.4.1** Implement exponential backoff:
  ```rust
  async fn with_retry<T, F, Fut>(f: F, max_retries: u32) -> Result<T>
  where
      F: Fn() -> Fut,
      Fut: Future<Output = Result<T>>,
  ```
- [x] **3.2.4.2** Retry on 429 (rate limit) with Retry-After header
- [x] **3.2.4.3** Retry on 500, 502, 503, 504 (server errors)
- [x] **3.2.4.4** Do not retry on 400, 401, 403 (client errors)
- [x] **3.2.4.5** Add jitter to backoff delays

#### 3.2.5 Implement Transcriber Trait
- [x] **3.2.5.1** Implement `transcribe()` method
- [x] **3.2.5.2** Implement `name()` → "OpenAI Whisper"
- [x] **3.2.5.3** Implement `max_file_size()` → 25MB
- [x] **3.2.5.4** Implement `supported_formats()`

#### 3.2.6 Testing
- [x] **3.2.6.1** Add unit tests with mocked responses (wiremock)
- [x] **3.2.6.2** Add integration test with real API (optional, requires key)
- [x] **3.2.6.3** Test error handling for various failure modes

### 3.3 Google Gemini Provider (`src/transcribe/gemini.rs`)

#### 3.3.1 Basic Setup
- [x] **3.3.1.1** Define `GeminiClient` struct:
  ```rust
  pub struct GeminiClient {
      client: reqwest::Client,
      api_key: String,
      model: String,  // gemini-2.0-flash
  }
  ```
- [x] **3.3.1.2** Define API endpoints:
  ```rust
  const GENERATE_CONTENT_URL: &str = 
      "https://generativelanguage.googleapis.com/v1beta/models";
  const FILES_UPLOAD_URL: &str = 
      "https://generativelanguage.googleapis.com/upload/v1beta/files";
  ```
- [x] **3.3.1.3** Implement `GeminiClient::new()` with config validation

#### 3.3.2 Files API (for large files >20MB)
- [x] **3.3.2.1** Implement `upload_file()`:
  ```rust
  async fn upload_file(&self, path: &Path) -> Result<String>  // Returns file URI
  ```
- [x] **3.3.2.2** Build upload request with resumable upload
- [x] **3.3.2.3** Parse upload response for file URI
- [x] **3.3.2.4** Implement `get_file_status()` to check processing
- [x] **3.3.2.5** Implement `delete_file()` for cleanup

#### 3.3.3 Generate Content API
- [x] **3.3.3.1** Build request for inline audio (< 20MB):
  ```rust
  async fn transcribe_inline(&self, chunk: &AudioChunk) -> Result<TranscriptSegment>
  ```
- [x] **3.3.3.2** Build request for uploaded file:
  ```rust
  async fn transcribe_file(&self, file_uri: &str) -> Result<TranscriptSegment>
  ```
- [x] **3.3.3.3** Construct prompt for transcription:
  ```
  Transcribe this audio with timestamps in the format:
  [MM:SS] Speaker: Text
  
  Include speaker identification if multiple speakers detected.
  ```
- [x] **3.3.3.4** Define request body structure:
  ```rust
  #[derive(Serialize)]
  struct GenerateContentRequest {
      contents: Vec<Content>,
      generation_config: Option<GenerationConfig>,
  }
  
  #[derive(Serialize)]
  struct Content {
      parts: Vec<Part>,
  }
  
  #[derive(Serialize)]
  #[serde(untagged)]
  enum Part {
      Text { text: String },
      InlineData { inline_data: InlineData },
      FileData { file_data: FileData },
  }
  ```

#### 3.3.4 Response Parsing
- [x] **3.3.4.1** Parse Gemini response:
  ```rust
  #[derive(Deserialize)]
  struct GenerateContentResponse {
      candidates: Vec<Candidate>,
  }
  
  #[derive(Deserialize)]
  struct Candidate {
      content: CandidateContent,
  }
  ```
- [x] **3.3.4.2** Extract text from response
- [x] **3.3.4.3** Parse timestamps from text (regex: `\[(\d{2}):(\d{2})\]`)
- [x] **3.3.4.4** Parse speaker labels if present
- [x] **3.3.4.5** Convert to `TranscriptSegment`

#### 3.3.5 Special Features
- [x] **3.3.5.1** Implement speaker diarization prompt:
  ```
  Identify and label each speaker as Speaker 1, Speaker 2, etc.
  ```
- [x] **3.3.5.2** Implement emotion detection prompt:
  ```
  Note the emotional tone (happy, sad, angry, neutral) for each segment.
  ```
- [x] **3.3.5.3** Implement translation prompt:
  ```
  Transcribe and translate to {target_language}.
  ```

#### 3.3.6 Retry Logic
- [x] **3.3.6.1** Implement exponential backoff (same as Whisper)
- [x] **3.3.6.2** Handle Gemini-specific error codes
- [x] **3.3.6.3** Handle quota exceeded errors

#### 3.3.7 Implement Transcriber Trait
- [x] **3.3.7.1** Implement `transcribe()` method
- [x] **3.3.7.2** Implement `name()` → "Google Gemini"
- [x] **3.3.7.3** Implement `max_file_size()` → determine based on inline vs file
- [x] **3.3.7.4** Implement `supported_formats()`

#### 3.3.8 Testing
- [x] **3.3.8.1** Add unit tests with mocked responses
- [x] **3.3.8.2** Add integration test (optional)
- [x] **3.3.8.3** Test file upload/delete lifecycle

### 3.4 Concurrent Processing

- [x] **3.4.1** Create `TranscriptionOrchestrator`:
  ```rust
  pub struct TranscriptionOrchestrator {
      transcriber: Box<dyn Transcriber>,
      concurrency: usize,
  }
  ```
- [x] **3.4.2** Implement concurrent chunk processing:
  ```rust
  pub async fn process_chunks(
      &self,
      chunks: Vec<AudioChunk>,
  ) -> Result<Vec<TranscriptSegment>>
  ```
- [x] **3.4.3** Use `tokio::sync::Semaphore` for concurrency control
- [x] **3.4.4** Use `futures::stream::FuturesUnordered` for parallel execution
- [x] **3.4.5** Maintain chunk order in results
- [x] **3.4.6** Add progress bar with indicatif:
  ```rust
  let pb = ProgressBar::new(chunks.len() as u64);
  pb.set_style(ProgressStyle::default_bar()
      .template("{spinner} [{bar:40}] {pos}/{len} chunks")?);
  ```
- [x] **3.4.7** Handle partial failures:
  - Continue on individual chunk failure
  - Collect errors for reporting
  - Option to retry failed chunks
- [x] **3.4.8** Add timing metrics (total time, per-chunk avg)

---

## Phase 4: Subtitle Generation

**Target: Week 4**
**Status:** 🟢 Complete

### 4.1 Subtitle Module Structure (`src/subtitle/mod.rs`)

- [x] **4.1.1** Define `SubtitleEntry`:
  ```rust
  pub struct SubtitleEntry {
      pub index: usize,
      pub start: Duration,
      pub end: Duration,
      pub text: String,
      pub speaker: Option<String>,
  }
  ```
- [x] **4.1.2** Define `SubtitleFormatter` trait:
  ```rust
  pub trait SubtitleFormatter {
      fn format(&self, entries: &[SubtitleEntry]) -> String;
      fn extension(&self) -> &'static str;
  }
  ```
- [x] **4.1.3** Create factory function:
  ```rust
  pub fn create_formatter(format: OutputFormat) -> Box<dyn SubtitleFormatter>
  ```

### 4.2 SRT Formatter (`src/subtitle/srt.rs`)

- [x] **4.2.1** Implement SRT format:
  ```
  1
  00:00:01,500 --> 00:00:04,000
  Hello, welcome to this video.
  
  2
  00:00:04,500 --> 00:00:07,000
  Today we're going to learn...
  ```
- [x] **4.2.2** Implement timestamp formatting: `HH:MM:SS,mmm`
- [x] **4.2.3** Handle multi-line text (preserve line breaks)
- [x] **4.2.4** Escape special characters if needed
- [x] **4.2.5** Add blank line between entries
- [x] **4.2.6** Implement `SubtitleFormatter` trait
- [x] **4.2.7** Add unit tests

### 4.3 VTT Formatter (`src/subtitle/vtt.rs`)

- [x] **4.3.1** Implement WebVTT format:
  ```
  WEBVTT
  
  00:00:01.500 --> 00:00:04.000
  Hello, welcome to this video.
  
  00:00:04.500 --> 00:00:07.000
  Today we're going to learn...
  ```
- [x] **4.3.2** Add `WEBVTT` header
- [x] **4.3.3** Implement timestamp formatting: `HH:MM:SS.mmm`
- [x] **4.3.4** Support optional cue identifiers
- [ ] **4.3.5** Support cue settings (position, alignment) - optional (deferred)
- [x] **4.3.6** Implement `SubtitleFormatter` trait
- [x] **4.3.7** Add unit tests

### 4.4 JSON Formatter (`src/subtitle/json.rs`)

- [x] **4.4.1** Define JSON output structure:
  ```rust
  #[derive(Serialize)]
  struct JsonOutput {
      metadata: JsonMetadata,
      subtitles: Vec<JsonSubtitle>,
  }
  
  #[derive(Serialize)]
  struct JsonMetadata {
      source_file: String,
      duration: f64,
      language: String,
      provider: String,
      generated_at: String,
  }
  
  #[derive(Serialize)]
  struct JsonSubtitle {
      index: usize,
      start: f64,
      end: f64,
      start_formatted: String,
      end_formatted: String,
      text: String,
      speaker: Option<String>,
      words: Option<Vec<JsonWord>>,
  }
  ```
- [x] **4.4.2** Implement pretty-printed JSON output
- [ ] **4.4.3** Include word-level timestamps if available (deferred)
- [x] **4.4.4** Implement `SubtitleFormatter` trait
- [x] **4.4.5** Add unit tests

### 4.5 Post-Processing

- [x] **4.5.1** Implement segment merging:
  - Merge segments < 1 second apart
  - Configurable threshold
- [x] **4.5.2** Implement line splitting:
  - Split text > 42 characters per line
  - Split at sentence boundaries when possible
- [x] **4.5.3** Implement timing adjustments:
  - Add minimum gap between subtitles (100ms)
  - Extend short subtitles to minimum duration (1s)
- [x] **4.5.4** Remove filler words option (um, uh, etc.)
- [ ] **4.5.5** Add punctuation if missing (deferred)

### 4.6 Transcript to Subtitle Conversion

- [x] **4.6.1** Implement `convert_to_subtitles()`:
  ```rust
  pub fn convert_to_subtitles(
      segments: Vec<TranscriptSegment>,
      config: PostProcessConfig,
  ) -> Vec<SubtitleEntry>
  ```
- [x] **4.6.2** Handle speaker labels (prefix text or separate line)
- [x] **4.6.3** Number entries sequentially
- [x] **4.6.4** Validate no overlapping timestamps

---

## Phase 5: Translation (Optional)

**Target: Week 5**
**Status:** 🟢 Complete

### 5.1 Translator Trait (`src/translate/mod.rs`)

- [x] **5.1.1** Define trait:
  ```rust
  #[async_trait]
  pub trait Translator: Send + Sync {
      async fn translate(&self, text: &str, target_lang: &str) -> Result<String>;
      async fn translate_batch(&self, texts: &[&str], target_lang: &str) -> Result<Vec<String>>;
      fn supported_languages(&self) -> &[&str];
  }
  ```

### 5.2 Google Translate (`src/translate/gemini.rs`)

- [x] **5.2.1** Implement using Gemini API (already have key)
- [x] **5.2.2** Build translation prompt
- [x] **5.2.3** Batch translations for efficiency
- [x] **5.2.4** Preserve formatting and line breaks
- [x] **5.2.5** Handle translation errors gracefully

### 5.3 Integration

- [x] **5.3.1** Add `--translate` flag handling in main
- [x] **5.3.2** Translate after transcription, before formatting
- [x] **5.3.3** Update metadata with target language

---

## Phase 6: Integration & Polish

**Target: Week 6**
**Status:** 🟢 Complete

### 6.1 Pipeline Orchestration (`src/pipeline.rs`)

- [x] **6.1.1** Create main pipeline function:
  ```rust
  pub async fn generate_subtitles(
      input: &Path,
      output: &Path,
      config: &Config,
      pipeline_config: PipelineConfig,
  ) -> Result<PipelineResult>
  ```
- [x] **6.1.2** Implement pipeline stages with progress reporting
- [x] **6.1.3** Handle cancellation (Ctrl+C)
- [x] **6.1.4** Cleanup temp files on success or failure
- [x] **6.1.5** Return summary statistics

### 6.2 Error Handling & User Experience

- [x] **6.2.1** Add helpful error messages:
  - FFmpeg not found → installation instructions
  - Invalid API key → how to set up
  - Rate limited → retry suggestion
- [x] **6.2.2** Add `--dry-run` flag to validate without processing
- [x] **6.2.3** Add `--force` flag to overwrite existing output
- [x] **6.2.4** Colored terminal output for status messages (via tracing)

### 6.3 Testing

- [x] **6.3.1** Add integration tests with sample files
- [x] **6.3.2** Add end-to-end test (requires API keys) — ✅ Tested with Gemini and Whisper
- [x] **6.3.3** Test with various input formats — ✅ SRT, VTT, JSON all working
- [x] **6.3.4** Test with different languages — ✅ Japanese (ja) tested successfully
- [x] **6.3.5** Test edge cases:
  - Empty audio
  - Very short audio (<1s)
  - Very long audio (>1hr)
  - Audio with no speech
  - Corrupted files

### 6.4 Documentation

- [x] **6.4.1** Complete README.md:
  - Installation instructions
  - Quick start guide
  - Configuration options
  - Examples
- [x] **6.4.2** Add `--help` documentation for all flags (via clap derive)
- [x] **6.4.3** Add CONTRIBUTING.md
- [x] **6.4.4** Add LICENSE (MIT or Apache-2.0)
- [ ] **6.4.5** Add rustdoc comments to public API (deferred)

### 6.5 Distribution

- [x] **6.5.1** Setup GitHub Actions CI:
  - Build on Linux, macOS, Windows
  - Run tests
  - Clippy lints
  - Format check
- [x] **6.5.2** Setup release workflow:
  - Build release binaries
  - Create GitHub release
  - Upload artifacts
- [ ] **6.5.3** Create install script
- [ ] **6.5.4** Optional: Homebrew formula
- [ ] **6.5.5** Optional: Publish to crates.io

---

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| TBD  | 0.1.0   | Initial release with Whisper & Gemini support |

---

## Notes & Learnings

_Document any issues, API quirks, or learnings here as you implement._

### API Quirks

- **Whisper**: `verbose_json` returns different structure than `json`
- **Gemini**: Timestamps in response are text format `[MM:SS]`, need parsing

### Performance Notes

- _Add performance observations here_

### Issues Encountered

- _Document bugs and solutions here_
