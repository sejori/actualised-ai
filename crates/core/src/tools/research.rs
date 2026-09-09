use reqwest::Client;

pub struct ResearchTool {
    client: Client,
}

impl ResearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Performs a web search and returns markdown results.
    pub async fn web_search(&self, query: &str) -> Result<String, String> {
        // Mock implementation for web search
        println!("Researching UX patterns/products for: {}", query);
        Ok(format!("Results for '{}':\n- Result 1\n- Result 2", query))
    }

    /// Reads a URL and extracts content.
    pub async fn read_browser(&self, url: &str) -> Result<String, String> {
        let res = self.client.get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let text = res.text().await.map_err(|e| e.to_string())?;
        // Mock extraction logic
        Ok(text.chars().take(500).collect::<String>())
    }
}
