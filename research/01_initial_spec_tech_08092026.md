# actualised.ai: Initial Spec & Technology Research

**Date:** 08-09-2026
**Topic:** Core Architecture, Sandboxing, and Data Modeling

## Executive Summary
actualised.ai aims to allow a single human to run a company of autonomous agents. The team structure operates as a tree where parent nodes (team leads) store team-wide memories. Agents run in isolated browser sandboxes, interacting with a local app functioning as the global admin UI. The core system is proposed to be in Rust, with Python and TypeScript SDKs.

## 1. Core Architecture: Rust with Python & TS SDKs
Building the core orchestration and inference queue in Rust provides the necessary performance, memory safety, and concurrency for running multiple agents locally.
*   **Python SDK (`PyO3`)**: To expose the Rust core to Python (critical for AI/ML ecosystems), **PyO3** and **Maturin** are the industry standards. They allow for seamless, type-safe Python bindings with minimal overhead, supporting high-concurrency (NoGIL) environments.
*   **TypeScript SDK (`napi-rs`)**: For the Node.js/TypeScript frontend and SDK, **napi-rs** is the optimal choice. It automatically generates TypeScript definitions (`.d.ts`) from Rust code and simplifies cross-compilation, enabling high-performance IPC between the frontend and the Rust core.

## 2. Agent Execution: Isolated Browser Sandboxes
Running agents in browser sandboxes is innovative but presents technical challenges, particularly for heavy computations like compiling Rust.
*   **WebContainers (In-Browser Execution)**: StackBlitz's WebContainers allow full Node.js environments and WebAssembly (Wasm) execution directly within a browser tab. This achieves true isolation and zero server compute cost. Experimental projects (like Rubrc) even allow running the Rust compiler (`rustc`) inside the browser via Wasm, validating the feasibility of in-browser compilation.
*   **Browserbase (Web Interaction)**: If agents need to navigate the external web, headless browser infrastructure like Browserbase can be used to handle proxies, CAPTCHAs, and session management, while keeping the execution context sandboxed.
*   *Recommendation*: Use WebAssembly/WebContainers for running agent logic securely in the browser. For tasks that are too heavy for Wasm, the local Rust core can act as a secure backend host, communicating with the browser sandbox via WebSockets.

## 3. Team Structure & Access Control (The Tree)
The hierarchical tree structure dictates that children can read upwards but only edit directly above them.
*   **Memory / State**: A **Vector Database** (e.g., Qdrant, Pinecone, or a local embedded alternative like LanceDB) is essential for agent memories.
*   **Access Control**: This tree-based access can be implemented using robust **metadata filtering** in the vector database. Each document/memory gets tagged with its node ID and hierarchical path. Read queries from a child filter for `path prefix == parent_path`, naturally restricting access.
*   **Topology Management**: A lightweight embedded Graph Database or simply an SQLite database storing adjacency lists/paths can manage the strict "edit directly above" permissions and structural topology.

## 4. Local App UI & Human Admin
The requirement that the first tab is a local app implies a desktop-native web architecture.
*   **Tauri**: Since the core is in Rust, **Tauri** is the perfect framework. It creates a lightweight local app utilizing the OS's native webview. The Tauri app can host the "first tab" UI (React/Svelte/Vue) and serve as the central hub for the human admin to monitor the company, manage agents, and view docs.
