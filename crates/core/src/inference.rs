use async_trait::async_trait;
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct Tool {
    pub company_id: Option<String>,
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
pub struct InferenceStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InferenceResult {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceResponse {
    pub result: InferenceResult,
    pub stats: InferenceStats,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait InferenceEngine: Send + Sync {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String>;
}

pub struct MockInferenceEngine;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl InferenceEngine for MockInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        #[cfg(target_arch = "wasm32")]
        wasmtimer::tokio::sleep(std::time::Duration::from_millis(500)).await;
        
        let stats = InferenceStats {
            prompt_tokens: system_prompt.len() + user_prompt.len(),
            completion_tokens: 50,
            total_tokens: system_prompt.len() + user_prompt.len() + 50,
        };

        if tools.is_empty() {
            Ok(InferenceResponse {
                result: InferenceResult::Text(format!("[MOCK INFERENCE RESPONSE]. Received system prompt length: {}, user prompt length: {}.", system_prompt.len(), user_prompt.len())),
                stats,
            })
        } else {
            Ok(InferenceResponse {
                result: InferenceResult::ToolCalls(vec![ToolCall {
                    id: "mock_id".to_string(),
                    name: tools[0].name.clone(),
                    args: json!({}),
                }]),
                stats,
            })
        }
    }
}

/// User-selected inference configuration, coming straight from the web UI settings popover.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceConfig {
    pub provider: String,
    pub model: String,
    pub service_tier: Option<String>,
    pub api_key: String,
}

/// Builds the engine matching a user's chosen provider. Falls back to the mock engine
/// when the provider is unrecognised or no API key was supplied.
pub fn build_engine(config: &InferenceConfig) -> Box<dyn InferenceEngine> {
    if config.api_key.trim().is_empty() {
        return Box::new(MockInferenceEngine);
    }

    match config.provider.as_str() {
        "gemini" => Box::new(GeminiInferenceEngine {
            api_key: config.api_key.clone(),
            model: if config.model.trim().is_empty() { "gemini-3.6-flash".to_string() } else { config.model.clone() },
        }),
        _ => Box::new(MockInferenceEngine),
    }
}

pub struct GeminiInferenceEngine {
    pub api_key: String,
    pub model: String,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl InferenceEngine for GeminiInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        let client = reqwest::Client::new();
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", self.model, self.api_key);
        
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
        
        let mut stats = InferenceStats {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        
        if let Some(usage) = response_json.get("usageMetadata") {
            stats.prompt_tokens = usage.get("promptTokenCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            stats.completion_tokens = usage.get("candidatesTokenCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            stats.total_tokens = usage.get("totalTokenCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        }

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
                return Ok(InferenceResponse {
                    result: InferenceResult::ToolCalls(tool_calls),
                    stats,
                });
            }
            
            if let Some(text) = parts[0].get("text") {
                return Ok(InferenceResponse {
                    result: InferenceResult::Text(text.as_str().unwrap_or_default().to_string()),
                    stats,
                });
            }
        }

        Err("Failed to parse Gemini response parts".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: json!({ "type": "object" }),
        };
        let serialized = serde_json::to_string(&tool).unwrap();
        assert!(serialized.contains("test_tool"));
    }

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            id: "call_123".to_string(),
            name: "do_work".to_string(),
            args: json!({ "target": "main.rs" }),
        };
        let serialized = serde_json::to_string(&call).unwrap();
        assert!(serialized.contains("call_123"));
    }

    #[tokio::test]
    async fn test_mock_engine_text() {
        let engine = MockInferenceEngine;
        let res = engine.generate_response("sys", "user", vec![]).await.unwrap();
        if let InferenceResult::Text(t) = res.result {
            assert!(t.contains("[MOCK INFERENCE RESPONSE]"));
        } else {
            panic!("Expected Text response");
        }
    }

    #[tokio::test]
    async fn test_mock_engine_tool() {
        let engine = MockInferenceEngine;
        let tool = Tool {
            name: "mock_tool".to_string(),
            description: "mock desc".to_string(),
            parameters: json!({}),
        };
        let res = engine.generate_response("sys", "user", vec![tool]).await.unwrap();
        if let InferenceResult::ToolCalls(calls) = res.result {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].name, "mock_tool");
        } else {
            panic!("Expected ToolCalls response");
        }
    }
}

