use reqwest::Client;
use serde_json::json;

pub struct NotificationDispatcher {
    client: Client,
    telegram_token: Option<String>,
    chat_id: Option<String>,
}

impl NotificationDispatcher {
    pub fn new(telegram_token: Option<String>, chat_id: Option<String>) -> Self {
        Self {
            client: Client::new(),
            telegram_token,
            chat_id,
        }
    }

    pub async fn send_notification(&self, message: &str) -> Result<(), String> {
        if let (Some(token), Some(chat_id)) = (&self.telegram_token, &self.chat_id) {
            let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
            let payload = json!({
                "chat_id": chat_id,
                "text": message
            });

            let response = self.client.post(&url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !response.status().is_success() {
                return Err(format!("Telegram sendMessage failed with status {}", response.status()));
            }

            println!("Sent Telegram notification");
            Ok(())
        } else {
            // Fallback or just ignore if not configured
            println!("Local Push Notification: {}", message);
            Ok(())
        }
    }
}
