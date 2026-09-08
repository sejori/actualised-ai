use actualised_core::state::{CompanyState, Agent, Project};
use actualised_core::orchestrator::Orchestrator;
use actualised_core::memory::MemoryManager;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    println!("Starting Actualised.ai Code-First MVP (Rust Native)...");
    
    let mut state = CompanyState::new();
    let state_dir = "./local_state";

    // 2. Define the Team Graph
    state.add_agent(Agent {
        id: "node_1_engineering".to_string(),
        name: "Engineering Lead".to_string(),
        role: "Manage architecture and break down tasks.".to_string(),
        parent_id: None,
    });
    
    state.add_agent(Agent {
        id: "node_2_backend".to_string(),
        name: "Backend Dev".to_string(),
        role: "Write Rust APIs.".to_string(),
        parent_id: Some("node_1_engineering".to_string()),
    });

    // 3. Define Initial Project
    state.add_project(Project {
        id: "proj_1".to_string(),
        title: "V1 Launch".to_string(),
        description: "Design the database schema and build the initial API.".to_string(),
    });

    println!("Starting engine... (Setting up memory directories and starting DFS loop)");
    
    let mem_mgr = MemoryManager::new(state_dir).unwrap();
    for agent in &state.agents {
        mem_mgr.setup_agent_dir(&agent.id).unwrap();
    }

    let mut orch = Orchestrator::new(state);
    
    // Inject Gemini API Key if present
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        println!("Gemini API key found. Using Gemini Inference Engine.");
        orch.inference = Box::new(actualised_core::inference::GeminiInferenceEngine { api_key });
    } else {
        println!("No Gemini API key found. Using Mock Inference Engine.");
    }

    orch.run().await;

    println!("Done.");
}
