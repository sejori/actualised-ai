use serde::{Deserialize, Serialize};

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

/// Abstract representation of the company state (SurrealDB or in-memory)
pub struct CompanyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub db: Surreal<Db>,
    pub agents: Vec<Agent>,
    pub projects: Vec<Project>,
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
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn init(_db_path: &str) -> Result<Self, String> {
        Ok(Self {
            agents: Vec::new(),
            projects: Vec::new(),
        })
    }

    pub async fn add_agent(&mut self, agent: Agent) -> Result<(), String> {
        let id = agent.id.clone();
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Insert into SurrealDB
            let mut _res = self.db
                .query("CREATE type::thing('agent', $id) CONTENT $agent")
                .bind(("id", &id))
                .bind(("agent", &agent))
                .await.map_err(|e| e.to_string())?;

            // Map edge if parent_id exists
            if let Some(parent) = &agent.parent_id {
                let sql = format!("RELATE agent:{}->manages->agent:{}", parent, id);
                let mut _res = self.db
                    .query(sql)
                    .await.map_err(|e| e.to_string())?;
            }
        }

        self.agents.push(agent);
        Ok(())
    }

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
}
