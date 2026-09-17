use crate::state::{CompanyState, Project};
use crate::inference::{build_engine, InferenceConfig, InferenceEngine, InferenceResult, ToolCall};
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

fn root_admin_tools() -> Vec<crate::inference::Tool> {
    use serde_json::json;
    [
        ("inspect_company", "Inspect the company structure, projects, tools, and non-secret inference settings. WARNING: Do NOT call this multiple times. Call it ONCE to gather state, then proceed with your task.", json!({ "type": "object", "properties": {} })),
        ("read_agent_memory", "Read a memory file belonging to any agent.", json!({ "type": "object", "properties": { "agent_id": { "type": "string" }, "file_name": { "type": "string" } }, "required": ["agent_id", "file_name"] })),
        ("read_shared_file", "Read a company shared file.", json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] })),
        ("write_shared_file", "Create or update a company shared file.", json!({ "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } }, "required": ["path", "content"] })),
        ("send_message", "Send a message, assign a task, or hand off work to another agent in the company. This will wake them up to process your request.", json!({ "type": "object", "properties": { "agent_id": { "type": "string" }, "message": { "type": "string" } }, "required": ["agent_id", "message"] })),
        ("start_company", "Enable continuous server-side company execution.", json!({ "type": "object", "properties": {} })),
        ("stop_company", "Pause continuous server-side company execution after this cycle.", json!({ "type": "object", "properties": {} })),
        ("update_inference_settings", "Update the inference provider and model. Existing credentials are preserved.", json!({ "type": "object", "properties": { "provider": { "type": "string", "enum": ["gemini", "openai", "anthropic"] }, "model": { "type": "string" }, "service_tier": { "type": "string" } }, "required": ["provider", "model"] })),
        ("create_agent", "Create a company agent.", json!({ "type": "object", "properties": { "id": { "type": "string" }, "name": { "type": "string" }, "role": { "type": "string" }, "system_prompt": { "type": "string" }, "parent_id": { "type": ["string", "null"] }, "tools": { "type": "array", "items": { "type": "string" } } }, "required": ["id", "name", "role", "system_prompt", "tools"] })),
        ("create_tool", "Create or replace a tool declaration and optionally assign it to the root agent.", json!({ "type": "object", "properties": { "name": { "type": "string" }, "description": { "type": "string" }, "parameters": { "type": "object" }, "assign_to_root": { "type": "boolean" } }, "required": ["name", "description", "parameters"] })),
    ].into_iter().map(|(name, description, parameters)| crate::inference::Tool {
        name: name.to_string(),
        description: description.to_string(),
        parameters,
        company_id: None,
    }).collect()
}

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
    pub inference: Arc<dyn InferenceEngine>,
    pub memory: MemoryManager,
    pub rate_limit: RateLimitConfig,
    pub agent_contexts: HashMap<String, AgentContext>,
    pub tool_executor: Option<Arc<dyn ToolExecutor>>,
}

impl Orchestrator {
    pub fn new(state: CompanyState, memory: MemoryManager) -> Self {
        let settings = state.settings.as_ref();
        let default_inference = build_engine(&InferenceConfig {
            provider: settings.and_then(|value| value.get("provider")).and_then(|value| value.as_str()).unwrap_or("gemini").to_string(),
            model: settings.and_then(|value| value.get("model")).and_then(|value| value.as_str()).unwrap_or_default().to_string(),
            service_tier: settings.and_then(|value| value.get("serviceTier")).and_then(|value| value.as_str()).map(str::to_string),
            api_key: settings.and_then(|value| value.get("apiKey")).and_then(|value| value.as_str()).unwrap_or_default().to_string(),
            telegram_bot_token: None,
        });

        Self {
            state,
            task_graph: DiGraph::new(),
            inference: Arc::from(default_inference),
            memory,
            rate_limit: RateLimitConfig::default(),
            agent_contexts: HashMap::new(),
            tool_executor: None,
        }
    }

    pub fn set_inference(&mut self, engine: Box<dyn InferenceEngine>) {
        self.inference = Arc::from(engine);
    }

    pub fn set_rate_limit(&mut self, rate_limit: RateLimitConfig) {
        self.rate_limit = rate_limit;
    }

    pub fn set_tool_executor(&mut self, executor: Arc<dyn ToolExecutor>) {
        self.tool_executor = Some(executor);
    }

