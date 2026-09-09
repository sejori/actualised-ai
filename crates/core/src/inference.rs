use async_trait::async_trait;
use serde_json::json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InferenceResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

#[async_trait]
pub trait InferenceEngine: Send + Sync {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String>;
}

pub struct MockInferenceEngine;

#[async_trait]
impl InferenceEngine for MockInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if tools.is_empty() {
            Ok(InferenceResponse::Text(format!("[MOCK INFERENCE RESPONSE]. Received system prompt length: {}, user prompt length: {}.", system_prompt.len(), user_prompt.len())))
        } else {
            Ok(InferenceResponse::ToolCalls(vec![ToolCall {
                id: "mock_id".to_string(),
                name: tools[0].name.clone(),
                args: json!({}),
            }]))
        }
    }
}

pub struct GeminiInferenceEngine {
    pub api_key: String,
}

#[async_trait]
impl InferenceEngine for GeminiInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        let client = reqwest::Client::new();
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:generateContent?key={}", self.api_key);
        
        let mut body = json!({
            "systemInstruction": { "parts": [{ "text": system_prompt }] },
            "contents": [{ "parts": [{ "text": user_prompt }] }]
        });

        if !tools.is_empty() {
            let function_declarations: Vec<serde_json::Value> = tools.into_iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                })
            }).collect();
            
            body["tools"] = json!([{
                "function_declarations": function_declarations
            }]);
        }
        
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
        
        if let Some(parts) = response_json["candidates"][0]["content"]["parts"].as_array() {
            let mut tool_calls = Vec::new();
            for part in parts {
                if let Some(func_call) = part.get("functionCall") {
                    tool_calls.push(ToolCall {
                        id: "gemini_tool_call".to_string(), // Gemini doesn't always provide an ID in basic genContent
                        name: func_call["name"].as_str().unwrap_or_default().to_string(),
                        args: func_call["args"].clone(),
                    });
                }
            }
            if !tool_calls.is_empty() {
                return Ok(InferenceResponse::ToolCalls(tool_calls));
            }
            
            if let Some(text) = parts[0].get("text") {
                return Ok(InferenceResponse::Text(text.as_str().unwrap_or_default().to_string()));
            }
        }

        Err("Failed to parse Gemini response parts".to_string())
    }
}
