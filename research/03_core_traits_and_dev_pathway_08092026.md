# actualised.ai: Core Traits & Development Pathway

**Date:** 08-09-2026
**Topic:** Rust Trait Definitions and Adapter Development Roadmap

## 1. Core Rust Traits (Technology-Agnostic)

The orchestrator interacts with environments and networking exclusively through generic traits. No specific technologies (like Docker or k8s) are referenced in the core abstractions.

### The Sandbox Trait
This trait defines how an agent interacts with its environment, whether that environment is a local subprocess, a browser tab, or a cloud VM.

```rust
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[async_trait]
pub trait Sandbox: Send + Sync {
    /// Executes a command within the sandbox environment
    async fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<ExecutionOutput, SandboxError>;
    
    /// Reads a file from the sandbox's filesystem
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, SandboxError>;
    
    /// Writes a file to the sandbox's filesystem
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), SandboxError>;
    
    /// Retrieves environment variables for the sandbox
    async fn get_env(&self) -> Result<HashMap<String, String>, SandboxError>;
}
```

### The Network Trait
This trait abstracts how sandboxes communicate with each other and the outside world.

```rust
#[async_trait]
pub trait NetworkBridge: Send + Sync {
    /// Exposes a port from a specific sandbox. 
    /// Returns a URL/URI that other agents or the human can use to access it.
    async fn expose_port(&self, sandbox_id: &str, port: u16) -> Result<String, NetworkError>;
    
    /// Revokes access to a previously exposed port
    async fn close_port(&self, sandbox_id: &str, port: u16) -> Result<(), NetworkError>;
}
```

## 2. The Development Pathway

The adapters will be built in progressive stages of complexity. This allows the core orchestration and agent logic to be tested locally with zero friction before scaling to complex cloud infrastructure.

### Stage 1: Basic UNIX Machine (Local Subprocess)
*   **Sandbox**: `LocalProcessSandbox`. The orchestrator simply runs `std::process::Command`. The agent's "sandbox" is just the human's local filesystem and OS.
*   **Network**: `LocalhostNetworkBridge`. Exposing a port simply returns `http://localhost:{port}`. Networking is free and instantaneous.
*   **Goal**: Prove the core agent loop, memory access, and tool usage work without any infrastructure overhead.

### Stage 2: The Browser
*   **Sandbox**: `WebContainerSandbox` / `WebViewSandbox`. The agent runs inside a browser tab.
*   **Network**: `ServiceWorkerNetworkBridge`. Exposing a port generates a virtual URL handled by the browser's service worker.
*   **Goal**: Prove the platform can run securely in the zero-infrastructure "free tier" intended for web users.

### Stage 3: Local Docker
*   **Sandbox**: `DockerSandbox`. The orchestrator uses the Docker daemon API to spawn containers.
*   **Network**: `DockerNetworkBridge`. Uses Docker's internal named networks to resolve IP addresses between containers.
*   **Goal**: Introduce true isolation and dependency management (e.g., pulling a Rust compiler image) while remaining on the local machine.

### Stage 4: Cloud Sandbox Providers
*   **Sandbox**: `FirecrackerSandbox` / `E2bSandbox`. Spawns remote ephemeral microVMs.
*   **Network**: `TunnelNetworkBridge`. Uses secure tunneling (like Wireguard or Ngrok APIs) to bridge the cloud VM back to the human's local browser or app.
*   **Goal**: Enable heavy-duty remote autonomous development (the "Pro Tier").

### Stage 5: Kubernetes (k8s)
*   **Sandbox**: `KubernetesSandbox`. Spawns pods.
*   **Network**: `K8sServiceBridge`. Uses Kubernetes Services and Ingress for routing.
*   **Goal**: Enterprise-scale self-hosting.
