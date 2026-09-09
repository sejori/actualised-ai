use crate::state::{CompanyState, Project};
use crate::inference::{InferenceEngine, MockInferenceEngine, InferenceResponse, Tool};
use crate::memory::MemoryManager;
use petgraph::graph::DiGraph;

pub struct Orchestrator {
    pub state: CompanyState,
    pub task_graph: DiGraph<String, ()>,
    pub inference: Box<dyn InferenceEngine>,
    pub memory: MemoryManager,
}

impl Orchestrator {
    pub fn new(state: CompanyState, memory: MemoryManager) -> Self {
        Self {
            state,
            task_graph: DiGraph::new(),
            inference: Box::new(MockInferenceEngine),
            memory,
        }
    }

    pub async fn run(&mut self) {
        println!("Orchestrator starting DFS loop...");
        
        let agents = self.state.agents.clone();
        for agent in agents {
            println!("Waking up agent: {} ({})", agent.name, agent.role);
            
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

            let response = self.inference.generate_response(
                &agent.system_prompt,
                "What actions will you take on your assigned tasks?",
                defined_tools
            ).await;

            match response {
                Ok(InferenceResponse::Text(text)) => {
                    println!("Agent Response (Text): {}", text);
                }
                Ok(InferenceResponse::ToolCalls(calls)) => {
                    for call in calls {
                        println!("Agent called tool: {}", call.name);
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
                            if let Err(e) = self.memory.write_memory(&agent.id, file_name, content) {
                                println!("Failed to write memory: {}", e);
                            } else {
                                println!("Successfully wrote memory file: {}", file_name);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Inference Error: {}", e);
                }
            }
        }
        println!("DFS Loop complete. Engine resting.");
    }
}
