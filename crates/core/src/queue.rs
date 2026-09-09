use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore};
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{sleep, Instant};
#[cfg(target_arch = "wasm32")]
use wasmtimer::std::Instant;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

use crate::inference::{InferenceEngine, InferenceResponse, Tool};

pub struct InferenceRequest {
    pub agent_id: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub tools: Vec<Tool>,
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_requests: usize,
    pub total_prompt_tokens: usize,
    pub total_completion_tokens: usize,
    pub total_tokens: usize,
    pub elapsed_time_sec: f64,
    pub tokens_per_sec: f64,
}

/// User-configurable throughput limits for the inference queue, driven by the settings UI.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RateLimitConfig {
    pub max_concurrent_requests: usize,
    /// Maximum number of requests dispatched per second. `0` means unlimited (only bounded by concurrency).
    pub requests_per_second: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self { max_concurrent_requests: 5, requests_per_second: 0.0 }
    }
}

pub struct InferenceQueue {
    engine: Arc<Box<dyn InferenceEngine>>,
    rate_limit: RateLimitConfig,
}

impl InferenceQueue {
    pub fn new(engine: Box<dyn InferenceEngine>, rate_limit: RateLimitConfig) -> Self {
        Self {
            engine: Arc::new(engine),
            rate_limit,
        }
    }

    /// Processes a batch of requests in parallel, respecting the configured concurrency and
    /// requests-per-second limits.
    pub async fn process_batch(&self, requests: Vec<InferenceRequest>) -> (Vec<(String, Result<InferenceResponse, String>)>, QueueStats) {
        let max_concurrent = self.rate_limit.max_concurrent_requests.max(1);
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut handles = Vec::new();
        let total_requests = requests.len();
        let dispatch_interval = if self.rate_limit.requests_per_second > 0.0 {
            Some(Duration::from_secs_f64(1.0 / self.rate_limit.requests_per_second))
        } else {
            None
        };

        let start_time = Instant::now();

        for (index, req) in requests.into_iter().enumerate() {
            if let Some(interval) = dispatch_interval {
                if index > 0 {
                    sleep(interval).await;
                }
            }

            let engine_clone = Arc::clone(&self.engine);
            let sem_clone = Arc::clone(&semaphore);
            let agent_id = req.agent_id.clone();

            let (tx, rx) = tokio::sync::oneshot::channel();
            
            let fut = async move {
                let _permit = sem_clone.acquire().await.unwrap();
                let res = engine_clone.generate_response(&req.system_prompt, &req.user_prompt, req.tools).await;
                let _ = tx.send((agent_id, res));
            };

            #[cfg(not(target_arch = "wasm32"))]
            tokio::spawn(fut);
            
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(fut);

            handles.push(rx);
        }

        let mut results = Vec::new();
        let mut total_prompt_tokens = 0;
        let mut total_completion_tokens = 0;
        let mut total_tokens = 0;

        for handle in handles {
            if let Ok((agent_id, res)) = handle.await {
                if let Ok(response) = &res {
                    total_prompt_tokens += response.stats.prompt_tokens;
                    total_completion_tokens += response.stats.completion_tokens;
                    total_tokens += response.stats.total_tokens;
                }
                results.push((agent_id, res));
            }
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        let tokens_per_sec = if elapsed > 0.0 {
            total_tokens as f64 / elapsed
        } else {
            0.0
        };

        let stats = QueueStats {
            total_requests,
            total_prompt_tokens,
            total_completion_tokens,
            total_tokens,
            elapsed_time_sec: elapsed,
            tokens_per_sec,
        };

        (results, stats)
    }
}
