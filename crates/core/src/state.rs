use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
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
    pub db: Surreal<Db>,
    pub agents: Vec<Agent>,
    pub projects: Vec<Project>,
}

impl CompanyState {
    pub async fn init(db_path: &str) -> surrealdb::Result<Self> {
        let db = Surreal::new::<SurrealKV>(db_path).await?;
        db.use_ns("actualised").use_db("core").await?;
        
        Ok(Self {
            db,
            agents: Vec::new(),
            projects: Vec::new(),
        })
    }

    pub async fn add_agent(&mut self, agent: Agent) -> surrealdb::Result<()> {
        let id = agent.id.clone();
        
        // Insert into SurrealDB
        let mut _res = self.db
            .query("CREATE type::thing('agent', $id) CONTENT $agent")
            .bind(("id", &id))
            .bind(("agent", &agent))
            .await?;

        // Map edge if parent_id exists
        if let Some(parent) = &agent.parent_id {
            let sql = format!("RELATE agent:{}->manages->agent:{}", parent, id);
            let mut _res = self.db
                .query(sql)
                .await?;
        }

        self.agents.push(agent);
        Ok(())
    }

    pub async fn add_project(&mut self, project: Project) -> surrealdb::Result<()> {
        let mut _res = self.db
            .query("CREATE type::thing('project', $id) CONTENT $project")
            .bind(("id", &project.id))
            .bind(("project", &project))
            .await?;
            
        self.projects.push(project);
        Ok(())
    }
}
