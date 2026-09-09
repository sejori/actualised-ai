# Actualised.ai

Build the company that builds the product.

Actualised.ai is a framework designed to let a single human run a company of AI agents. Rather than a single agent operating within a human company, this system orchestrates a hierarchical graph of agents (Team Leads, Developers, Designers, Marketers) that autonomously communicate, delegate, and execute projects.

## Design Philosophy

The core philosophy of Actualised.ai revolves around three pillars:

1. **Human at the Root**: The human user acts as the CEO, setting high-level epics and objectives. The framework handles the rest, allowing AI Team Leads to dynamically create sub-projects and assign them to Individual Contributors (ICs).
2. **Autonomous DAG Execution**: The system is structured as a Directed Acyclic Graph (DAG) stored in a graph database (**SurrealDB**). Agents form edges (`manages`) and nodes, allowing the orchestrator to perform Depth-First Search (DFS) or Breadth-First Search (BFS) traversals to execute workloads hierarchically.
3. **Decoupled Inference Adapter**: The framework logic (database mutations, filesystem access) is strictly decoupled from the LLM. Inference Engines (like the `GeminiInferenceEngine`) act purely as dumb adapters—parsing generic function-calling schemas and returning generic `ToolCall` responses for the framework's `Orchestrator` to execute safely.

## How it Works

The Rust Native Core (`crates/core`) handles everything from memory persistence to API calling. Here is how the system is wired:

### 1. Defining the Graph

Agents are defined with dynamic `system_prompt`s and `tools`. The Human acts as the implicit root, injecting the first level of projects.

```rust
let mut state = CompanyState::init("./local_state/actualised.db").await?;

// The Lead delegates tasks
state.add_agent(Agent {
    id: "node_eng_lead".to_string(),
    name: "Engineering Lead".to_string(),
    role: "Lead Engineer".to_string(),
    parent_id: None,
    system_prompt: "You are the Eng Lead. Break down technical epics and assign them...".to_string(),
    tools: vec!["create_sub_project".to_string(), "assign_task".to_string()],
}).await?;

// The IC produces work
state.add_agent(Agent {
    id: "node_eng_frontend".to_string(),
    name: "Frontend Engineer".to_string(),
    role: "React Developer".to_string(),
    parent_id: Some("node_eng_lead".to_string()),
    system_prompt: "You are the Frontend Engineer. Build UI components...".to_string(),
    tools: vec!["write_memory".to_string()],
}).await?;
```

### 2. The Inference Adapter & Function Calling

The framework uses a unified `InferenceEngine` trait. The `Orchestrator` passes generic JSON-schema tools to the adapter, and the adapter returns generic `ToolCall` objects.

```rust
let response = orch.inference.generate_response(
    &agent.system_prompt,
    "What actions will you take on your assigned tasks?",
    defined_tools // e.g., create_sub_project, write_memory
).await;

match response {
    Ok(InferenceResponse::ToolCalls(calls)) => {
        // Orchestrator safely executes the tool calls!
    }
    _ => {}
}
```

### 3. Execution & Memory

When the LLM decides to use a tool, the orchestrator performs the side effect:
- **`create_sub_project`**: Updates the SurrealKV embedded graph database.
- **`write_memory`**: Persists the agent's output to their local filesystem footprint (`./local_state/[agent_id]/memories`).

### 4. Batched Parallel Inference

To maximize efficiency and minimize bottlenecking, tree traversal is separated from inference execution. The orchestrator stages all possible agent work during its DFS/BFS loop into an `InferenceQueue`. The queue then dispatches these jobs as batched parallel requests, adhering to configured rate limits while tracking crucial stats like tokens per second. This allows the system to fine-tune its inference output to match API provider constraints.

## Quick Start (Rust Native)

1. **Install Prerequisites**: You'll need Node.js (v24+) and Rust (`rustup`).
2. **Environment Variable**: Create a `.env` file in the root or export it in your shell:
   ```bash
   export GEMINI_API_KEY="your_api_key_here"
   export RUSTFLAGS="--cfg surrealdb_unstable" # Required for SurrealKV
   ```
3. **Run the Orchestrator**:
   ```bash
   cd crates/core
   cargo run
   ```

Watch the terminal as the `Engineering Lead`, `Product Lead`, and `Growth Lead` autonomously break down their assignments into epics, and witness the ICs writing code and marketing copy directly to your filesystem!

## Next Steps

- **UI Dashboard**: Expose a frontend dashboard to visualize the agent graph, monitor live tool execution, and allow the Human Root to manually intervene.
- **Execution Sandboxing**: Integrate Docker to let Developer agents safely build and test their own generated code via a new `execute_terminal` tool.
- **Node.js Bindings**: Finalize the `napi-rs` bridge to allow scripting and UI integration from the Node/TypeScript world.
