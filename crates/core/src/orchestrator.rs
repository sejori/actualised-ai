use crate::state::CompanyState;
use crate::inference::{InferenceEngine, MockInferenceEngine};
use petgraph::graph::DiGraph;

pub struct Orchestrator {
    pub state: CompanyState,
    pub task_graph: DiGraph<String, ()>,
    pub inference: Box<dyn InferenceEngine>,
}

impl Orchestrator {
    pub fn new(state: CompanyState) -> Self {
        Self {
            state,
            task_graph: DiGraph::new(),
            inference: Box::new(MockInferenceEngine),
        }
    }

    pub async fn run(&mut self) {
        println!("Orchestrator starting DFS loop...");
        println!("Agents initialized: {}", self.state.agents.len());
        
        for agent in &self.state.agents {
            println!("Waking up agent: {} ({})", agent.name, agent.role);
            let response = self.inference.generate_response(
                &format!("You are {}. Role: {}", agent.name, agent.role),
                "What is your status on your assigned tasks?"
            ).await.unwrap();
            println!("Agent Response: {}", response);
        }
        println!("DFS Loop complete. Engine resting.");
    }
}
