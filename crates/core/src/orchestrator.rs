use crate::state::{CompanyState, Project};
use crate::inference::{InferenceEngine, MockInferenceEngine, InferenceResult, ToolCall};
use crate::memory::MemoryManager;
use crate::queue::{InferenceQueue, InferenceRequest, RateLimitConfig};
use async_trait::async_trait;
use petgraph::graph::DiGraph;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, agent_id: &str, tool_call: &ToolCall) -> Result<String, String>;
}

const DEFAULT_USER_PROMPT: &str = "What actions will you take on your assigned tasks?";
/// How many past turns to keep per agent so the context view doesn't grow unbounded.
const MAX_HISTORY_TURNS: usize = 40;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationTurn {
    /// "operator" for messages queued via the UI, "agent" for inference responses.
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentContext {
    pub history: Vec<ConversationTurn>,
}

impl AgentContext {
    fn push_turn(&mut self, role: &str, content: String) {
        self.history.push(ConversationTurn { role: role.to_string(), content });
        if self.history.len() > MAX_HISTORY_TURNS {
            let overflow = self.history.len() - MAX_HISTORY_TURNS;
            self.history.drain(0..overflow);
        }
    }
}

pub struct Orchestrator {
    pub state: CompanyState,
    pub task_graph: DiGraph<String, ()>,
    pub inference: Box<dyn InferenceEngine>,
    pub memory: MemoryManager,
    pub rate_limit: RateLimitConfig,
    pub agent_contexts: HashMap<String, AgentContext>,
    pub tool_executor: Option<Arc<dyn ToolExecutor>>,
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
            tool_executor: None,
        }
    }

    pub fn set_inference(&mut self, engine: Box<dyn InferenceEngine>) {
        self.inference = engine;
    }

    pub fn set_rate_limit(&mut self, rate_limit: RateLimitConfig) {
        self.rate_limit = rate_limit;
    }

    pub fn set_tool_executor(&mut self, executor: Arc<dyn ToolExecutor>) {
        self.tool_executor = Some(executor);
    }

    /// Queues an operator message to be appended to an agent's prompt on its next turn.
    pub async fn queue_message(&mut self, agent_id: &str, message: String) -> Result<(), String> {
        if let Some(agent) = self.state.agents.iter().find(|a| a.id == agent_id).cloned() {
            let mut updated_agent = agent.clone();
            let mut pending = updated_agent.pending_messages.unwrap_or_default();
            pending.push(message);
            updated_agent.pending_messages = Some(pending);
            self.state.update_agent(agent_id, updated_agent).await?;
            Ok(())
        } else {
            Err("Agent not found".to_string())
        }
    }

    pub fn get_agent_context(&self, agent_id: &str) -> AgentContext {
        self.agent_contexts.get(agent_id).cloned().unwrap_or_default()
    }

    pub fn get_agent_memory_tree(&self, agent_id: &str) -> Vec<crate::memory::MemoryNode> {
        self.memory.read_memory_tree(agent_id)
    }

    pub fn get_shared_tree(&self) -> Vec<crate::memory::MemoryNode> {
        self.memory.read_shared_tree()
    }

    pub async fn run(&mut self) {
        println!("Orchestrator staging work to inference queue...");
        
        // Take the configured inference engine
        let engine = std::mem::replace(&mut self.inference, Box::new(MockInferenceEngine));
        let queue = InferenceQueue::new(engine, self.rate_limit.clone());
        
        let agents = self.state.agents.clone();
        let mut requests = Vec::new();

        for agent in &agents {
            println!("Staging work for agent: {} ({})", agent.name, agent.role);
            
            let mut defined_tools = Vec::new();
            for t_name in &agent.tools {
                if let Some(tool) = self.state.tools.iter().find(|t| &t.name == t_name) {
                    defined_tools.push(tool.clone());
                }
            }

            let pending = agent.pending_messages.clone().unwrap_or_default();
            
            // Prioritize overdue scheduled tasks
            let mut overdue_tasks = Vec::new();
            if let Some(tasks) = &agent.scheduled_tasks {
                for task in tasks {
                    if !task.completed {
                        if let Ok(due) = chrono::DateTime::parse_from_rfc3339(&task.due_date) {
                            if chrono::Utc::now() > due.with_timezone(&chrono::Utc) {
                                overdue_tasks.push(task.description.clone());
                            }
                        }
                    }
                }
            }

            let mut prompt_parts = Vec::new();
            if !overdue_tasks.is_empty() {
                prompt_parts.push(format!("URGENT: You have the following scheduled tasks that are OVERDUE and must be prioritized:\n{}", overdue_tasks.join("\n")));
            }
            if !pending.is_empty() {
                prompt_parts.push(format!("Operator messages for this turn:\n{}", pending.iter().map(|m| format!("- {}", m)).collect::<Vec<_>>().join("\n")));
            }
            
            let user_prompt = if prompt_parts.is_empty() {
                DEFAULT_USER_PROMPT.to_string()
            } else {
                format!("{}\n\n{}", prompt_parts.join("\n\n"), DEFAULT_USER_PROMPT)
            };

            let context = self.agent_contexts.entry(agent.id.clone()).or_default();
            for message in &pending {
                context.push_turn("operator", message.clone());
            }

            // Clear pending messages from state
            if !pending.is_empty() {
                let mut updated_agent = agent.clone();
                updated_agent.pending_messages = Some(Vec::new());
                let _ = self.state.update_agent(&agent.id, updated_agent).await;
            }

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
                    // Update telemetry
                    if let Some(agent) = self.state.agents.iter().find(|a| a.id == agent_id).cloned() {
                        let mut updated_agent = agent.clone();
                        let mut tel = updated_agent.telemetry.unwrap_or_default();
                        tel.prompt_tokens += res.stats.prompt_tokens;
                        tel.completion_tokens += res.stats.completion_tokens;
                        tel.total_tokens += res.stats.total_tokens;
                        tel.turns_taken += 1;
                        updated_agent.telemetry = Some(tel);
                        let _ = self.state.update_agent(&agent_id, updated_agent).await;
                    }

                    match &res.result {
                        InferenceResult::Text(text) => {
                            println!("Agent {} Response (Text): {}", agent_id, text);
                            self.agent_contexts.entry(agent_id.clone()).or_default().push_turn("agent", text.clone());
                        }
                        InferenceResult::ToolCalls(calls) => {
                            let summary = format!("Called tools: {}", calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(", "));
                            self.agent_contexts.entry(agent_id.clone()).or_default().push_turn("agent", summary);
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
                                } else if call.name == "write_shared_file" {
                                    let path = call.args["path"].as_str().unwrap_or("output.txt");
                                    let content = call.args["content"].as_str().unwrap_or_default();
                                    if let Err(e) = self.memory.write_shared(path, content) {
                                        println!("Failed to write shared file: {}", e);
                                    } else {
                                        println!("Successfully wrote shared file: {}", path);
                                    }
                                } else {
                                    if let Some(ref executor) = self.tool_executor {
                                        match executor.execute(&agent_id, call).await {
                                            Ok(res) => println!("Custom tool {} executed: {}", call.name, res),
                                            Err(e) => println!("Custom tool {} failed: {}", call.name, e),
                                        }
                                    } else {
                                        println!("Unknown tool: {}", call.name);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Inference Error for Agent {}: {}", agent_id, e);
                    self.agent_contexts.entry(agent_id.clone()).or_default().push_turn("agent", format!("Error: {}", e));
                }
            }
        }

        println!("DFS/Batch Loop complete. Engine resting.");
    }
}