    /// Queues an operator message to be appended to an agent's prompt on its next turn.
    pub async fn queue_message(&mut self, agent_id: &str, message: String) -> Result<(), String> {
        let target_id = agent_id.replace("agent:", "");
        if let Some(agent) = self.state.agents.iter().find(|a| a.id == target_id).cloned() {
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

    async fn execute_root_admin_tool(&mut self, agent_id: &str, call: &ToolCall) -> Option<Result<String, String>> {
        if !root_admin_tools().iter().any(|tool| tool.name == call.name) {
            return None;
        }
        let is_root = self.state.agents.iter().any(|agent| agent.id == agent_id && agent.parent_id.is_none());
        if !is_root {
            return Some(Err("Root administration tools are restricted to the root agent".to_string()));
        }

        let result = match call.name.as_str() {
            "inspect_company" => {
                let settings = self.state.settings.as_ref();
                serde_json::to_string(&serde_json::json!({
                    "name": self.state.company_name,
                    "agents": self.state.agents,
                    "projects": self.state.projects,
                    "tools": self.state.tools.iter().map(|tool| &tool.name).collect::<Vec<_>>(),
                    "inference": {
                        "provider": settings.and_then(|value| value.get("provider")),
                        "model": settings.and_then(|value| value.get("model")),
                        "serviceTier": settings.and_then(|value| value.get("serviceTier")),
                    },
                    "executionEnabled": settings.and_then(|value| value.get("executionEnabled")).and_then(|value| value.as_bool()).unwrap_or(false),
                })).map_err(|error| error.to_string())
            }
            "read_agent_memory" => {
                let target = call.args["agent_id"].as_str().ok_or_else(|| "agent_id is required".to_string());
                let file_name = call.args["file_name"].as_str().ok_or_else(|| "file_name is required".to_string());
                target.and_then(|target| file_name.and_then(|file_name| self.memory.read_memory(target, file_name)))
            }
            "read_shared_file" => call.args["path"].as_str()
                .ok_or_else(|| "path is required".to_string())
                .and_then(|path| self.memory.read_shared(path)),
            "write_shared_file" => {
                let path = call.args["path"].as_str().ok_or_else(|| "path is required".to_string());
                let content = call.args["content"].as_str().ok_or_else(|| "content is required".to_string());
                path.and_then(|path| content.and_then(|content| self.memory.write_shared(path, content).map_err(|error| error.to_string()).map(|_| format!("Updated shared file {}", path))))
            }
            "send_message" => {
                let target_id = call.args["agent_id"].as_str().ok_or_else(|| "agent_id is required".to_string());
                let message = call.args["message"].as_str().ok_or_else(|| "message is required".to_string());
                match (target_id, message) {
                    (Ok(target), Ok(msg)) => {
                        let final_msg = format!("Message from root agent {}: {}", agent_id, msg);
                        self.queue_message(target, final_msg).await.map(|_| format!("Message successfully sent to {}. They will process it on their next turn.", target))
                    }
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            }
            "start_company" | "stop_company" => {
                let enabled = call.name == "start_company";
                self.state.update_company_settings(serde_json::json!({ "executionEnabled": enabled })).await
                    .map(|_| if enabled { "Company execution enabled" } else { "Company execution paused" }.to_string())
            }
            "update_inference_settings" => {
                let provider = call.args["provider"].as_str().ok_or_else(|| "provider is required".to_string());
                let model = call.args["model"].as_str().ok_or_else(|| "model is required".to_string());
                match (provider, model) {
                    (Ok(provider), Ok(model)) => {
                        let mut update = serde_json::json!({ "provider": provider, "model": model });
                        if let Some(service_tier) = call.args["service_tier"].as_str() {
                            update["serviceTier"] = serde_json::json!(service_tier);
                        }
                        self.state.update_company_settings(update).await.map(|_| {
                            let settings = self.state.settings.as_ref();
                            self.set_inference(build_engine(&InferenceConfig {
                                provider: provider.to_string(),
                                model: model.to_string(),
                                service_tier: settings.and_then(|value| value.get("serviceTier")).and_then(|value| value.as_str()).map(str::to_string),
                                api_key: settings.and_then(|value| value.get("apiKey")).and_then(|value| value.as_str()).unwrap_or_default().to_string(),
                                telegram_bot_token: None,
                            }));
                            format!("Inference updated to {}/{}", provider, model)
                        })
                    }
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
            "create_agent" => {
                let agent = serde_json::from_value::<crate::state::Agent>(serde_json::json!({
                    "id": call.args["id"],
                    "name": call.args["name"],
                    "role": call.args["role"],
                    "parent_id": call.args.get("parent_id").cloned().unwrap_or(serde_json::Value::Null),
                    "system_prompt": call.args["system_prompt"],
                    "tools": call.args["tools"],
                    "company_id": null,
                    "telemetry": null,
                    "scheduled_tasks": null,
                    "pending_messages": null,
                    "issue_triggers": null
                })).map_err(|error| error.to_string());
                match agent {
                    Ok(agent) => {
                        let name = agent.name.clone();
                        self.state.add_agent(agent).await.map(|_| format!("Created agent {}", name))
                    }
                    Err(error) => Err(error),
                }
            }
            "create_tool" => {
                let tool = serde_json::from_value::<crate::inference::Tool>(serde_json::json!({
                    "name": call.args["name"],
                    "description": call.args["description"],
                    "parameters": call.args["parameters"],
                    "company_id": null
                })).map_err(|error| error.to_string());
                match tool {
                    Ok(tool) => {
                        let name = tool.name.clone();
                        let mutation = if self.state.tools.iter().any(|existing| existing.name == name) {
                            self.state.update_tool(&name, tool).await
                        } else {
                            self.state.add_tool(tool).await
                        };
                        if mutation.is_ok() && call.args["assign_to_root"].as_bool().unwrap_or(false) {
                            if let Some(mut root) = self.state.agents.iter().find(|agent| agent.id == agent_id).cloned() {
                                if !root.tools.contains(&name) {
                                    root.tools.push(name.clone());
                                    if let Err(error) = self.state.update_agent(agent_id, root).await {
                                        return Some(Err(error));
                                    }
                                }
                            }
                        }
                        mutation.map(|_| format!("Created tool {}", name))
                    }
                    Err(error) => Err(error),
                }
            }
            _ => unreachable!(),
        };
        Some(result)
    }

    pub async fn run(&mut self) -> Result<(), String> {
        println!("Orchestrator staging work to inference queue...");
        let queue = InferenceQueue::new(Arc::clone(&self.inference), self.rate_limit.clone());
        
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
            if agent.parent_id.is_none() {
                for tool in root_admin_tools() {
                    if !defined_tools.iter().any(|defined| defined.name == tool.name) {
                        defined_tools.push(tool);
                    }
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

            let has_issues = agent.issue_triggers.as_ref().map_or(false, |triggers| !triggers.is_empty());

            // If an agent has no new inputs (no messages, no tasks, no issue triggers), it should rest.
            // This prevents idle agents (including root agents) from running in an infinite loop and burning tokens.
            if pending.is_empty() && overdue_tasks.is_empty() && !has_issues {
                continue;
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

        let mut errors = Vec::new();

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
                                if let Some(result) = self.execute_root_admin_tool(&agent_id, call).await {
                                    let message = result.unwrap_or_else(|error| format!("Tool {} failed: {}", call.name, error));
                                    let _ = self.queue_message(&agent_id, message).await;
                                } else if call.name == "create_sub_project" {
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
                                        let mut telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
                                        let mut chat_id = std::env::var("TELEGRAM_CHAT_ID").ok();
                                        
                                        if let Some(settings) = &self.state.settings {
                                            if let Some(t) = settings.get("telegramBotToken").and_then(|v| v.as_str()) {
                                                telegram_token = Some(t.to_string());
                                            }
                                            if let Some(c) = settings.get("telegramChatId").and_then(|v| {
                                                if v.is_number() {
                                                    Some(v.to_string())
                                                } else {
                                                    v.as_str().map(|s| s.to_string())
                                                }
                                            }) {
                                                chat_id = Some(c);
                                            }
                                        }

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
                                        let tool_start = std::time::Instant::now();
                                        println!("Dispatching custom TS tool {} for Agent {}...", call.name, agent_id);
                                        match executor.execute(&agent_id, call).await {
                                            Ok(res) => {
                                                println!("✅ Custom tool {} executed (took {:.2}s)", call.name, tool_start.elapsed().as_secs_f64());
                                                let _ = self.queue_message(&agent_id, res).await;
                                            },
                                            Err(e) => println!("❌ Custom tool {} failed (took {:.2}s): {}", call.name, tool_start.elapsed().as_secs_f64(), e),
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
                    self.agent_contexts.entry(agent_id.clone()).or_default().push_turn("operator", format!("System Error during your last generation attempt: {}. Please try again.", e));
                    errors.push(e.clone());
                }
            }
        }

        println!("DFS/Batch Loop complete. Engine resting.");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::{InferenceResponse, InferenceStats};
    use crate::state::Agent;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn agent(id: &str, parent_id: Option<&str>) -> Agent {
        Agent {
            company_id: None,
            id: id.to_string(),
            name: id.to_string(),
            role: "Test".to_string(),
            parent_id: parent_id.map(str::to_string),
            system_prompt: "Test".to_string(),
            tools: Vec::new(),
            telemetry: None,
            scheduled_tasks: None,
            pending_messages: None,
            issue_triggers: None,
        }
    }

    async fn orchestrator() -> Orchestrator {
        let mut state = CompanyState::init("mem://", None, None).await.unwrap();
        state.add_agent(agent("root", None)).await.unwrap();
        state.add_agent(agent("child", Some("root"))).await.unwrap();
        state.settings = Some(json!({ "provider": "gemini", "apiKey": "secret", "executionEnabled": false }));
        let path = std::env::temp_dir().join(format!("actualised-orchestrator-test-{}", uuid::Uuid::new_v4()));
        let memory = MemoryManager::new(path).unwrap();
        memory.setup_agent_dir("root").unwrap();
        memory.setup_agent_dir("child").unwrap();
        Orchestrator::new(state, memory)
    }

    fn call(name: &str, args: serde_json::Value) -> ToolCall {
        ToolCall { id: format!("call-{name}"), name: name.to_string(), args }
    }

    struct CountingInferenceEngine(Arc<AtomicUsize>);

    #[async_trait]
    impl InferenceEngine for CountingInferenceEngine {
        async fn generate_response(&self, _system_prompt: &str, _user_prompt: &str, _tools: Vec<crate::inference::Tool>) -> Result<InferenceResponse, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(InferenceResponse {
                result: InferenceResult::Text("configured response".to_string()),
                stats: InferenceStats { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
            })
        }
    }

    #[test]
    fn root_admin_tool_library_contains_expected_capabilities() {
        let names = root_admin_tools().into_iter().map(|tool| tool.name).collect::<Vec<_>>();
        assert!(names.contains(&"inspect_company".to_string()));
        assert!(names.contains(&"start_company".to_string()));
        assert!(names.contains(&"stop_company".to_string()));
        assert!(names.contains(&"read_agent_memory".to_string()));
        assert!(names.contains(&"create_agent".to_string()));
        assert!(names.contains(&"create_tool".to_string()));
    }

    #[tokio::test]
    async fn root_admin_tools_inspect_without_secrets_and_restrict_children() {
        let mut orchestrator = orchestrator().await;
        let inspection = orchestrator.execute_root_admin_tool("root", &call("inspect_company", json!({}))).await.unwrap().unwrap();
        assert!(inspection.contains("gemini"));
        assert!(!inspection.contains("secret"));

        let denied = orchestrator.execute_root_admin_tool("child", &call("stop_company", json!({}))).await.unwrap().unwrap_err();
        assert!(denied.contains("restricted"));
    }

    #[tokio::test]
    async fn root_admin_tools_manage_lifecycle_and_memory() {
        let mut orchestrator = orchestrator().await;
        orchestrator.memory.write_memory("child", "notes.md", "child notes").unwrap();

        let memory = orchestrator.execute_root_admin_tool("root", &call("read_agent_memory", json!({ "agent_id": "child", "file_name": "notes.md" }))).await.unwrap().unwrap();
        assert_eq!(memory, "child notes");

        orchestrator.execute_root_admin_tool("root", &call("write_shared_file", json!({ "path": "plans/roadmap.md", "content": "roadmap" }))).await.unwrap().unwrap();
        let shared = orchestrator.execute_root_admin_tool("root", &call("read_shared_file", json!({ "path": "plans/roadmap.md" }))).await.unwrap().unwrap();
        assert_eq!(shared, "roadmap");

        orchestrator.execute_root_admin_tool("root", &call("start_company", json!({}))).await.unwrap().unwrap();
        assert_eq!(orchestrator.state.settings.as_ref().unwrap()["executionEnabled"], true);
        orchestrator.execute_root_admin_tool("root", &call("stop_company", json!({}))).await.unwrap().unwrap();
        assert_eq!(orchestrator.state.settings.as_ref().unwrap()["executionEnabled"], false);
    }

    #[tokio::test]
    async fn root_admin_tools_create_agents_and_tools() {
        let mut orchestrator = orchestrator().await;
        orchestrator.execute_root_admin_tool("root", &call("create_tool", json!({
            "name": "search_docs",
            "description": "Search documentation",
            "parameters": { "type": "object" },
            "assign_to_root": true
        }))).await.unwrap().unwrap();
        assert!(orchestrator.state.tools.iter().any(|tool| tool.name == "search_docs"));
        assert!(orchestrator.state.agents.iter().find(|agent| agent.id == "root").unwrap().tools.contains(&"search_docs".to_string()));

        orchestrator.execute_root_admin_tool("root", &call("create_agent", json!({
            "id": "researcher",
            "name": "Researcher",
            "role": "Research",
            "system_prompt": "Research carefully",
            "parent_id": "root",
            "tools": ["search_docs"]
        }))).await.unwrap().unwrap();
        assert!(orchestrator.state.agents.iter().any(|agent| agent.id.ends_with("researcher")));
    }

    #[tokio::test]
    async fn configured_inference_engine_is_reused_across_cycles() {
        let mut orchestrator = orchestrator().await;
        let calls = Arc::new(AtomicUsize::new(0));
        orchestrator.set_inference(Box::new(CountingInferenceEngine(Arc::clone(&calls))));

        orchestrator.run().await.unwrap();
        orchestrator.run().await.unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert!(orchestrator.agent_contexts.values().all(|context| {
            context.history.iter().any(|turn| turn.content == "configured response")
        }));
    }
}
