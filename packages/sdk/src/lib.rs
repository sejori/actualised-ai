#![deny(clippy::all)]
use napi_derive::napi;
use actualised_core::state::{Agent, CompanyState, SharedFile};
use actualised_core::inference::{build_engine, Tool};
use actualised_core::orchestrator::Orchestrator;
use actualised_core::memory::MemoryManager;
use actualised_core::default_company::seed_default_company;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;

#[napi]
pub struct Company {
    orchestrator: Arc<Mutex<Orchestrator>>,
    tool_executor: Option<Arc<JsToolExecutor>>,
}

pub struct JsToolExecutor {
    tsfn: napi::threadsafe_function::ThreadsafeFunction<(String, String, String), napi::threadsafe_function::ErrorStrategy::Fatal>,
}

#[async_trait]
impl actualised_core::orchestrator::ToolExecutor for JsToolExecutor {
    async fn execute(&self, agent_id: &str, tool_call: &actualised_core::inference::ToolCall) -> Result<String, String> {
        let args_json = serde_json::to_string(&tool_call.args).unwrap_or_default();
        
        let res: Result<String, napi::Error> = self.tsfn.call_async((agent_id.to_string(), tool_call.name.clone(), args_json)).await;
        match res {
            Ok(js_str) => Ok(js_str),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[napi]
impl Company {
    #[napi]
    pub async fn init(name: String, mission: String, state_directory: String, db_path: String) -> napi::Result<Self> {
        println!("Initializing Company: {} - Mission: {}", name, mission);
        let mut state = CompanyState::init(&db_path, None).await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        if !state.agents.is_empty() && state.company_name.is_none() {
            state.set_company_name("Pawsome".to_string()).await
                .map_err(napi::Error::from_reason)?;
        }
        let memory = MemoryManager::new(&state_directory)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        for agent in &state.agents {
            memory.setup_agent_dir(&agent.id)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        }

        Ok(Self {
            orchestrator: Arc::new(Mutex::new(Orchestrator::new(state, memory))),
            tool_executor: None,
        })
    }

    #[napi]
    pub async fn init_with_token(name: String, mission: String, state_directory: String, db_path: String, token: String) -> napi::Result<Self> {
        println!("Initializing Authenticated Company: {} - Mission: {}", name, mission);
        let mut state = CompanyState::init(&db_path, Some(token)).await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        if !state.agents.is_empty() && state.company_name.is_none() {
            state.set_company_name("Pawsome".to_string()).await
                .map_err(napi::Error::from_reason)?;
        }
        let memory = MemoryManager::new(&state_directory)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        for agent in &state.agents {
            memory.setup_agent_dir(&agent.id)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        }

        Ok(Self {
            orchestrator: Arc::new(Mutex::new(Orchestrator::new(state, memory))),
            tool_executor: None,
        })
    }

    #[napi]
    pub async fn signup(db_path: String, email: String, pass: String) -> napi::Result<String> {
        actualised_core::auth::signup(&db_path, &email, &pass).await
            .map_err(napi::Error::from_reason)
    }

    #[napi]
    pub async fn signin(db_path: String, email: String, pass: String) -> napi::Result<String> {
        actualised_core::auth::signin(&db_path, &email, &pass).await
            .map_err(napi::Error::from_reason)
    }


    #[napi]
    pub async fn get_company_name(&self) -> Option<String> {
        self.orchestrator.lock().await.state.company_name.clone()
    }

    #[napi]
    pub async fn found_company(&self, name: String) -> napi::Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(napi::Error::from_reason("Company name is required"));
        }
        let mut orchestrator = self.orchestrator.lock().await;
        if orchestrator.state.company_name.is_some() || !orchestrator.state.agents.is_empty() {
            return Err(napi::Error::from_reason("A company already exists"));
        }
        seed_default_company(&mut orchestrator.state).await
            .map_err(napi::Error::from_reason)?;
        orchestrator.state.set_company_name(name.to_string()).await
            .map_err(napi::Error::from_reason)
    }

    #[napi(ts_args_type = "executor: (agentId: string, toolName: string, argsJson: string) => string")]
    pub fn register_tool_executor(&mut self, executor: napi::JsFunction) -> napi::Result<()> {
        use napi::threadsafe_function::{ThreadsafeFunction, ErrorStrategy};
        let tsfn: ThreadsafeFunction<(String, String, String), ErrorStrategy::Fatal> = executor.create_threadsafe_function(0, |ctx| {
            let (agent_id, tool_name, args_json): (String, String, String) = ctx.value;
            let mut vec = Vec::new();
            vec.push(ctx.env.create_string(&agent_id)?);
            vec.push(ctx.env.create_string(&tool_name)?);
            vec.push(ctx.env.create_string(&args_json)?);
            Ok(vec)
        })?;
        
        self.tool_executor = Some(Arc::new(JsToolExecutor { tsfn }));
        Ok(())
    }

    #[napi]
    pub async fn set_pacing(&self, requests_per_minute: f64, working_hours_start: Option<String>, working_hours_end: Option<String>) -> napi::Result<()> {
        let mut orchestrator = self.orchestrator.lock().await;
        let mut rl = orchestrator.rate_limit.clone();
        rl.requests_per_minute = requests_per_minute;
        if let (Some(s), Some(e)) = (working_hours_start, working_hours_end) {
            rl.working_hours = Some((s, e));
        } else {
            rl.working_hours = None;
        }
        orchestrator.set_rate_limit(rl);
        Ok(())
    }

    #[napi]
    pub async fn configure_inference(&self, config_json: String) -> napi::Result<()> {
        let config = serde_json::from_str(&config_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.orchestrator.lock().await.set_inference(build_engine(&config));
        Ok(())
    }

    #[napi]
    pub async fn add_agent(&self, agent_json: String) -> napi::Result<()> {
        let agent: Agent = serde_json::from_str(&agent_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.orchestrator.lock().await.state.add_agent(agent).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn update_agent(&self, id: String, agent_json: String) -> napi::Result<()> {
        let agent: Agent = serde_json::from_str(&agent_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.orchestrator.lock().await.state.update_agent(&id, agent).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn remove_agent(&self, id: String) -> napi::Result<()> {
        self.orchestrator.lock().await.state.remove_agent(&id).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn get_agents(&self) -> napi::Result<String> {
        serde_json::to_string(&self.orchestrator.lock().await.state.agents)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn get_projects(&self) -> napi::Result<String> {
        serde_json::to_string(&self.orchestrator.lock().await.state.projects)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    // Tools
    #[napi]
    pub async fn add_tool(&self, tool_json: String) -> napi::Result<()> {
        let tool: Tool = serde_json::from_str(&tool_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.orchestrator.lock().await.state.add_tool(tool).await
            .map_err(|e| napi::Error::from_reason(e))?;
        Ok(())
    }

    #[napi]
    pub async fn get_tools(&self) -> napi::Result<String> {
        serde_json::to_string(&self.orchestrator.lock().await.state.tools)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn remove_tool(&self, name: String) -> napi::Result<()> {
        self.orchestrator.lock().await.state.remove_tool(&name).await
            .map_err(napi::Error::from_reason)
    }

    #[napi]
    pub async fn add_shared_file(&self, file_json: String) -> napi::Result<()> {
        let file: SharedFile = serde_json::from_str(&file_json)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let mut orchestrator = self.orchestrator.lock().await;
        orchestrator.memory.write_shared(&file.name, &file.content)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        orchestrator.state.add_shared_file(file).await
            .map_err(napi::Error::from_reason)
    }

    #[napi]
    pub async fn get_shared_tree(&self) -> napi::Result<String> {
        serde_json::to_string(&self.orchestrator.lock().await.get_shared_tree())
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn get_agent_memory_tree(&self, agent_id: String) -> napi::Result<String> {
        serde_json::to_string(&self.orchestrator.lock().await.get_agent_memory_tree(&agent_id))
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn get_agent_context(&self, agent_id: String) -> napi::Result<String> {
        serde_json::to_string(&self.orchestrator.lock().await.get_agent_context(&agent_id))
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn queue_message(&self, agent_id: String, message: String) -> napi::Result<()> {
        self.orchestrator.lock().await.queue_message(&agent_id, message).await
            .map_err(napi::Error::from_reason)
    }

    #[napi]
    pub async fn start(&self) -> napi::Result<()> {
        let mut orchestrator = self.orchestrator.lock().await;
        if let Some(ref executor) = self.tool_executor {
            orchestrator.set_tool_executor(executor.clone());
        }
        orchestrator.run().await;
        Ok(())
    }
}
