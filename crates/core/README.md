# actualised-core

`actualised-core` is the native Rust engine for **Actualised AI** — an autonomous multi-agent orchestration framework designed for AI engineering teams and generative game studios.

## Features

- **Autonomous Agent Hierarchy**: Topological DFS and parent/child delegation across lead agents and specialists.
- **Pluggable Multi-Provider LLM Engine**: Native support for Doubleword AI, Google Gemini, OpenAI, and Anthropic.
- **Per-Agent Routing**: Route individual agents to specific models, providers, and temperatures (e.g. DeepSeek v3.2 / v4.1 on Doubleword for leads, lighter models for leaf nodes).
- **Doubleword Batch Processing**: Automated submission and asynchronous polling of OpenAI-compatible JSONL batch jobs. Completed batches are automatically injected into the requesting agent's next turn.
- **Native Custom Tool Execution**: Implement the `ToolExecutor` trait in pure Rust to intercept and handle custom studio or engine tools (e.g. Godot scene inspections, procedural generators, compiler triggers).
- **Persistent State**: Full integration with SurrealDB (local embedded KV or cloud WebSocket/HTTP) for transactional state, agent memory footprints, and conversation turns.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
actualised-core = "0.1"
```

## Quick Start (Rust Embedding)

```rust
use std::sync::Arc;
use async_trait::async_trait;
use actualised_core::orchestrator::{Orchestrator, ToolExecutor};
use actualised_core::state::{CompanyState, Agent};
use actualised_core::memory::MemoryManager;
use actualised_core::inference::ToolCall;

// 1. Define custom game studio or application tools
struct GameStudioToolExecutor;

#[async_trait]
impl ToolExecutor for GameStudioToolExecutor {
    async fn execute(&self, agent_id: &str, tool_call: &ToolCall) -> Result<String, String> {
        match tool_call.name.as_str() {
            "inspect_scene_tree" => {
                Ok("{\"root\": \"Node3D\", \"children\": [\"Player\", \"WorldEnvironment\"]}".to_string())
            }
            "generate_terrain_tres" => {
                Ok("Generated terrain resource at res://levels/chunk_0_0.tres".to_string())
            }
            _ => Err(format!("Unknown custom tool: {}", tool_call.name)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. Initialize company state and memory manager
    let mut state = CompanyState::init("surrealkv://./state.db", None, None).await?;
    let memory = MemoryManager::new("./agent_memory")?;

    // 3. Set up orchestrator and register the custom tool executor
    let mut orchestrator = Orchestrator::new(state, memory);
    orchestrator.set_tool_executor(Arc::new(GameStudioToolExecutor));

    // 4. Run an orchestration cycle
    orchestrator.run_cycle().await?;

    Ok(())
}
```

## License

MIT
