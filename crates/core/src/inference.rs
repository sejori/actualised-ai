use serde::{Deserialize, Serialize};
use serde_json::json;
use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub company_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceStats {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceResult {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(alias = "serviceTier")]
    pub service_tier: Option<String>,
    #[serde(alias = "apiKey")]
    pub api_key: String,
    #[serde(alias = "telegramBotToken")]
    pub telegram_bot_token: Option<String>,
}

/// Builds the engine matching a user's chosen provider. Falls back to the mock engine
/// when the provider is unrecognised or no API key was supplied.
pub fn build_engine(config: &InferenceConfig) -> Box<dyn InferenceEngine> {
    let mut resolved_key = config.api_key.clone();
    if resolved_key.trim().is_empty() {
        if let Ok(env_key) = std::env::var("INFERENCE_API_KEY") {
            resolved_key = env_key;
        } else {
            return Box::new(MockInferenceEngine);
        }
    }

    match config.provider.as_str() {
        "gemini" => Box::new(GeminiInferenceEngine {
            api_key: resolved_key,
            model: if config.model.trim().is_empty() { "gemini-3.6-flash".to_string() } else { config.model.clone() },
        }),
        "openai" => Box::new(OpenAiInferenceEngine {
            api_key: resolved_key,
            model: if config.model.trim().is_empty() { "gpt-5".to_string() } else { config.model.clone() },
        }),
        "anthropic" => Box::new(AnthropicInferenceEngine {
            api_key: resolved_key,
            model: if config.model.trim().is_empty() { "claude-sonnet-4-5".to_string() } else { config.model.clone() },
        }),
        _ => Box::new(MockInferenceEngine),
    }
}

pub struct GeminiInferenceEngine {
    pub api_key: String,
    pub model: String,
}

fn gemini_request_body(system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> serde_json::Value {
    let mut body = json!({
        "systemInstruction": { "parts": [{ "text": system_prompt }] },
        "contents": [{ "parts": [{ "text": user_prompt }] }]
    });

    if !tools.is_empty() {
        let allowed_function_names: Vec<String> = tools.iter().map(|tool| tool.name.clone()).collect();
        let function_declarations: Vec<serde_json::Value> = tools.into_iter().map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters
            })
        }).collect();

        body["tools"] = json!([{
            "function_declarations": function_declarations
        }]);
        body["toolConfig"] = json!({
            "functionCallingConfig": {
                "mode": "AUTO",
                "allowedFunctionNames": allowed_function_names
            }
        });
    }

    body
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl InferenceEngine for GeminiInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        let client = reqwest::Client::new();
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", self.model, self.api_key);
        let body = gemini_request_body(system_prompt, user_prompt, tools);
        
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

pub struct OpenAiInferenceEngine {
    pub api_key: String,
    pub model: String,
}

fn openai_request_body(model: &str, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> serde_json::Value {
    let mut body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ]
    });

    if !tools.is_empty() {
        let tools_json: Vec<serde_json::Value> = tools.into_iter().map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                }
            })
        }).collect();

        body["tools"] = json!(tools_json);
        body["tool_choice"] = json!("auto");
    }

    body
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl InferenceEngine for OpenAiInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        let client = reqwest::Client::new();
        let url = "https://api.openai.com/v1/chat/completions";
        let body = openai_request_body(&self.model, system_prompt, user_prompt, tools);
        
        let res = client.post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
            
        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(format!("OpenAI API Error: {}", error_text));
        }
        
        let response_json: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let mut stats = InferenceStats {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        
        if let Some(usage) = response_json.get("usage") {
            stats.prompt_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            stats.completion_tokens = usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            stats.total_tokens = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        }

        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(message) = choices[0].get("message") {
                if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
                    let mut parsed_calls = Vec::new();
                    for tc in tool_calls {
                        if tc.get("type").and_then(|t| t.as_str()) == Some("function") {
                            if let Some(func) = tc.get("function") {
                                let name = func.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string();
                                let args_str = func.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");
                                let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                                let id = tc.get("id").and_then(|i| i.as_str()).unwrap_or_default().to_string();
                                parsed_calls.push(ToolCall { id, name, args });
                            }
                        }
                    }
                    if !parsed_calls.is_empty() {
                        return Ok(InferenceResponse {
                            result: InferenceResult::ToolCalls(parsed_calls),
                            stats,
                        });
                    }
                }
                
                if let Some(text) = message.get("content").and_then(|c| c.as_str()) {
                    return Ok(InferenceResponse {
                        result: InferenceResult::Text(text.to_string()),
                        stats,
                    });
                }
            }
        }

        Err("Failed to parse OpenAI response".to_string())
    }
}

