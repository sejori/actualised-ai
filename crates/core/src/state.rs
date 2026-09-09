use serde::{Deserialize, Serialize};
use crate::inference::Tool;

#[cfg(not(target_arch = "wasm32"))]
use surrealdb::Surreal;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::engine::local::{Db, SurrealKV};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub parent_id: Option<String>,
    pub system_prompt: String,
    pub tools: Vec<String>,
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
    pub db: Surreal<Db>,
    pub agents: Vec<Agent>,
    pub projects: Vec<Project>,
    pub tools: Vec<Tool>,
    pub shared_files: Vec<SharedFile>,
}

impl CompanyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn init(db_path: &str) -> Result<Self, String> {
        let db = Surreal::new::<SurrealKV>(db_path).await.map_err(|e| e.to_string())?;
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

    pub async fn add_agent(&mut self, agent: Agent) -> Result<(), String> {
        let id = agent.id.clone();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut _res = self.db
                .query("CREATE type::thing('agent', $id) CONTENT $agent")
                .bind(("id", &id))
                .bind(("agent", &agent))
                .await.map_err(|e| e.to_string())?;

            if let Some(parent) = &agent.parent_id {
                let sql = format!("RELATE agent:{}->manages->agent:{}", parent, id);
                let mut _res = self.db.query(sql).await.map_err(|e| e.to_string())?;
            }
        }

        self.agents.push(agent);
        Ok(())
    }

    pub async fn update_agent(&mut self, id: &str, new_agent: Agent) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut _res = self.db
                .query("UPDATE type::thing('agent', $id) CONTENT $agent")
                .bind(("id", id))
                .bind(("agent", &new_agent))
                .await.map_err(|e| e.to_string())?;
            
            // To be perfectly rigorous, we should also update the `manages` edges if parent_id changed,
            // but for simplicity we assume parent_id is immutable for now or handled separately.
        }

        if let Some(idx) = self.agents.iter().position(|a| a.id == id) {
            self.agents[idx] = new_agent;
        }
        Ok(())
    }

    pub async fn remove_agent(&mut self, id: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut _res = self.db
                .query("DELETE type::thing('agent', $id)")
                .bind(("id", id))
                .await.map_err(|e| e.to_string())?;
        }

        self.agents.retain(|a| a.id != id);
        // Cascading deletes of sub-agents omitted for simplicity in this demo
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
