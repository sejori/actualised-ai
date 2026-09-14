use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct VcsError(pub String);

impl From<reqwest::Error> for VcsError {
    fn from(err: reqwest::Error) -> Self {
        VcsError(err.to_string())
    }
}

impl std::fmt::Display for VcsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VCS Error: {}", self.0)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(not(target_arch = "wasm32"))]
pub trait VersionControl: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<String, VcsError>;
    async fn write_file(&self, path: &str, content: &str, message: &str) -> Result<(), VcsError>;
    async fn list_directory(&self, path: &str) -> Result<Vec<String>, VcsError>;
    async fn search_codebase(&self, query: &str) -> Result<Vec<String>, VcsError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(target_arch = "wasm32")]
pub trait VersionControl {
    async fn read_file(&self, path: &str) -> Result<String, VcsError>;
    async fn write_file(&self, path: &str, content: &str, message: &str) -> Result<(), VcsError>;
    async fn list_directory(&self, path: &str) -> Result<Vec<String>, VcsError>;
    async fn search_codebase(&self, query: &str) -> Result<Vec<String>, VcsError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(not(target_arch = "wasm32"))]
pub trait IssueTracker: Send + Sync {
    async fn create_issue(&self, title: &str, body: &str, labels: &[String]) -> Result<String, VcsError>;
    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<(), VcsError>;
    async fn close_issue(&self, issue_id: &str) -> Result<(), VcsError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(target_arch = "wasm32")]
pub trait IssueTracker {
    async fn create_issue(&self, title: &str, body: &str, labels: &[String]) -> Result<String, VcsError>;
    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<(), VcsError>;
    async fn close_issue(&self, issue_id: &str) -> Result<(), VcsError>;
}
