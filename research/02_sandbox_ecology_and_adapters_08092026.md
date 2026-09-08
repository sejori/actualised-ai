# actualised.ai: Sandbox Ecology & Adapter Pattern

**Date:** 08-09-2026
**Topic:** Open-Source Architecture, Sandbox Adapters, and Development Roadmap

## 1. The Core Architecture: The Adapter Pattern
To support both a free "100% in-browser" tier and a self-hosted/enterprise "cloud UNIX" tier, the Rust core will utilize the **Adapter Pattern**. 
The core orchestration engine will define generic traits (interfaces) for environments, ensuring the agent's logic is completely agnostic to where it is running.

Key Rust Traits:
*   `trait SandboxProvider`: Methods to `spawn()`, `execute_command()`, `read_file()`, `write_file()`.
*   `trait BrowserEnvironment`: Methods to `open_tab()`, `navigate()`, `extract_dom()`, `click()`.
*   `trait NetworkTunnel`: Methods to `expose_port()`, `get_public_url()`.

### Concrete Implementations (Adapters)
*   **Browser-Native (Free Tier)**: `WebContainerSandbox`, `TauriWebviewBrowser`, `ServiceWorkerTunnel`.
*   **Self-Hosted/Cloud (Pro Tier)**: `KubernetesSandbox` / `FirecrackerSandbox`, `BrowserbaseEnvironment`, `NgrokTunnel` / `TailscaleTunnel`.

## 2. Phase 1: The Single-Browser MVP
The initial focus is the browser-native environment. 
*   **Mechanism**: The human runs a local app (or a standard web app). Each agent is spawned as a background tab or invisible WebView.
*   **Spawning**: When an agent needs a new environment (e.g., to view Figma), it calls the `BrowserEnvironment::open_tab()` trait. The browser adapter translates this into opening a new local WebView window or an iframe.
*   **Value**: This validates the core agent reasoning, team hierarchy, and memory access without racking up cloud infrastructure costs, making the open-source project highly accessible.

## 3. The Natural Next Steps (Expanding to UNIX & Cloud)
To evolve from the Single-Browser MVP to the full Sandbox Ecology capable of compiling Rust backends, follow these steps:

### Step 1: Network Abstraction (The Bridge)
Before introducing external VMs, you must force the browser-based agents to communicate via standard network protocols (HTTP/WebSockets) instead of direct local IPC. By treating another browser tab as a "remote" server via virtual networking (Service Workers intercepting fetch requests), you guarantee the agent logic won't break when the sandboxes are eventually physically separated.

### Step 2: The Local UNIX Adapter (Docker)
Implement the first non-browser adapter: a `DockerSandboxProvider`. 
When an agent determines it needs to compile a heavy backend language, it requests a UNIX sandbox. The Rust core spins up a local Docker container. The agent uses its standardized toolset to execute `cargo build` inside that container. This proves the agent can manage a hybrid environment (Browser Brain + UNIX Worker) on a local machine.

### Step 3: Local Tunneling Integration
Implement the `NetworkTunnel` adapter. When the agent runs the compiled server in the Docker container, the Rust core establishes a secure tunnel (e.g., using local port forwarding or an ngrok wrapper). The agent can now instruct its browser tab to navigate to the tunneled URL, successfully testing the UNIX backend from the Browser frontend.

### Step 4: Cloud Infrastructure Adapters
Once the hybrid local setup works perfectly, you simply write new adapters for cloud providers (`KubernetesSandbox`, `FlyIoSandbox`). Because of the Adapter pattern, the agent code requires **zero changes**. The transition from local testing to cloud-native autonomous development is completely seamless for the user.
