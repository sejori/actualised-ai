use wasm_bindgen::prelude::*;
use actualised_core::state::{CompanyState, Agent, Project};
use actualised_core::default_company::seed_default_company;
use actualised_core::memory::MemoryManager;
use actualised_core::orchestrator::Orchestrator;
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
    pub async fn add_project(&mut self, project_json: JsValue) -> Result<(), JsValue> {
        let project: Project = serde_wasm_bindgen::from_value(project_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid Project JSON: {}", e)))?;
            
        self.inner.state.add_project(project).await
            .map_err(|e| JsValue::from_str(&format!("Failed to add project: {}", e)))?;
            
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_agents(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.state.agents)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_projects(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.state.projects)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn run_orchestrator(&mut self) -> Result<(), JsValue> {
        self.inner.run().await;
        Ok(())
    }
}
