use wasm_bindgen::prelude::*;
use actualised_core::state::{CompanyState, Agent, Project, SharedFile};
use actualised_core::default_company::seed_default_company;
use actualised_core::memory::MemoryManager;
use actualised_core::orchestrator::Orchestrator;
use actualised_core::inference::{InferenceConfig, build_engine, Tool};
use actualised_core::queue::RateLimitConfig;
use wasm_bindgen_futures::future_to_promise;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct OrchestratorWasm {
    inner: Orchestrator,
}

#[wasm_bindgen]
impl OrchestratorWasm {
    #[wasm_bindgen]
    pub async fn init() -> Result<OrchestratorWasm, JsValue> {
        console_error_panic_hook::set_once();
        
        // Initialize state using IndxDb
        let state = CompanyState::init("indxdb://actualised_core").await
            .map_err(|e| JsValue::from_str(&format!("State Init Error: {}", e)))?;
        let mut state = state;
        seed_default_company(&mut state).await
            .map_err(|e| JsValue::from_str(&format!("Default company seed error: {}", e)))?;
            
        let memory = MemoryManager::new("/mock/memory/path")
            .map_err(|e| JsValue::from_str(&format!("Memory Init Error: {}", e)))?;
            
        Ok(OrchestratorWasm {
            inner: Orchestrator::new(state, memory)
        })
    }

    #[wasm_bindgen]
    pub async fn add_agent(&mut self, agent_json: JsValue) -> Result<(), JsValue> {
        let agent: Agent = serde_wasm_bindgen::from_value(agent_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid Agent JSON: {}", e)))?;
        self.inner.state.add_agent(agent).await
            .map_err(|e| JsValue::from_str(&format!("Failed to add agent: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn update_agent(&mut self, id: &str, agent_json: JsValue) -> Result<(), JsValue> {
        let agent: Agent = serde_wasm_bindgen::from_value(agent_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid Agent JSON: {}", e)))?;
        self.inner.state.update_agent(id, agent).await
            .map_err(|e| JsValue::from_str(&format!("Failed to update agent: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn remove_agent(&mut self, id: &str) -> Result<(), JsValue> {
        self.inner.state.remove_agent(id).await
            .map_err(|e| JsValue::from_str(&format!("Failed to remove agent: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_agents(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.state.agents)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // --- Tools ---

    #[wasm_bindgen]
    pub fn get_tools(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.state.tools)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn add_tool(&mut self, tool_json: JsValue) -> Result<(), JsValue> {
        let tool: Tool = serde_wasm_bindgen::from_value(tool_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid Tool JSON: {}", e)))?;
        self.inner.state.add_tool(tool).await
            .map_err(|e| JsValue::from_str(&format!("Failed to add tool: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn update_tool(&mut self, name: &str, tool_json: JsValue) -> Result<(), JsValue> {
        let tool: Tool = serde_wasm_bindgen::from_value(tool_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid Tool JSON: {}", e)))?;
        self.inner.state.update_tool(name, tool).await
            .map_err(|e| JsValue::from_str(&format!("Failed to update tool: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn remove_tool(&mut self, name: &str) -> Result<(), JsValue> {
        self.inner.state.remove_tool(name).await
            .map_err(|e| JsValue::from_str(&format!("Failed to remove tool: {}", e)))?;
        Ok(())
    }

    // --- Shared Files ---

    #[wasm_bindgen]
    pub fn get_shared_files(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.state.shared_files)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn add_shared_file(&mut self, file_json: JsValue) -> Result<(), JsValue> {
        let file: SharedFile = serde_wasm_bindgen::from_value(file_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid SharedFile JSON: {}", e)))?;
        self.inner.state.add_shared_file(file).await
            .map_err(|e| JsValue::from_str(&format!("Failed to add shared file: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn remove_shared_file(&mut self, id: &str) -> Result<(), JsValue> {
        self.inner.state.remove_shared_file(id).await
            .map_err(|e| JsValue::from_str(&format!("Failed to remove shared file: {}", e)))?;
        Ok(())
    }

    // --- Projects ---
    #[wasm_bindgen]
    pub async fn add_project(&mut self, project_json: JsValue) -> Result<(), JsValue> {
        let project: Project = serde_wasm_bindgen::from_value(project_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid Project JSON: {}", e)))?;
        self.inner.state.add_project(project).await
            .map_err(|e| JsValue::from_str(&format!("Failed to add project: {}", e)))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_projects(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.state.projects)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    // --- Orchestration ---
    #[wasm_bindgen]
    pub async fn run_orchestrator(&mut self) -> Result<(), JsValue> {
        self.inner.run().await;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn configure_inference(&mut self, config_json: JsValue) -> Result<(), JsValue> {
        let config: InferenceConfig = serde_wasm_bindgen::from_value(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid inference config: {}", e)))?;

        self.inner.set_inference(build_engine(&config));
        Ok(())
    }

    #[wasm_bindgen]
    pub fn configure_rate_limits(&mut self, config_json: JsValue) -> Result<(), JsValue> {
        let config: RateLimitConfig = serde_wasm_bindgen::from_value(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid rate limit config: {}", e)))?;

        self.inner.set_rate_limit(config);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_agent_memories(&self, agent_id: &str) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.memory.read_memories(agent_id))
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_agent_memory_tree(&self, agent_id: &str) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.get_agent_memory_tree(agent_id))
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_shared_tree(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.get_shared_tree())
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_agent_context(&self, agent_id: &str) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.get_agent_context(agent_id))
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn send_agent_message(&mut self, agent_id: &str, message: String) {
        self.inner.queue_message(agent_id, message);
    }
}
