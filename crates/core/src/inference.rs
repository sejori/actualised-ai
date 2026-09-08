use async_trait::async_trait;
use serde_json::json;

#[async_trait]
pub trait InferenceEngine: Send + Sync {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String>;
}

pub struct MockInferenceEngine;

#[async_trait]
impl InferenceEngine for MockInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        Ok(format!("[MOCK INFERENCE RESPONSE]. Received system prompt length: {}, user prompt length: {}.", system_prompt.len(), user_prompt.len()))
    }
}

pub struct GeminiInferenceEngine {
    pub api_key: String,
}

#[async_trait]
impl InferenceEngine for GeminiInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        let client = reqwest::Client::new();
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:generateContent?key={}", self.api_key);
        
        let body = json!({
            "systemInstruction": { "parts": [{ "text": system_prompt }] },
            "contents": [{ "parts": [{ "text": user_prompt }] }]
        });
        
        let res = client.post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
            
        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(format!("Gemini API Error: {}", error_text));
        }
        
        let response_json: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let text = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("Failed to extract text from response")
            .to_string();
            
        Ok(text)
    }
}
