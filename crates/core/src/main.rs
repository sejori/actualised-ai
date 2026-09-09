use actualised_core::state::{CompanyState, Agent, Project};
use actualised_core::orchestrator::Orchestrator;
use actualised_core::memory::MemoryManager;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    println!("Starting Actualised.ai Code-First MVP (Rust Native)...");
    
    let state_dir = "./local_state";
    let db_path = "./local_state/actualised.db";
    // Clear out old db for clean test
    let _ = std::fs::remove_dir_all(state_dir);
    
    let mut state = CompanyState::init(db_path).await.expect("Failed to init SurrealDB");

    // 1. Engineering Team
    state.add_agent(Agent {
        id: "node_eng_lead".to_string(),
        name: "Engineering Lead".to_string(),
        role: "Lead Engineer".to_string(),
        parent_id: None,
        system_prompt: "You are the Eng Lead. Break down technical epics and assign them to your direct reports: Frontend Engineer and Backend Engineer. Use create_sub_project to outline epics.".to_string(),
        tools: vec!["create_sub_project".to_string(), "assign_task".to_string()],
    }).await.unwrap();
    
    state.add_agent(Agent {
        id: "node_eng_frontend".to_string(),
        name: "Frontend Engineer".to_string(),
        role: "React/UI Developer".to_string(),
        parent_id: Some("node_eng_lead".to_string()),
        system_prompt: "You are the Frontend Engineer. Build UI components based on your assigned tasks. Output your code to 'frontend_component.tsx' using write_memory.".to_string(),
        tools: vec!["write_memory".to_string()],
    }).await.unwrap();

    state.add_agent(Agent {
        id: "node_eng_backend".to_string(),
        name: "Backend Engineer".to_string(),
        role: "Rust API Developer".to_string(),
        parent_id: Some("node_eng_lead".to_string()),
        system_prompt: "You are the Backend Engineer. Build robust Rust APIs. Output your code to 'api.rs' using write_memory.".to_string(),
        tools: vec!["write_memory".to_string()],
    }).await.unwrap();

    // 2. Product Team
    state.add_agent(Agent {
        id: "node_product_lead".to_string(),
        name: "Product Lead".to_string(),
        role: "Product Manager".to_string(),
        parent_id: None,
        system_prompt: "You are the Product Lead. Define product requirements and assign design work to the Designer. Use create_sub_project.".to_string(),
        tools: vec!["create_sub_project".to_string(), "assign_task".to_string()],
    }).await.unwrap();

    state.add_agent(Agent {
        id: "node_product_designer".to_string(),
        name: "Product Designer".to_string(),
        role: "UI/UX Designer".to_string(),
        parent_id: Some("node_product_lead".to_string()),
        system_prompt: "You are the Designer. Create wireframes and CSS templates. Output your design spec to 'wireframe.css' using write_memory.".to_string(),
        tools: vec!["write_memory".to_string()],
    }).await.unwrap();

    // 3. Growth Team
    state.add_agent(Agent {
        id: "node_growth_lead".to_string(),
        name: "Growth Lead".to_string(),
        role: "Marketing Director".to_string(),
        parent_id: None,
        system_prompt: "You are the Growth Lead. Define marketing campaigns. Use create_sub_project to document the campaign.".to_string(),
        tools: vec!["create_sub_project".to_string(), "assign_task".to_string()],
    }).await.unwrap();

    state.add_agent(Agent {
        id: "node_growth_exec".to_string(),
        name: "Marketing Exec".to_string(),
        role: "Marketing Execution".to_string(),
        parent_id: Some("node_growth_lead".to_string()),
        system_prompt: "You are the Marketing Exec. Write marketing copy. Output your copy to 'campaign_email.txt' using write_memory.".to_string(),
        tools: vec!["write_memory".to_string()],
    }).await.unwrap();

    // Inject top-level projects (from implicit Human Root)
    state.add_project(Project {
        id: "proj_root_1".to_string(),
        title: "V1 Launch Architecture".to_string(),
        description: "Assigned to Eng Lead: Build the core database and UI.".to_string(),
    }).await.unwrap();

    state.add_project(Project {
        id: "proj_root_2".to_string(),
        title: "V1 Product Spec".to_string(),
        description: "Assigned to Product Lead: Define the user journey.".to_string(),
    }).await.unwrap();

    println!("Starting engine... (Setting up memory directories and starting DFS loop)");
    
    let mem_mgr = MemoryManager::new(state_dir).unwrap();
    for agent in &state.agents {
        mem_mgr.setup_agent_dir(&agent.id).unwrap();
    }

    let mut orch = Orchestrator::new(state, mem_mgr);
    
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        println!("Gemini API key found. Using Gemini Inference Engine.");
        orch.inference = Box::new(actualised_core::inference::GeminiInferenceEngine { api_key });
    } else {
        println!("No Gemini API key found. Using Mock Inference Engine.");
    }

    orch.run().await;

    println!("Done.");
}