pub struct AnthropicInferenceEngine {
    pub api_key: String,
    pub model: String,
}

fn anthropic_request_body(model: &str, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> serde_json::Value {
    let mut body = json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{ "role": "user", "content": user_prompt }]
    });

    if !tools.is_empty() {
        let tools_json: Vec<serde_json::Value> = tools.into_iter().map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters
            })
        }).collect();

        body["tools"] = json!(tools_json);
        body["tool_choice"] = json!({ "type": "auto" });
    }

    body
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl InferenceEngine for AnthropicInferenceEngine {
    async fn generate_response(&self, system_prompt: &str, user_prompt: &str, tools: Vec<Tool>) -> Result<InferenceResponse, String> {
        let body = anthropic_request_body(&self.model, system_prompt, user_prompt, tools);
        let mut request = reqwest::Client::new()
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body);
        #[cfg(target_arch = "wasm32")]
        {
            request = request.header("anthropic-dangerous-direct-browser-access", "true");
        }
        let res = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(format!("Anthropic API Error: {}", error_text));
        }

        let response_json: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let input_tokens = response_json["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = response_json["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize;
        let stats = InferenceStats {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens,
            total_tokens: input_tokens + output_tokens,
        };

        let content = response_json["content"].as_array().ok_or_else(|| "Failed to parse Anthropic response content".to_string())?;
        let tool_calls: Vec<ToolCall> = content.iter().filter_map(|block| {
            if block["type"].as_str() != Some("tool_use") {
                return None;
            }
            Some(ToolCall {
                id: block["id"].as_str().unwrap_or_default().to_string(),
                name: block["name"].as_str().unwrap_or_default().to_string(),
                args: block["input"].clone(),
            })
        }).collect();

        if !tool_calls.is_empty() {
            return Ok(InferenceResponse { result: InferenceResult::ToolCalls(tool_calls), stats });
        }

        let text = content.iter()
            .filter(|block| block["type"].as_str() == Some("text"))
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            return Ok(InferenceResponse { result: InferenceResult::Text(text), stats });
        }

        Err("Failed to parse Anthropic response".to_string())
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
            company_id: None,
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

    #[test]
    fn gemini_payload_allows_only_offered_tools() {
        let body = gemini_request_body("system", "user", vec![Tool {
            name: "read_memory".to_string(),
            description: "Read memory".to_string(),
            parameters: json!({ "type": "object" }),
            company_id: None,
        }]);

        assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "AUTO");
        assert_eq!(body["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"], json!(["read_memory"]));
        assert_eq!(body["tools"][0]["function_declarations"][0]["name"], "read_memory");
    }

    #[test]
    fn provider_payloads_use_native_automatic_tool_choice() {
        let tool = Tool {
            name: "read_memory".to_string(),
            description: "Read memory".to_string(),
            parameters: json!({ "type": "object" }),
            company_id: None,
        };

        let openai = openai_request_body("gpt-5", "system", "user", vec![tool.clone()]);
        assert_eq!(openai["tool_choice"], "auto");
        assert_eq!(openai["tools"][0]["function"]["name"], "read_memory");

        let anthropic = anthropic_request_body("claude-sonnet-4-5", "system", "user", vec![tool]);
        assert_eq!(anthropic["tool_choice"], json!({ "type": "auto" }));
        assert_eq!(anthropic["tools"][0]["name"], "read_memory");
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
            company_id: None,
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
