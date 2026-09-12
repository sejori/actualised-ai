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

#[async_trait]
pub trait VersionControl: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<String, VcsError>;
    async fn write_file(&self, path: &str, content: &str, message: &str) -> Result<(), VcsError>;
    async fn list_directory(&self, path: &str) -> Result<Vec<String>, VcsError>;
    async fn search_codebase(&self, query: &str) -> Result<Vec<String>, VcsError>;
}

#[async_trait]
pub trait IssueTracker: Send + Sync {
    async fn create_issue(&self, title: &str, body: &str, labels: &[String]) -> Result<String, VcsError>;
    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<(), VcsError>;
    async fn close_issue(&self, issue_id: &str) -> Result<(), VcsError>;
}
