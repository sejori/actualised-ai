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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chat_jsonl_line_structure() {
        let line = build_chat_jsonl_line(
            "item_001",
            "deepseek-ai/DeepSeek-V4.1-Flash",
            "You are an assistant.",
            "Tell me a joke.",
            Some(0.7),
        );

        let parsed: serde_json::Value = serde_json::from_str(&line).expect("JSONL line must be valid JSON");
        assert_eq!(parsed["custom_id"], "item_001");
        assert_eq!(parsed["method"], "POST");
        assert_eq!(parsed["url"], "/v1/chat/completions");
        assert_eq!(parsed["body"]["model"], "deepseek-ai/DeepSeek-V4.1-Flash");
        assert_eq!(parsed["body"]["messages"][0]["role"], "system");
        assert_eq!(parsed["body"]["messages"][0]["content"], "You are an assistant.");
        assert_eq!(parsed["body"]["messages"][1]["role"], "user");
        assert_eq!(parsed["body"]["messages"][1]["content"], "Tell me a joke.");
        let temp = parsed["body"]["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 1e-4);
    }

    #[test]
    fn test_batch_job_record_serde() {
        let record = BatchJobRecord {
            id: "batch_123".to_string(),
            company_id: Some("comp_456".to_string()),
            requesting_agent_id: "agent_lead".to_string(),
            input_file_id: "file_in_789".to_string(),
            output_file_id: Some("file_out_012".to_string()),
            error_file_id: None,
            status: "completed".to_string(),
            endpoint: "/v1/chat/completions".to_string(),
            total_requests: 10,
            completed_requests: 10,
            failed_requests: 0,
            destination_path: Some("batches/job_123.jsonl".to_string()),
            created_at: 1700000000,
            completed_at: Some(1700000050),
        };

        let json_str = serde_json::to_string(&record).unwrap();
        let decoded: BatchJobRecord = serde_json::from_str(&json_str).unwrap();

        assert_eq!(decoded.id, "batch_123");
        assert_eq!(decoded.status, "completed");
        assert_eq!(decoded.completed_requests, 10);
        assert_eq!(decoded.destination_path.as_deref(), Some("batches/job_123.jsonl"));
    }

    #[test]
    fn test_batch_output_entry_parsing() {
        let raw_output = r#"{
            "id": "batch_req_001",
            "custom_id": "sword-desc",
            "response": {
                "status_code": 200,
                "request_id": "req-xyz",
                "body": {
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "A rusted iron longsword forged in ancient flames."
                            }
                        }
                    ]
                }
            },
            "error": null
        }"#;

        let entry: BatchOutputEntry = serde_json::from_str(raw_output).expect("Must parse BatchOutputEntry");
        assert_eq!(entry.custom_id, "sword-desc");
        let resp = entry.response.expect("Must have response");
        assert_eq!(resp.status_code, 200);
        assert_eq!(
            resp.body["choices"][0]["message"]["content"],
            "A rusted iron longsword forged in ancient flames."
        );
    }
}
