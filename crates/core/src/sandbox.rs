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
    async fn execute_command(&self, _cmd: &str, _args: &[&str]) -> Result<ExecutionOutput, SandboxError> {
        // Implementation stub
        Ok(ExecutionOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        })
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
