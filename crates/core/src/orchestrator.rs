use crate::state::{CompanyState, Project};
use crate::inference::{InferenceEngine, MockInferenceEngine, Tool, InferenceResult};
use crate::memory::MemoryManager;
use crate::queue::{InferenceQueue, InferenceRequest, RateLimitConfig};
use petgraph::graph::DiGraph;
use std::collections::HashMap;

const DEFAULT_USER_PROMPT: &str = "What actions will you take on your assigned tasks?";

/// The rolling conversational state the UI can inspect for a single agent.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentContext {
    /// Operator messages queued via the UI, to be folded into the agent's next turn.
    pub pending_messages: Vec<String>,
    pub last_user_prompt: Option<String>,
    pub last_response: Option<String>,
}

pub struct Orchestrator {
    pub state: CompanyState,
    pub task_graph: DiGraph<String, ()>,
    pub inference: Box<dyn InferenceEngine>,
    pub memory: MemoryManager,
    pub rate_limit: RateLimitConfig,
    pub agent_contexts: HashMap<String, AgentContext>,
}

impl Orchestrator {
    pub fn new(state: CompanyState, memory: MemoryManager) -> Self {
        Self {
            state,
            task_graph: DiGraph::new(),
            inference: Box::new(MockInferenceEngine),
            memory,
            rate_limit: RateLimitConfig::default(),
            agent_contexts: HashMap::new(),
        }
    }

    pub fn set_inference(&mut self, engine: Box<dyn InferenceEngine>) {
        self.inference = engine;
    }

    pub fn set_rate_limit(&mut self, rate_limit: RateLimitConfig) {
        self.rate_limit = rate_limit;
    }

    /// Queues an operator message to be appended to an agent's prompt on its next turn.
    pub fn queue_message(&mut self, agent_id: &str, message: String) {
        self.agent_contexts.entry(agent_id.to_string()).or_default().pending_messages.push(message);
    }

    pub fn get_agent_context(&self, agent_id: &str) -> AgentContext {
        self.agent_contexts.get(agent_id).cloned().unwrap_or_default()
    }

    pub async fn run(&mut self) {
        println!("Orchestrator staging work to inference queue...");
        
        // Take the configured inference engine
        let engine = std::mem::replace(&mut self.inference, Box::new(MockInferenceEngine));
        let queue = InferenceQueue::new(engine, self.rate_limit);
        
        let agents = self.state.agents.clone();
        let mut requests = Vec::new();

        for agent in &agents {
            println!("Staging work for agent: {} ({})", agent.name, agent.role);
            
            let mut defined_tools = Vec::new();
            for t in &agent.tools {
                if t == "create_sub_project" {
                    defined_tools.push(Tool {
                        name: "create_sub_project".to_string(),
                        description: "Create a sub project/epic".to_string(),
                        parameters: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "description": { "type": "string" }
                            },
                            "required": ["title", "description"]
                        }),
                    });
                } else if t == "write_memory" {
                    defined_tools.push(Tool {
                        name: "write_memory".to_string(),
                        description: "Write content to your local memory footprint".to_string(),
                        parameters: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "file_name": { "type": "string" },
                                "content": { "type": "string" }
                            },
                            "required": ["file_name", "content"]
                        }),
                    });
                }
            }

            let context = self.agent_contexts.entry(agent.id.clone()).or_default();
            let pending = std::mem::take(&mut context.pending_messages);
            let user_prompt = if pending.is_empty() {
                DEFAULT_USER_PROMPT.to_string()
            } else {
                format!("{}\n\nOperator messages for this turn:\n{}", DEFAULT_USER_PROMPT, pending.iter().map(|m| format!("- {}", m)).collect::<Vec<_>>().join("\n"))
            };
            context.last_user_prompt = Some(user_prompt.clone());

            requests.push(InferenceRequest {
                agent_id: agent.id.clone(),
                system_prompt: agent.system_prompt.clone(),
                user_prompt,
                tools: defined_tools,
            });
        }

        println!("Executing {} batched requests in parallel...", requests.len());
        let (responses, stats) = queue.process_batch(requests).await;

        println!("\n--- Inference Queue Stats ---");
        println!("Total Requests: {}", stats.total_requests);
        println!("Total Tokens: {}", stats.total_tokens);
        println!("Prompt Tokens: {}", stats.total_prompt_tokens);
        println!("Completion Tokens: {}", stats.total_completion_tokens);
        println!("Elapsed Time: {:.2}s", stats.elapsed_time_sec);
        println!("Tokens/sec: {:.2}", stats.tokens_per_sec);
        println!("-----------------------------\n");

        for (agent_id, response) in responses {
            match response {
                Ok(res) => {
                    match &res.result {
                        InferenceResult::Text(text) => {
                            println!("Agent {} Response (Text): {}", agent_id, text);
                            self.agent_contexts.entry(agent_id.clone()).or_default().last_response = Some(text.clone());
                        }
                        InferenceResult::ToolCalls(calls) => {
                            self.agent_contexts.entry(agent_id.clone()).or_default().last_response =
                                Some(format!("Called tools: {}", calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(", ")));
                            for call in calls {
                                println!("Agent {} called tool: {}", agent_id, call.name);
                                if call.name == "create_sub_project" {
                                    let title = call.args["title"].as_str().unwrap_or_default().to_string();
                                    let description = call.args["description"].as_str().unwrap_or_default().to_string();
                                    let project = Project {
                                        id: format!("proj_{}", uuid::Uuid::new_v4().simple()),
                                        title: title.clone(),
                                        description,
                                    };
                                    if let Err(e) = self.state.add_project(project).await {
                                        println!("Failed to create project: {}", e);
                                    } else {
                                        println!("Successfully created sub-project: {}", title);
                                    }
                                } else if call.name == "write_memory" {
                                    let file_name = call.args["file_name"].as_str().unwrap_or("output.txt");
                                    let content = call.args["content"].as_str().unwrap_or_default();
                                    if let Err(e) = self.memory.write_memory(&agent_id, file_name, content) {
                                        println!("Failed to write memory: {}", e);
                                    } else {
                                        println!("Successfully wrote memory file: {}", file_name);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Inference Error for Agent {}: {}", agent_id, e);
                    self.agent_contexts.entry(agent_id.clone()).or_default().last_response = Some(format!("Error: {}", e));
                }
            }
        }

        println!("DFS/Batch Loop complete. Engine resting.");
    }
}
