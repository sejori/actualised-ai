use async_trait::async_trait;
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT, ACCEPT}};
use serde::{Deserialize, Serialize};

use crate::vcs::{VersionControl, IssueTracker, VcsError};

pub struct GithubProvider {
    client: Client,
    repo_url: String, // e.g. "sejori/actualised-ai"
}

impl GithubProvider {
    pub fn new(repo_url: String, token: String) -> Result<Self, VcsError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("actualised-ai"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github.v3+json"));
        if !token.is_empty() {
            let auth = HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| VcsError(e.to_string()))?;
            headers.insert(AUTHORIZATION, auth);
        }

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| VcsError(e.to_string()))?;

        Ok(Self { client, repo_url })
    }
}

#[async_trait]
impl VersionControl for GithubProvider {
    async fn read_file(&self, path: &str) -> Result<String, VcsError> {
        let url = format!("https://api.github.com/repos/{}/contents/{}", self.repo_url, path);
        let res = self.client.get(&url).send().await?;
        
        if !res.status().is_success() {
            return Err(VcsError(format!("Failed to read file: {}", res.status())));
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            content: String,
            encoding: String,
        }

        let content_res: ContentResponse = res.json().await?;
        if content_res.encoding == "base64" {
            let decoded = base64::decode(content_res.content.replace("\n", ""))
                .map_err(|e| VcsError(e.to_string()))?;
            String::from_utf8(decoded).map_err(|e| VcsError(e.to_string()))
        } else {
            Ok(content_res.content)
        }
    }

    async fn write_file(&self, path: &str, content: &str, message: &str) -> Result<(), VcsError> {
        Err(VcsError("Not implemented".to_string()))
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<String>, VcsError> {
        Err(VcsError("Not implemented".to_string()))
    }

    async fn search_codebase(&self, query: &str) -> Result<Vec<String>, VcsError> {
        Err(VcsError("Not implemented".to_string()))
    }
}

#[async_trait]
impl IssueTracker for GithubProvider {
    async fn create_issue(&self, title: &str, body: &str, labels: &[String]) -> Result<String, VcsError> {
        Err(VcsError("Not implemented".to_string()))
    }
    
    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<(), VcsError> {
        Err(VcsError("Not implemented".to_string()))
    }
    
    async fn close_issue(&self, issue_id: &str) -> Result<(), VcsError> {
        Err(VcsError("Not implemented".to_string()))
    }
}
