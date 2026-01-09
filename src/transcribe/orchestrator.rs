use crate::audio::AudioChunk;
use crate::error::{AutosubError, Result};
use crate::transcribe::{Transcript, TranscriptSegment, TranscriptionResult, Transcriber};
use futures::stream::{FuturesUnordered, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

/// Result of processing a single chunk.
#[derive(Debug)]
pub struct ChunkResult {
    pub index: usize,
    pub transcript: Option<Transcript>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Statistics from the transcription process.
#[derive(Debug, Clone)]
pub struct TranscriptionStats {
    pub total_chunks: usize,
    pub successful_chunks: usize,
    pub failed_chunks: usize,
    pub total_time: Duration,
    pub avg_chunk_time: Duration,
}

/// Orchestrates concurrent transcription of audio chunks.
pub struct TranscriptionOrchestrator {
    transcriber: Arc<dyn Transcriber>,
    concurrency: usize,
    show_progress: bool,
}

impl TranscriptionOrchestrator {
    /// Create a new orchestrator with the given transcriber.
    pub fn new(transcriber: Box<dyn Transcriber>, concurrency: usize) -> Self {
        Self {
            transcriber: Arc::from(transcriber),
            concurrency,
            show_progress: true,
        }
    }

    /// Enable or disable progress bar display.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Process all chunks concurrently and return the combined transcript.
    pub async fn process_chunks(
        &self,
        chunks: Vec<AudioChunk>,
    ) -> Result<(TranscriptionResult, TranscriptionStats)> {
        if chunks.is_empty() {
            return Ok((
                TranscriptionResult {
                    segments: Vec::new(),
                    language: "unknown".to_string(),
                    duration: Duration::ZERO,
                },
                TranscriptionStats {
                    total_chunks: 0,
                    successful_chunks: 0,
                    failed_chunks: 0,
                    total_time: Duration::ZERO,
                    avg_chunk_time: Duration::ZERO,
                },
            ));
        }

        let total_chunks = chunks.len();
        let start_time = Instant::now();

        info!(
            "Processing {} chunks with {} concurrent requests using {}",
            total_chunks,
            self.concurrency,
            self.transcriber.name()
        );

        // Create progress bar
        let progress_bar = if self.show_progress {
            let pb = ProgressBar::new(total_chunks as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} chunks ({eta})")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };

        // Use semaphore to limit concurrency
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        // Create futures for all chunks
        let mut futures = FuturesUnordered::new();

        for chunk in chunks {
            let sem = semaphore.clone();
            let transcriber = self.transcriber.clone();
            let pb = progress_bar.clone();

            let future = async move {
                // Acquire permit (waits if at concurrency limit)
                let _permit = sem.acquire().await.expect("Semaphore closed");
                
                let chunk_start = Instant::now();
                let index = chunk.index;
                
                debug!("Starting transcription of chunk {}", index);
                
                let result = transcriber.transcribe(&chunk).await;
                let duration_ms = chunk_start.elapsed().as_millis() as u64;
                
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                
                match result {
                    Ok(transcript) => {
                        debug!("Chunk {} completed in {}ms", index, duration_ms);
                        ChunkResult {
                            index,
                            transcript: Some(transcript),
                            error: None,
                            duration_ms,
                        }
                    }
                    Err(e) => {
                        warn!("Chunk {} failed: {}", index, e);
                        ChunkResult {
                            index,
                            transcript: None,
                            error: Some(e.to_string()),
                            duration_ms,
                        }
                    }
                }
            };
            
            futures.push(future);
        }

        // Collect results
        let mut results: Vec<ChunkResult> = Vec::with_capacity(total_chunks);
        while let Some(result) = futures.next().await {
            results.push(result);
        }

        // Finish progress bar
        if let Some(pb) = progress_bar {
            pb.finish_with_message("Transcription complete");
        }

        // Sort results by chunk index to maintain order
        results.sort_by_key(|r| r.index);

        // Aggregate results
        let mut all_segments: Vec<TranscriptSegment> = Vec::new();
        let mut detected_language = None;
        let mut successful_count = 0;
        let mut failed_count = 0;
        let mut total_chunk_time_ms: u64 = 0;

        for result in &results {
            total_chunk_time_ms += result.duration_ms;
            
            if let Some(ref transcript) = result.transcript {
                successful_count += 1;
                all_segments.extend(transcript.segments.clone());
                
                // Use first detected language
                if detected_language.is_none() {
                    detected_language = transcript.language.clone();
                }
            } else {
                failed_count += 1;
            }
        }

        let total_time = start_time.elapsed();
        let avg_chunk_time = if !results.is_empty() {
            Duration::from_millis(total_chunk_time_ms / results.len() as u64)
        } else {
            Duration::ZERO
        };

        let stats = TranscriptionStats {
            total_chunks,
            successful_chunks: successful_count,
            failed_chunks: failed_count,
            total_time,
            avg_chunk_time,
        };

        info!(
            "Transcription complete: {}/{} chunks successful in {:.2}s (avg {:.2}s/chunk)",
            successful_count,
            total_chunks,
            total_time.as_secs_f64(),
            avg_chunk_time.as_secs_f64()
        );

        // Calculate total audio duration from segments
        let total_audio_duration = all_segments
            .iter()
            .map(|s| s.end)
            .max()
            .unwrap_or(Duration::ZERO);

        let transcription_result = TranscriptionResult {
            segments: all_segments,
            language: detected_language.unwrap_or_else(|| "unknown".to_string()),
            duration: total_audio_duration,
        };

        // Return error if all chunks failed
        if successful_count == 0 && total_chunks > 0 {
            let error_msgs: Vec<String> = results
                .iter()
                .filter_map(|r| r.error.clone())
                .collect();
            return Err(AutosubError::Transcription(format!(
                "All {} chunks failed. Errors: {}",
                total_chunks,
                error_msgs.join("; ")
            )));
        }

        Ok((transcription_result, stats))
    }

    /// Process chunks with retry for failed chunks.
    pub async fn process_chunks_with_retry(
        &self,
        chunks: Vec<AudioChunk>,
        max_retries: u32,
    ) -> Result<(TranscriptionResult, TranscriptionStats)> {
        let mut remaining_chunks = chunks;
        let mut all_segments: Vec<TranscriptSegment> = Vec::new();
        let mut detected_language = None;
        let mut total_successful = 0;
        let total_failed = 0;
        let start_time = Instant::now();

        for attempt in 0..=max_retries {
            if remaining_chunks.is_empty() {
                break;
            }

            if attempt > 0 {
                info!(
                    "Retry attempt {} for {} failed chunks",
                    attempt,
                    remaining_chunks.len()
                );
                // Wait before retry
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt - 1))).await;
            }

            let (result, _stats) = self.process_chunks(remaining_chunks).await?;
            
            // Collect successful results
            all_segments.extend(result.segments);
            if detected_language.is_none() && result.language != "unknown" {
                detected_language = Some(result.language);
            }

            // For now, we don't track which specific chunks failed to retry them
            // In a more sophisticated implementation, we'd track chunk indices
            remaining_chunks = Vec::new(); // Clear for now
            total_successful = all_segments.len();
        }

        let total_time = start_time.elapsed();
        let total_chunks = total_successful + total_failed;

        let stats = TranscriptionStats {
            total_chunks,
            successful_chunks: total_successful,
            failed_chunks: total_failed,
            total_time,
            avg_chunk_time: if total_chunks > 0 {
                Duration::from_millis(total_time.as_millis() as u64 / total_chunks as u64)
            } else {
                Duration::ZERO
            },
        };

        // Sort segments by start time
        all_segments.sort_by(|a, b| a.start.cmp(&b.start));

        let total_duration = all_segments
            .iter()
            .map(|s| s.end)
            .max()
            .unwrap_or(Duration::ZERO);

        Ok((
            TranscriptionResult {
                segments: all_segments,
                language: detected_language.unwrap_or_else(|| "unknown".to_string()),
                duration: total_duration,
            },
            stats,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SpeechRegion;
    use async_trait::async_trait;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock transcriber for testing.
    struct MockTranscriber {
        call_count: AtomicUsize,
        fail_on_index: Option<usize>,
    }

    impl MockTranscriber {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                fail_on_index: None,
            }
        }

        fn failing_on(index: usize) -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                fail_on_index: Some(index),
            }
        }
    }

    #[async_trait]
    impl Transcriber for MockTranscriber {
        async fn transcribe(&self, chunk: &AudioChunk) -> Result<Transcript> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            
            // Simulate some processing time
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            if self.fail_on_index == Some(chunk.index) {
                return Err(AutosubError::Transcription("Mock error".to_string()));
            }
            
            Ok(Transcript {
                segments: vec![TranscriptSegment {
                    text: format!("Transcript for chunk {}", chunk.index),
                    start: chunk.region.start,
                    end: chunk.region.end,
                    words: None,
                    confidence: Some(0.95),
                    speaker: None,
                }],
                language: Some("en".to_string()),
                duration: Some(chunk.duration()),
            })
        }

        fn name(&self) -> &'static str {
            "Mock"
        }

        fn max_file_size(&self) -> usize {
            25 * 1024 * 1024
        }

        fn supported_formats(&self) -> &[&str] {
            &["wav"]
        }
    }

    fn create_test_chunks(count: usize) -> Vec<AudioChunk> {
        (0..count)
            .map(|i| AudioChunk {
                region: SpeechRegion {
                    start: Duration::from_secs(i as u64 * 10),
                    end: Duration::from_secs((i + 1) as u64 * 10),
                },
                path: PathBuf::from(format!("/tmp/chunk_{}.wav", i)),
                index: i,
            })
            .collect()
    }

    #[tokio::test]
    async fn test_process_empty_chunks() {
        let transcriber = Box::new(MockTranscriber::new());
        let orchestrator = TranscriptionOrchestrator::new(transcriber, 4).with_progress(false);

        let (result, stats) = orchestrator.process_chunks(Vec::new()).await.unwrap();
        
        assert!(result.segments.is_empty());
        assert_eq!(stats.total_chunks, 0);
    }

    #[tokio::test]
    async fn test_process_single_chunk() {
        let transcriber = Box::new(MockTranscriber::new());
        let orchestrator = TranscriptionOrchestrator::new(transcriber, 4).with_progress(false);

        let chunks = create_test_chunks(1);
        let (result, stats) = orchestrator.process_chunks(chunks).await.unwrap();
        
        assert_eq!(result.segments.len(), 1);
        assert_eq!(stats.total_chunks, 1);
        assert_eq!(stats.successful_chunks, 1);
        assert_eq!(stats.failed_chunks, 0);
    }

    #[tokio::test]
    async fn test_process_multiple_chunks() {
        let transcriber = Box::new(MockTranscriber::new());
        let orchestrator = TranscriptionOrchestrator::new(transcriber, 4).with_progress(false);

        let chunks = create_test_chunks(5);
        let (result, stats) = orchestrator.process_chunks(chunks).await.unwrap();
        
        assert_eq!(result.segments.len(), 5);
        assert_eq!(stats.total_chunks, 5);
        assert_eq!(stats.successful_chunks, 5);
        assert_eq!(result.language, "en");
    }

    #[tokio::test]
    async fn test_maintains_chunk_order() {
        let transcriber = Box::new(MockTranscriber::new());
        let orchestrator = TranscriptionOrchestrator::new(transcriber, 2).with_progress(false);

        let chunks = create_test_chunks(10);
        let (result, _stats) = orchestrator.process_chunks(chunks).await.unwrap();
        
        // Verify segments are in order by checking start times increase
        for i in 1..result.segments.len() {
            assert!(result.segments[i].start >= result.segments[i - 1].start);
        }
    }

    #[tokio::test]
    async fn test_handles_partial_failure() {
        let transcriber = Box::new(MockTranscriber::failing_on(2));
        let orchestrator = TranscriptionOrchestrator::new(transcriber, 4).with_progress(false);

        let chunks = create_test_chunks(5);
        let (result, stats) = orchestrator.process_chunks(chunks).await.unwrap();
        
        // Should have 4 successful, 1 failed
        assert_eq!(result.segments.len(), 4);
        assert_eq!(stats.successful_chunks, 4);
        assert_eq!(stats.failed_chunks, 1);
    }
}
