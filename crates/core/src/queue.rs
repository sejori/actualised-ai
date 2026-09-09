use std::sync::Arc;
use tokio::sync::{Semaphore};
use tokio::time::Instant;

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

pub struct InferenceQueue {
    engine: Arc<Box<dyn InferenceEngine>>,
    max_concurrent_requests: usize,
}

impl InferenceQueue {
    pub fn new(engine: Box<dyn InferenceEngine>, max_concurrent_requests: usize) -> Self {
        Self {
            engine: Arc::new(engine),
            max_concurrent_requests,
        }
    }

    /// Processes a batch of requests in parallel, respecting the concurrency limit.
    pub async fn process_batch(&self, requests: Vec<InferenceRequest>) -> (Vec<(String, Result<InferenceResponse, String>)>, QueueStats) {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));
        let mut handles = Vec::new();
        let total_requests = requests.len();

        let start_time = Instant::now();

        for req in requests {
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
