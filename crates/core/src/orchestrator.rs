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
    pub pending_messages: Vec<String>,
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
        let mut context = self.agent_contexts.get(agent_id).cloned().unwrap_or_default();
        context.pending_messages = self.state.agents.iter()
            .find(|agent| agent.id == agent_id)
            .and_then(|agent| agent.pending_messages.clone())
            .unwrap_or_default();
        context
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
            
            if let Some(issue_triggers) = &agent.issue_triggers {
                if !issue_triggers.is_empty() {
                    let mut issues_context = String::new();
                    for issue_id in issue_triggers {
                        if let Some(issue) = self.state.issues.iter().find(|i| &i.id == issue_id) {
                            issues_context.push_str(&format!("\nIssue [{}]: {}\nState: {}\nBody: {}\n", issue.id, issue.title, issue.state, issue.body));
                            if !issue.comments.is_empty() {
                                issues_context.push_str("Comments:\n");
                                for comment in &issue.comments {
                                    issues_context.push_str(&format!("- {}: {}\n", comment.author, comment.content));
                                }
                            }
                        }
                    }
                    if !issues_context.is_empty() {
                        prompt_parts.push(format!("FOCUS: You are currently working on GitHub issues. Use your tools to investigate, plan, and resolve them.\n\nYou have been triggered to resolve the following issues:\n{}", issues_context));
                    }
                }
            }

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

            let mut final_system_prompt = agent.system_prompt.clone();
            
            // INJECT MEMORY INDEX for long-term retention & caching efficiency
            if let Ok(index_content) = self.memory.read_memory(&agent.id, "index.md") {
                final_system_prompt = format!("{}

=== Core Memory (index.md) ===
{}", final_system_prompt, index_content);
            }
            
            requests.push(InferenceRequest {
                agent_id: agent.id.clone(),
                system_prompt: final_system_prompt,
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
                                        description, company_id: None,
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
                                } else if call.name == "read_memory" {
                                    let file_name = call.args["file_name"].as_str().unwrap_or("index.md");
                                    let msg = match self.memory.read_memory(&agent_id, file_name) {
                                        Ok(content) => format!("Memory file {} content:\n{}", file_name, content),
                                        Err(e) => format!("Failed to read memory file: {}", e)
                                    };
                                    let _ = self.queue_message(&agent_id, msg).await;
                                } else if call.name == "telegram_notify" {
                                    let message = call.args["message"].as_str().unwrap_or_default();
                                    
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
                                        let chat_id = std::env::var("TELEGRAM_CHAT_ID").ok();
                                        let dispatcher = crate::tools::notifications::NotificationDispatcher::new(telegram_token, chat_id);
                                        let msg = match dispatcher.send_notification(message).await {
                                            Ok(_) => "Successfully sent telegram notification.".to_string(),
                                            Err(e) => format!("Failed to send telegram notification: {}", e)
                                        };
                                        let _ = self.queue_message(&agent_id, msg).await;
                                    }
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let _ = self.queue_message(&agent_id, "Telegram notifications are not supported in WASM sandbox.".to_string()).await;
                                    }
                                } else if call.name.starts_with("github_") {
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        use crate::vcs::{VersionControl, IssueTracker};
                                        if let Some(repo) = &self.state.repository {
                                            if let Ok(gh) = crate::providers::github::GithubProvider::new(repo.repo_url.clone(), repo.access_token.clone()) {
                                                if call.name == "github_read_file" {
                                                    let path = call.args["path"].as_str().unwrap_or_default();
                                                    let msg = match gh.read_file(path).await {
                                                        Ok(content) => format!("File {} content:\n{}", path, content),
                                                        Err(e) => format!("Failed to read file: {}", e)
                                                    };
                                                    let _ = self.queue_message(&agent_id, msg).await;
                                                } else if call.name == "github_write_file" {
                                                    let path = call.args["path"].as_str().unwrap_or_default();
                                                    let content = call.args["content"].as_str().unwrap_or_default();
                                                    let message = call.args["message"].as_str().unwrap_or_default();
                                                    let msg = match gh.write_file(path, content, message).await {
                                                        Ok(_) => format!("Successfully wrote to {} and committed.", path),
                                                        Err(e) => format!("Failed to write file: {}", e)
                                                    };
                                                    let _ = self.queue_message(&agent_id, msg).await;
                                                } else if call.name == "github_comment_issue" {
                                                    let issue_id = call.args["issue_id"].as_str().unwrap_or_default();
                                                    let comment = call.args["comment"].as_str().unwrap_or_default();
                                                    let msg = match gh.add_comment(issue_id, comment).await {
                                                        Ok(_) => format!("Successfully commented on issue {}.", issue_id),
                                                        Err(e) => format!("Failed to comment: {}", e)
                                                    };
                                                    let _ = self.queue_message(&agent_id, msg).await;
                                                } else {
                                                    let _ = self.queue_message(&agent_id, format!("Unknown GitHub tool: {}", call.name)).await;
                                                }
                                            } else {
                                                let _ = self.queue_message(&agent_id, "Failed to initialize GitHub provider.".to_string()).await;
                                            }
                                        } else {
                                            let _ = self.queue_message(&agent_id, "No GitHub repository configured for this company.".to_string()).await;
                                        }
                                    }
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let _ = self.queue_message(&agent_id, "GitHub tools are not supported in the WASM sandbox environment.".to_string()).await;
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
