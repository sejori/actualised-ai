use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct BatchJobRecord {
    pub id: String,
    pub company_id: Option<String>,
    pub requesting_agent_id: String,
    pub input_file_id: String,
    pub output_file_id: Option<String>,
    pub error_file_id: Option<String>,
    pub status: String,
    pub endpoint: String,
    pub total_requests: usize,
    pub completed_requests: usize,
    pub failed_requests: usize,
    pub destination_path: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJobItemInput {
    pub custom_id: String,
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOutputEntry {
    pub id: String,
    pub custom_id: String,
    pub response: Option<BatchResponseBody>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponseBody {
    pub status_code: u16,
    pub request_id: Option<String>,
    pub body: serde_json::Value,
}

pub struct DoublewordBatchClient {
    pub api_key: String,
    pub base_url: String,
    pub timeout_secs: u64,
}

impl DoublewordBatchClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.doubleword.ai/v1".to_string()),
            timeout_secs: 60,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn upload_batch_jsonl(&self, jsonl_content: &str, filename: Option<&str>) -> Result<String, String> {
        use reqwest::multipart::{Form, Part};

        let fname = filename.unwrap_or("batch_requests.jsonl").to_string();
        let part = Part::bytes(jsonl_content.as_bytes().to_vec())
            .file_name(fname)
            .mime_str("application/jsonl")
            .map_err(|e| format!("Failed to create multipart part: {}", e))?;

        let form = Form::new()
            .text("purpose", "batch")
            .part("file", part);

        let url = format!("{}/files", self.base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let res = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Failed to upload batch file: {}", e))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Batch file upload failed: {}", err));
        }

        let resp_json: serde_json::Value = res.json().await
            .map_err(|e| format!("Failed to parse file upload response: {}", e))?;

        let file_id = resp_json.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing file id in upload response".to_string())?
            .to_string();

        Ok(file_id)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn upload_batch_jsonl(&self, _jsonl_content: &str, _filename: Option<&str>) -> Result<String, String> {
        Err("Batch file upload not supported in WASM".to_string())
    }

    pub async fn create_batch(&self, input_file_id: &str, completion_window: Option<&str>) -> Result<serde_json::Value, String> {
        let url = format!("{}/batches", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "input_file_id": input_file_id,
            "endpoint": "/v1/chat/completions",
            "completion_window": completion_window.unwrap_or("24h")
        });

        let mut builder = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.timeout(std::time::Duration::from_secs(self.timeout_secs));
        }
        let client = builder.build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let res = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to create batch: {}", e))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Batch creation failed: {}", err));
        }

        let resp_json: serde_json::Value = res.json().await
            .map_err(|e| format!("Failed to parse batch create response: {}", e))?;

        Ok(resp_json)
    }

    pub async fn get_batch(&self, batch_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/batches/{}", self.base_url.trim_end_matches('/'), batch_id);
        let mut builder = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.timeout(std::time::Duration::from_secs(self.timeout_secs));
        }
        let client = builder.build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let res = client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to retrieve batch: {}", e))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Batch retrieval failed: {}", err));
        }

        let resp_json: serde_json::Value = res.json().await
            .map_err(|e| format!("Failed to parse batch retrieval response: {}", e))?;

        Ok(resp_json)
    }

    pub async fn get_file_content(&self, file_id: &str) -> Result<String, String> {
        let url = format!("{}/files/{}/content", self.base_url.trim_end_matches('/'), file_id);
        let mut builder = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.timeout(std::time::Duration::from_secs(self.timeout_secs));
        }
        let client = builder.build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let res = client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to retrieve file content: {}", e))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("File content retrieval failed: {}", err));
        }

        let text = res.text().await
            .map_err(|e| format!("Failed to read file text: {}", e))?;

        Ok(text)
    }
}

pub fn build_chat_jsonl_line(
    custom_id: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: Option<f32>,
) -> String {
    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ]
    });
    if let Some(temp) = temperature {
        body["temperature"] = serde_json::json!(temp);
    }

    let line_obj = serde_json::json!({
        "custom_id": custom_id,
        "method": "POST",
        "url": "/v1/chat/completions",
        "body": body
    });

    line_obj.to_string()
}
