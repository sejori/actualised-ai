import re

with open("crates/core/src/orchestrator.rs", "r", encoding="utf-8") as f:
    c = f.read()

# 1. Prompt Logic
old_prompt = """            let mut prompt_parts = Vec::new();
            if !overdue_tasks.is_empty() {
                prompt_parts.push(format!("URGENT: You have the following scheduled tasks that are OVERDUE and must be prioritized:\\n{}", overdue_tasks.join("\\n")));
            }"""
new_prompt = """            let mut prompt_parts = Vec::new();
            
            if let Some(issue_triggers) = &agent.issue_triggers {
                if !issue_triggers.is_empty() {
                    let mut issues_context = String::new();
                    for issue_id in issue_triggers {
                        if let Some(issue) = self.state.issues.iter().find(|i| &i.id == issue_id) {
                            issues_context.push_str(&format!("\\nIssue [{}]: {}\\nState: {}\\nBody: {}\\n", issue.id, issue.title, issue.state, issue.body));
                            if !issue.comments.is_empty() {
                                issues_context.push_str("Comments:\\n");
                                for comment in &issue.comments {
                                    issues_context.push_str(&format!("- {}: {}\\n", comment.author, comment.content));
                                }
                            }
                        }
                    }
                    if !issues_context.is_empty() {
                        prompt_parts.push(format!("You have been triggered to resolve the following issues:\\n{}", issues_context));
                    }
                }
            }

            if !overdue_tasks.is_empty() {
                prompt_parts.push(format!("URGENT: You have the following scheduled tasks that are OVERDUE and must be prioritized:\\n{}", overdue_tasks.join("\\n")));
            }"""
c = c.replace(old_prompt, new_prompt)

# 2. Add System Prompt dynamic replacement
c = c.replace("""            requests.push(InferenceRequest {
                agent_id: agent.id.clone(),
                system_prompt: agent.system_prompt.clone(),
                user_prompt,
                tools: defined_tools,
            });""", """            let mut final_system_prompt = agent.system_prompt.clone();
            if let Some(triggers) = &agent.issue_triggers {
                if !triggers.is_empty() {
                    final_system_prompt = format!("{}\\n\\nFOCUS: You are currently working on GitHub issues. Use your tools to investigate, plan, and resolve them.", agent.system_prompt);
                }
            }
            
            requests.push(InferenceRequest {
                agent_id: agent.id.clone(),
                system_prompt: final_system_prompt,
                user_prompt,
                tools: defined_tools,
            });""")

# 3. Handle Tool Execution
old_tools = """                                } else if call.name == "write_shared_file" {
                                    let path = call.args["path"].as_str().unwrap_or("output.txt");
                                    let content = call.args["content"].as_str().unwrap_or_default();
                                    if let Err(e) = self.memory.write_shared(path, content) {
                                        println!("Failed to write shared file: {}", e);
                                    } else {
                                        println!("Successfully wrote shared file: {}", path);
                                    }
                                } else {"""
new_tools = """                                } else if call.name.starts_with("github_") {
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        use crate::vcs::{VersionControl, IssueTracker};
                                        if let Some(repo) = &self.state.repository {
                                            if let Ok(gh) = crate::providers::github::GithubProvider::new(repo.repo_url.clone(), repo.access_token.clone()) {
                                                if call.name == "github_read_file" {
                                                    let path = call.args["path"].as_str().unwrap_or_default();
                                                    let msg = match gh.read_file(path).await {
                                                        Ok(content) => format!("File {} content:\\n{}", path, content),
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
                                } else {"""
c = c.replace(old_tools, new_tools)

with open("crates/core/src/orchestrator.rs", "w", encoding="utf-8") as f:
    f.write(c)

