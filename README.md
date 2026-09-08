# Actualised.ai

Build the company that builds the product.

Actualised.ai is a framework designed to let a single human run a company of AI agents. Rather than a single agent operating within a human company, this system orchestrates a hierarchical graph of agents (Team Leads, Developers, Designers) that autonomously communicate and execute projects.

## Code-First TypeScript SDK MVP

This repository contains the initial Minimum Viable Product (MVP) using a Code-First approach:
- **Rust Core (`crates/core`)**: The technology-agnostic orchestration engine, managing state, filesystem memory, and DFS loops.
- **TypeScript SDK (`packages/sdk`)**: A `napi-rs` bridge exposing the Rust core to Node.js.

### Prerequisites
- Node.js (v24+)
- Rust (via `rustup`)
- MSVC / Visual Studio C++ Build Tools (Windows)

### Quick Start (Rust Native)

1. Create a `.env` file in the root and add your Gemini API key:
   ```env
   GEMINI_API_KEY="your_api_key_here"
   ```
2. Run the core orchestrator using Cargo:
   ```bash
   cd crates/core
   cargo run
   ```

Watch as the agents wake up, generate their memory footprints in `./local_state`, and respond to their roles using the Gemini API!

### Next Steps
- Integrate the `surrealdb` graph database for persistent topological state.
- Expand the `Sandbox` trait to support Docker and Browser-based execution environments.
- Compile and publish the `napi-rs` SDK for full Node.js usage.
