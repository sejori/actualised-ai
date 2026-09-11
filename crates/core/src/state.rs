use serde::{Deserialize, Serialize};
use crate::inference::Tool;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb_types::{RecordId, SurrealValue};

#[cfg(not(target_arch = "wasm32"))]
use surrealdb::Surreal;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::engine::any::{connect, Any};
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::opt::auth::Root;
#[cfg(not(target_arch = "wasm32"))]
use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
async fn with_database_timeout<T, E>(phase: &str, future: impl Future<Output = Result<T, E>>) -> Result<T, String>
where
    E: std::fmt::Display,
{
    eprintln!("Starting SurrealDB {phase}");
    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(15), future)
        .await
        .map_err(|_| format!("SurrealDB {phase} timed out after 15 seconds"))?
        .map_err(|error| format!("SurrealDB {phase} failed: {error}"));
    eprintln!("Completed SurrealDB {phase} in {} ms", started.elapsed().as_millis());
    result
}

#[cfg(not(target_arch = "wasm32"))]
fn record_content<T: Serialize>(value: &T) -> Result<serde_json::Value, String> {
    let mut content = serde_json::to_value(value).map_err(|e| e.to_string())?;
    if let serde_json::Value::Object(fields) = &mut content {
        fields.remove("id");
    }
    Ok(content)
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct AgentTelemetry {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub turns_taken: usize,
    pub projects_completed: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct ScheduledTask {
    pub id: String,
    pub description: String,
    pub due_date: String,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct Agent {
    pub company_id: Option<String>,
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
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct Project {
    pub company_id: Option<String>,
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(SurrealValue))]
pub struct SharedFile {
    pub company_id: Option<String>,
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
    pub company_name: Option<String>,
    pub company_id: Option<String>,
}

impl CompanyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn init(db_path: &str, token: Option<String>) -> Result<Self, String> {
        let url = if db_path.contains("://") {
            db_path.to_string()
        } else {
            format!("surrealkv://{}", db_path)
        };
        let db = with_database_timeout("connection", async { connect(&url).await }).await?;

        if let Some(tok) = token {
            with_database_timeout("token authentication", async {
                db.authenticate(tok).await
            }).await?;
        } else if let (Ok(user), Ok(pass)) = (std::env::var("SURREALDB_USER"), std::env::var("SURREALDB_PASS")) {
            with_database_timeout("authentication", async {
                db.signin(Root {
                    username: user,
                    password: pass,
                }).await
            }).await?;
        }

        with_database_timeout("namespace selection", async {
            db.use_ns("actualised").use_db("core").await
        }).await?;

        with_database_timeout("schema initialization", async {
            db.query(
                "DEFINE TABLE IF NOT EXISTS user SCHEMALESS PERMISSIONS FOR select, update, delete WHERE id = $auth.id;
                 DEFINE ACCESS user ON DATABASE TYPE RECORD
                    SIGNUP ( CREATE user SET email = $email, pass = crypto::argon2::generate($pass) )
                    SIGNIN ( SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(pass, $pass) )
                    DURATION FOR TOKEN 30d, FOR SESSION 30d;
                 DEFINE TABLE IF NOT EXISTS company SCHEMALESS PERMISSIONS FOR select, update, delete WHERE owner = $auth.id;
                 DEFINE TABLE IF NOT EXISTS agent SCHEMALESS PERMISSIONS FOR select, update, delete WHERE company_id.owner = $auth.id OR company_id = null;
                 DEFINE TABLE IF NOT EXISTS project SCHEMALESS PERMISSIONS FOR select, update, delete WHERE company_id.owner = $auth.id OR company_id = null;
                 DEFINE TABLE IF NOT EXISTS tool SCHEMALESS PERMISSIONS FOR select, update, delete WHERE company_id.owner = $auth.id OR company_id = null;
                 DEFINE TABLE IF NOT EXISTS shared_file SCHEMALESS PERMISSIONS FOR select, update, delete WHERE company_id.owner = $auth.id OR company_id = null;"
            ).await?.check()
        }).await?;

        let mut response = with_database_timeout("state hydration", async {
            db.query(
                "SELECT record::id(id) AS id, name, role, parent_id, system_prompt, tools, telemetry, scheduled_tasks, pending_messages FROM agent;
                 SELECT record::id(id) AS id, title, description FROM project;
                 SELECT name, description, parameters FROM tool;
                 SELECT record::id(id) AS id, name, content FROM shared_file;
                 SELECT record::id(id) AS id, name FROM company LIMIT 1"
            ).await
        }).await?;
        let agents = response.take::<Vec<Agent>>(0).map_err(|e| e.to_string())?;
        let projects = response.take::<Vec<Project>>(1).map_err(|e| e.to_string())?;
        let tools = response.take::<Vec<Tool>>(2).map_err(|e| e.to_string())?;
        let shared_files = response.take::<Vec<SharedFile>>(3).map_err(|e| e.to_string())?;
        let company_docs = response.take::<Vec<serde_json::Value>>(4).map_err(|e| e.to_string())?;
        let company_id = company_docs.first().and_then(|d| d.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()));
        let company_name = company_docs.first().and_then(|d| d.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()));
        
        Ok(Self {
            db,
            agents,
            projects,
            tools,
            shared_files,
            company_name,
            company_id,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn init(_db_path: &str) -> Result<Self, String> {
        Ok(Self {
            agents: Vec::new(),
            projects: Vec::new(),
            tools: Vec::new(),
            shared_files: Vec::new(),
            company_name: None,
            company_id: None,
        })
    }

    pub async fn set_company_name(&mut self, name: String) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let res = self.db
                .query("IF (SELECT VALUE id FROM company LIMIT 1)[0] THEN UPDATE company SET name = $name ELSE CREATE company SET name = $name, owner = $auth.id END;")
                .bind(("name", name.clone()))
                .await.map_err(|e| e.to_string())?
                .check().map_err(|e| e.to_string())?;
            // We should reload company_id if we created it, but the app can just reload
        }

        self.company_name = Some(name);
        Ok(())
    }

    // --- Agents ---

    pub async fn add_agent(&mut self, mut agent: Agent) -> Result<(), String> {
        let mut updates_needed = Vec::new();
        let id = agent.id.clone();
        agent.company_id = self.company_id.clone();
        
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
            let content = record_content(&ag)?;
            let mut _res = self.db
                .query("UPDATE $record CONTENT $agent")
                .bind(("record", RecordId::new("agent", ag.id.clone())))
                .bind(("agent", content))
                .await.map_err(|e| e.to_string())?
                .check().map_err(|e| e.to_string())?;
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
            let content = record_content(&ag)?;
            let mut _res = self.db
                .query("UPDATE $record CONTENT $agent")
                .bind(("record", RecordId::new("agent", ag.id.clone())))
                .bind(("agent", content))
                .await.map_err(|e| e.to_string())?
                .check().map_err(|e| e.to_string())?;
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
                .query("DELETE $record")
                .bind(("record", RecordId::new("agent", id)))
                .await.map_err(|e| e.to_string())?
                .check().map_err(|e| e.to_string())?;

            for ag in updates_needed {
                let content = record_content(&ag)?;
                let mut _res = self.db
                    .query("UPDATE $record CONTENT $agent")
                    .bind(("record", RecordId::new("agent", ag.id.clone())))
                    .bind(("agent", content))
                    .await.map_err(|e| e.to_string())?
                    .check().map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    // --- Projects ---

    pub async fn add_project(&mut self, project: Project) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let content = record_content(&project)?;
            let mut _res = self.db
                .query("CREATE $record CONTENT $project")
                .bind(("record", RecordId::new("project", project.id.clone())))
                .bind(("project", content))
                .await.map_err(|e| e.to_string())?
                .check().map_err(|e| e.to_string())?;
        }
            
        self.projects.push(project);
        Ok(())
    }

    // --- Tools ---

    pub async fn add_tool(&mut self, mut tool: Tool) -> Result<(), String> {
        tool.company_id = self.company_id.clone();
        #[cfg(not(target_arch = "wasm32"))]
        self.db
            .query("UPDATE $record CONTENT $tool")
            .bind(("record", RecordId::new("tool", tool.name.clone())))
            .bind(("tool", tool.clone()))
            .await.map_err(|e| e.to_string())?
            .check().map_err(|e| e.to_string())?;

        self.tools.push(tool);
        Ok(())
    }

    pub async fn update_tool(&mut self, name: &str, new_tool: Tool) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        self.db
            .query("DELETE $old_record; UPDATE $record CONTENT $tool")
            .bind(("old_record", RecordId::new("tool", name)))
            .bind(("record", RecordId::new("tool", new_tool.name.clone())))
            .bind(("tool", new_tool.clone()))
            .await.map_err(|e| e.to_string())?
            .check().map_err(|e| e.to_string())?;

        if let Some(idx) = self.tools.iter().position(|t| t.name == name) {
            self.tools[idx] = new_tool;
        }
        Ok(())
    }

    pub async fn remove_tool(&mut self, name: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        self.db
            .query("DELETE $record")
            .bind(("record", RecordId::new("tool", name)))
            .await.map_err(|e| e.to_string())?
            .check().map_err(|e| e.to_string())?;

        self.tools.retain(|t| t.name != name);
        Ok(())
    }

    // --- Shared Files ---

    pub async fn add_shared_file(&mut self, file: SharedFile) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let content = record_content(&file)?;
            self.db
                .query("UPDATE $record CONTENT $file")
                .bind(("record", RecordId::new("shared_file", file.id.clone())))
                .bind(("file", content))
                .await.map_err(|e| e.to_string())?
                .check().map_err(|e| e.to_string())?;
        }

        self.shared_files.push(file);
        Ok(())
    }

    pub async fn remove_shared_file(&mut self, id: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        self.db
            .query("DELETE $record")
            .bind(("record", RecordId::new("shared_file", id)))
            .await.map_err(|e| e.to_string())?
            .check().map_err(|e| e.to_string())?;

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
            pending_messages: None, company_id: None,
        }
    }

    async fn create_state() -> CompanyState {
        CompanyState::init("mem://").await.unwrap()
    }

    #[tokio::test]
    async fn test_first_agent_is_root() {
        let mut state = create_state().await;

        // Added with a parent, but it's the first agent, so it should be forced to root
        let agent1 = create_agent("agent1", Some("some_parent"));
        state.add_agent(agent1.clone()).await.unwrap();

        assert_eq!(state.agents.len(), 1);
        assert_eq!(state.agents[0].parent_id, None);
    }

    #[tokio::test]
    async fn test_add_second_root_swaps_first() {
        let mut state = create_state().await;

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
        let mut state = create_state().await;

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
        let mut state = create_state().await;

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
        let mut state = create_state().await;

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
        let mut state = create_state().await;

        state.add_agent(create_agent("root", None)).await.unwrap();
        state.add_agent(create_agent("middle", Some("root"))).await.unwrap();
        state.add_agent(create_agent("leaf", Some("middle"))).await.unwrap();

        state.remove_agent("middle").await.unwrap();

        assert_eq!(state.agents.len(), 2);
        
        let leaf = state.agents.iter().find(|a| a.id == "leaf").unwrap();
        assert_eq!(leaf.parent_id, Some("root".to_string()));
    }
}
