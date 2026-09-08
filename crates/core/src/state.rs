use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: String,
}

/// Abstract representation of the company state (SurrealDB or in-memory)
pub struct CompanyState {
    pub agents: Vec<Agent>,
    pub projects: Vec<Project>,
}

impl CompanyState {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            projects: Vec::new(),
        }
    }

    pub fn add_agent(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    pub fn add_project(&mut self, project: Project) {
        self.projects.push(project);
    }
}
