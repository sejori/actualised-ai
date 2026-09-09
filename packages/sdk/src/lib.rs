#![deny(clippy::all)]
use napi_derive::napi;
use actualised_core::state::{Agent, CompanyState};
use actualised_core::inference::Tool;
use actualised_core::orchestrator::Orchestrator;
use actualised_core::memory::MemoryManager;
use std::sync::Arc;
use tokio::sync::Mutex;

#[napi]
pub struct Company {
    state: Arc<Mutex<CompanyState>>,
    state_dir: String,
    rate_limit: Arc<Mutex<actualised_core::queue::RateLimitConfig>>,
}

#[napi]
impl Company {
    #[napi]
    pub async fn init(name: String, mission: String, state_directory: String, db_path: String) -> napi::Result<Self> {
        println!("Initializing Company: {} - Mission: {}", name, mission);
        let state = CompanyState::init(&db_path).await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;

        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            state_dir: state_directory,
            rate_limit: Arc::new(Mutex::new(actualised_core::queue::RateLimitConfig::default())),
        })
    }

    #[napi]
    pub async fn set_pacing(&self, requests_per_minute: f64, working_hours_start: Option<String>, working_hours_end: Option<String>) -> napi::Result<()> {
        let mut rl = self.rate_limit.lock().await;
        rl.requests_per_minute = requests_per_minute;
        if let (Some(s), Some(e)) = (working_hours_start, working_hours_end) {
            rl.working_hours = Some((s, e));
        } else {
            rl.working_hours = None;
        }
        Ok(())
    }

    #[napi]
    pub async fn add_agent(&self, agent_json: String) -> napi::Result<()> {
        let agent: Agent = serde_json::from_str(&agent_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.state.lock().await.add_agent(agent).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn update_agent(&self, id: String, agent_json: String) -> napi::Result<()> {
        let agent: Agent = serde_json::from_str(&agent_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.state.lock().await.update_agent(&id, agent).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn remove_agent(&self, id: String) -> napi::Result<()> {
        self.state.lock().await.remove_agent(&id).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn get_agents(&self) -> napi::Result<String> {
        serde_json::to_string(&self.state.lock().await.agents)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    // Tools
    #[napi]
    pub async fn add_tool(&self, tool_json: String) -> napi::Result<()> {
        let tool: Tool = serde_json::from_str(&tool_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.state.lock().await.add_tool(tool).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn get_tools(&self) -> napi::Result<String> {
        serde_json::to_string(&self.state.lock().await.tools)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn queue_message(&self, agent_id: String, message: String) -> napi::Result<()> {
        let mut state = self.state.lock().await;
        if let Some(agent) = state.agents.iter().find(|a| a.id == agent_id).cloned() {
            let mut updated_agent = agent.clone();
            let mut pending = updated_agent.pending_messages.unwrap_or_default();
            pending.push(message);
            updated_agent.pending_messages = Some(pending);
            state.update_agent(&agent_id, updated_agent).await
                .map_err(|e| napi::Error::from_reason(e))?;
            Ok(())
        } else {
            Err(napi::Error::from_reason("Agent not found".to_string()))
        }
    }

    #[napi]
    pub async fn start(&self) -> napi::Result<()> {
        let mem_mgr = MemoryManager::new(&self.state_dir)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            
        let state_guard = self.state.lock().await;
        for agent in &state_guard.agents {
            mem_mgr.setup_agent_dir(&agent.id)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        }

        let mut cloned_state = CompanyState::init("memory").await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        cloned_state.agents = state_guard.agents.clone();
        cloned_state.projects = state_guard.projects.clone();
        cloned_state.tools = state_guard.tools.clone();
        cloned_state.shared_files = state_guard.shared_files.clone();
        drop(state_guard);

        let mut orch = Orchestrator::new(cloned_state, mem_mgr);
        orch.run().await;
        Ok(())
    }
}
