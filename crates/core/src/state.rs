use serde::{Deserialize, Serialize};
use crate::inference::Tool;

#[cfg(not(target_arch = "wasm32"))]
use surrealdb::Surreal;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::engine::any::{connect, Any};
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::opt::auth::Root;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AgentTelemetry {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub turns_taken: usize,
    pub projects_completed: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledTask {
    pub id: String,
    pub description: String,
    pub due_date: String,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub parent_id: Option<String>,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub telemetry: Option<AgentTelemetry>,
    pub scheduled_tasks: Option<Vec<ScheduledTask>>,
    pub pending_messages: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SharedFile {
    pub id: String,
    pub name: String,
    pub content: String,
}

/// Abstract representation of the company state (SurrealDB or in-memory)
pub struct CompanyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub db: Surreal<Any>,
    pub agents: Vec<Agent>,
    pub projects: Vec<Project>,
    pub tools: Vec<Tool>,
    pub shared_files: Vec<SharedFile>,
}

impl CompanyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn init(db_path: &str) -> Result<Self, String> {
        let url = if db_path.contains("://") {
            db_path.to_string()
        } else {
            format!("surrealkv://{}", db_path)
        };

        let db = connect(&url).await.map_err(|e| e.to_string())?;

        if let (Ok(user), Ok(pass)) = (std::env::var("SURREALDB_USER"), std::env::var("SURREALDB_PASS")) {
            db.signin(Root {
                username: &user,
                password: &pass,
            }).await.map_err(|e| e.to_string())?;
        }

        db.use_ns("actualised").use_db("core").await.map_err(|e| e.to_string())?;
        
        Ok(Self {
            db,
            agents: Vec::new(),
            projects: Vec::new(),
            tools: Vec::new(),
            shared_files: Vec::new(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn init(_db_path: &str) -> Result<Self, String> {
        Ok(Self {
            agents: Vec::new(),
            projects: Vec::new(),
            tools: Vec::new(),
            shared_files: Vec::new(),
        })
    }

    // --- Agents ---

    pub async fn add_agent(&mut self, mut agent: Agent) -> Result<(), String> {
        let mut updates_needed = Vec::new();
        let id = agent.id.clone();
        
        if self.agents.is_empty() {
            agent.parent_id = None;
        } else if agent.parent_id.is_none() {
            for a in self.agents.iter_mut() {
                if a.parent_id.is_none() {
                    a.parent_id = Some(id.clone());
                    updates_needed.push(a.clone());
                }
            }
        }

        self.agents.push(agent.clone());
        updates_needed.push(agent);

        #[cfg(not(target_arch = "wasm32"))]
        for ag in updates_needed {
            let mut _res = self.db
                .query("UPDATE type::thing('agent', $id) CONTENT $agent")
                .bind(("id", &ag.id))
                .bind(("agent", &ag))
                .await.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn update_agent(&mut self, id: &str, mut new_agent: Agent) -> Result<(), String> {
        let mut updates_needed = Vec::new();
        let was_root = self.agents.iter().find(|a| a.id == id).map(|a| a.parent_id.is_none()).unwrap_or(false);

        if new_agent.parent_id.is_none() {
            for a in self.agents.iter_mut() {
                if a.id != id && a.parent_id.is_none() {
                    a.parent_id = Some(id.to_string());
                    updates_needed.push(a.clone());
                }
            }
        } else if was_root {
            let manager_id = new_agent.parent_id.as_ref().unwrap().clone();
            for a in self.agents.iter_mut() {
                if a.id == manager_id {
                    a.parent_id = None;
                    updates_needed.push(a.clone());
                }
            }
        }

        if let Some(idx) = self.agents.iter().position(|a| a.id == id) {
            self.agents[idx] = new_agent.clone();
            updates_needed.push(new_agent);
        }

        #[cfg(not(target_arch = "wasm32"))]
        for ag in updates_needed {
            let mut _res = self.db
                .query("UPDATE type::thing('agent', $id) CONTENT $agent")
                .bind(("id", &ag.id))
                .bind(("agent", &ag))
                .await.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn remove_agent(&mut self, id: &str) -> Result<(), String> {
        let is_root = self.agents.iter().find(|a| a.id == id).map(|a| a.parent_id.is_none()).unwrap_or(false);
        let parent_id = self.agents.iter().find(|a| a.id == id).and_then(|a| a.parent_id.clone());
        let mut updates_needed = Vec::new();

        if is_root {
            let next_root = self.agents.iter().find(|a| a.parent_id.as_deref() == Some(id)).map(|a| a.id.clone())
                .or_else(|| self.agents.iter().find(|a| a.id != id).map(|a| a.id.clone()));
                
            if let Some(next_root_id) = next_root {
                for a in self.agents.iter_mut() {
                    if a.id == next_root_id {
                        a.parent_id = None;
                        updates_needed.push(a.clone());
                    } else if a.parent_id.as_deref() == Some(id) {
                        a.parent_id = Some(next_root_id.clone());
                        updates_needed.push(a.clone());
                    }
                }
            }
        } else {
            for a in self.agents.iter_mut() {
                if a.parent_id.as_deref() == Some(id) {
                    a.parent_id = parent_id.clone();
                    updates_needed.push(a.clone());
                }
            }
        }

        self.agents.retain(|a| a.id != id);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut _res = self.db
                .query("DELETE type::thing('agent', $id)")
                .bind(("id", id))
                .await.map_err(|e| e.to_string())?;

            for ag in updates_needed {
                let mut _res = self.db
                    .query("UPDATE type::thing('agent', $id) CONTENT $agent")
                    .bind(("id", &ag.id))
                    .bind(("agent", &ag))
                    .await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    // --- Projects ---

    pub async fn add_project(&mut self, project: Project) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut _res = self.db
                .query("CREATE type::thing('project', $id) CONTENT $project")
                .bind(("id", &project.id))
                .bind(("project", &project))
                .await.map_err(|e| e.to_string())?;
        }
            
        self.projects.push(project);
        Ok(())
    }

    // --- Tools ---

    pub async fn add_tool(&mut self, tool: Tool) -> Result<(), String> {
        // In a real DB we'd store these. For now, in-memory array is synced.
        self.tools.push(tool);
        Ok(())
    }

    pub async fn update_tool(&mut self, name: &str, new_tool: Tool) -> Result<(), String> {
        if let Some(idx) = self.tools.iter().position(|t| t.name == name) {
            self.tools[idx] = new_tool;
        }
        Ok(())
    }

    pub async fn remove_tool(&mut self, name: &str) -> Result<(), String> {
        self.tools.retain(|t| t.name != name);
        Ok(())
    }

    // --- Shared Files ---

    pub async fn add_shared_file(&mut self, file: SharedFile) -> Result<(), String> {
        self.shared_files.push(file);
        Ok(())
    }

    pub async fn remove_shared_file(&mut self, id: &str) -> Result<(), String> {
        self.shared_files.retain(|f| f.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_agent(id: &str, parent_id: Option<&str>) -> Agent {
        Agent {
            id: id.to_string(),
            name: "Test Agent".to_string(),
            role: "Role".to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            system_prompt: "Prompt".to_string(),
            tools: vec![],
            telemetry: None,
            scheduled_tasks: None,
            pending_messages: None,
        }
    }

    #[tokio::test]
    async fn test_first_agent_is_root() {
        let mut state = CompanyState {
            #[cfg(not(target_arch = "wasm32"))]
            db: surrealdb::engine::any::connect("mem://").await.unwrap(),
            agents: vec![],
            projects: vec![],
            tools: vec![],
            shared_files: vec![],
        };

        // Added with a parent, but it's the first agent, so it should be forced to root
        let agent1 = create_agent("agent1", Some("some_parent"));
        state.add_agent(agent1.clone()).await.unwrap();

        assert_eq!(state.agents.len(), 1);
        assert_eq!(state.agents[0].parent_id, None);
    }

    #[tokio::test]
    async fn test_add_second_root_swaps_first() {
        let mut state = CompanyState {
            #[cfg(not(target_arch = "wasm32"))]
            db: surrealdb::engine::any::connect("mem://").await.unwrap(),
            agents: vec![],
            projects: vec![],
            tools: vec![],
            shared_files: vec![],
        };

        state.add_agent(create_agent("agent1", None)).await.unwrap();
        // Add a second root
        state.add_agent(create_agent("agent2", None)).await.unwrap();

        assert_eq!(state.agents.len(), 2);
        
        let agent1 = state.agents.iter().find(|a| a.id == "agent1").unwrap();
        let agent2 = state.agents.iter().find(|a| a.id == "agent2").unwrap();

        // agent2 should be the new root, agent1 should report to agent2
        assert_eq!(agent2.parent_id, None);
        assert_eq!(agent1.parent_id, Some("agent2".to_string()));
    }

    #[tokio::test]
    async fn test_update_agent_to_root_swaps_current_root() {
        let mut state = CompanyState {
            #[cfg(not(target_arch = "wasm32"))]
            db: surrealdb::engine::any::connect("mem://").await.unwrap(),
            agents: vec![],
            projects: vec![],
            tools: vec![],
            shared_files: vec![],
        };

        state.add_agent(create_agent("root", None)).await.unwrap();
        state.add_agent(create_agent("child", Some("root"))).await.unwrap();

        // Make child the root
        let updated_child = create_agent("child", None);
        state.update_agent("child", updated_child).await.unwrap();

        let old_root = state.agents.iter().find(|a| a.id == "root").unwrap();
        let new_root = state.agents.iter().find(|a| a.id == "child").unwrap();

        assert_eq!(new_root.parent_id, None);
        assert_eq!(old_root.parent_id, Some("child".to_string()));
    }

    #[tokio::test]
    async fn test_update_root_with_manager_promotes_manager() {
        let mut state = CompanyState {
            #[cfg(not(target_arch = "wasm32"))]
            db: surrealdb::engine::any::connect("mem://").await.unwrap(),
            agents: vec![],
            projects: vec![],
            tools: vec![],
            shared_files: vec![],
        };

        state.add_agent(create_agent("root", None)).await.unwrap();
        state.add_agent(create_agent("manager", Some("root"))).await.unwrap();

        // Give the root a manager
        let updated_root = create_agent("root", Some("manager"));
        state.update_agent("root", updated_root).await.unwrap();

        let old_root = state.agents.iter().find(|a| a.id == "root").unwrap();
        let new_root = state.agents.iter().find(|a| a.id == "manager").unwrap();

        assert_eq!(new_root.parent_id, None);
        assert_eq!(old_root.parent_id, Some("manager".to_string()));
    }

    #[tokio::test]
    async fn test_remove_root_promotes_child() {
        let mut state = CompanyState {
            #[cfg(not(target_arch = "wasm32"))]
            db: surrealdb::engine::any::connect("mem://").await.unwrap(),
            agents: vec![],
            projects: vec![],
            tools: vec![],
            shared_files: vec![],
        };

        state.add_agent(create_agent("root", None)).await.unwrap();
        state.add_agent(create_agent("child1", Some("root"))).await.unwrap();
        state.add_agent(create_agent("child2", Some("root"))).await.unwrap();

        state.remove_agent("root").await.unwrap();

        assert_eq!(state.agents.len(), 2);
        
        let new_root = state.agents.iter().find(|a| a.parent_id.is_none()).unwrap();
        let other_child = state.agents.iter().find(|a| a.id != new_root.id).unwrap();

        // One of the children must be promoted to root, and the other must report to it
        assert_eq!(new_root.parent_id, None);
        assert_eq!(other_child.parent_id, Some(new_root.id.clone()));
    }

    #[tokio::test]
    async fn test_remove_intermediate_node_orphans_to_grandparent() {
        let mut state = CompanyState {
            #[cfg(not(target_arch = "wasm32"))]
            db: surrealdb::engine::any::connect("mem://").await.unwrap(),
            agents: vec![],
            projects: vec![],
            tools: vec![],
            shared_files: vec![],
        };

        state.add_agent(create_agent("root", None)).await.unwrap();
        state.add_agent(create_agent("middle", Some("root"))).await.unwrap();
        state.add_agent(create_agent("leaf", Some("middle"))).await.unwrap();

        state.remove_agent("middle").await.unwrap();

        assert_eq!(state.agents.len(), 2);
        
        let leaf = state.agents.iter().find(|a| a.id == "leaf").unwrap();
        assert_eq!(leaf.parent_id, Some("root".to_string()));
    }
}
