#![deny(clippy::all)]
use napi_derive::napi;
use actualised_core::state::{Agent, CompanyState, Project};
use actualised_core::orchestrator::Orchestrator;
use actualised_core::memory::MemoryManager;

#[napi]
pub struct Company {
    state: CompanyState,
    state_dir: String,
}

#[napi]
impl Company {
    #[napi(constructor)]
    pub fn new(name: String, mission: String, state_directory: String) -> Self {
        println!("Initializing Company: {} - Mission: {}", name, mission);
        Self {
            state: CompanyState::new(),
            state_dir: state_directory,
        }
    }

    #[napi]
    pub fn add_agent(&mut self, id: String, name: String, role: String) {
        self.state.add_agent(Agent {
            id,
            name,
            role,
            parent_id: None, // Simplified for stub
        });
    }

    #[napi]
    pub fn add_project(&mut self, id: String, title: String, description: String) {
        self.state.add_project(Project {
            id,
            title,
            description,
        });
    }

    #[napi]
    pub async fn start(&mut self) -> napi::Result<()> {
        let mem_mgr = MemoryManager::new(&self.state_dir)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            
        for agent in &self.state.agents {
            mem_mgr.setup_agent_dir(&agent.id)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        }

        let mut orch = Orchestrator::new(CompanyState {
            agents: self.state.agents.clone(),
            projects: self.state.projects.clone(),
        });
        
        orch.run().await;
        Ok(())
    }
}
