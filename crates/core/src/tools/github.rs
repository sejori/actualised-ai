use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: Option<String>,
    pub error: Option<String>,
}

pub struct GithubClient {
    client: Client,
    client_id: String,
}

impl GithubClient {
    pub fn new(client_id: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
        }
    }

    /// Step 1 of Device Authentication: Request device and user codes
    pub async fn request_device_code(&self) -> Result<DeviceAuthResponse, String> {
        let res = self.client.post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&[("client_id", &self.client_id)])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let auth_res: DeviceAuthResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(auth_res)
    }

    /// Step 2 of Device Authentication: Poll for the access token
    pub async fn poll_for_token(&self, device_code: &str, interval: u64) -> Result<String, String> {
        loop {
            let res = self.client.post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", &self.client_id),
                    ("device_code", &device_code.to_string()),
                    ("grant_type", &"urn:ietf:params:oauth:grant-type:device_code".to_string()),
                ])
                .send()
                .await
                .map_err(|e| e.to_string())?;

            let token_res: TokenResponse = res.json().await.map_err(|e| e.to_string())?;

            if let Some(token) = token_res.access_token {
                return Ok(token);
            }

            if let Some(err) = token_res.error {
                if err != "authorization_pending" {
                    return Err(format!("Auth error: {}", err));
                }
            }

            sleep(Duration::from_secs(interval)).await;
        }
    }
    pub async fn create_commit(&self, token: &str, repo: &str, message: &str, agent_name: &str) -> Result<(), String> {
        let tagged_message = format!("{}\n\n[Agent: {}]", message, agent_name);
        
        // In a real implementation, this would involve creating a tree, a commit object, and updating the ref.
        // For now, we stub it to demonstrate the tagging.
        println!("Pushing commit to {}: {}", repo, tagged_message);
        
        // Example structure for pushing (pseudo-code):
        // let res = self.client.post(&format!("https://api.github.com/repos/{}/git/commits", repo))
        //    .bearer_auth(token)
        //    .json(&serde_json::json!({
        //        "message": tagged_message,
        //        "tree": "...",
        //        "parents": ["..."]
        //    }))
        //    .send().await...
        
        Ok(())
    }
}
