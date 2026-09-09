use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug)]
pub struct SandboxError(pub String);

#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<ExecutionOutput, SandboxError>;
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, SandboxError>;
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), SandboxError>;
    async fn get_env(&self) -> Result<HashMap<String, String>, SandboxError>;
}

/// A simple sandbox that runs commands on the host machine.
pub struct LocalSubprocessSandbox {
    pub working_dir: String,
}

#[async_trait]
impl Sandbox for LocalSubprocessSandbox {
    async fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<ExecutionOutput, SandboxError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::process::Stdio;
            use tokio::process::Command;

            let output = Command::new(cmd)
                .args(args)
                .current_dir(&self.working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| SandboxError(format!("Failed to execute command: {}", e)))?;

            Ok(ExecutionOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Err(SandboxError("Command execution is not supported in WASM".to_string()))
        }
    }

    async fn read_file(&self, path: &str) -> Result<Vec<u8>, SandboxError> {
        std::fs::read(path).map_err(|e| SandboxError(e.to_string()))
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), SandboxError> {
        std::fs::write(path, content).map_err(|e| SandboxError(e.to_string()))
    }

    async fn get_env(&self) -> Result<HashMap<String, String>, SandboxError> {
        Ok(HashMap::new())
    }
}
