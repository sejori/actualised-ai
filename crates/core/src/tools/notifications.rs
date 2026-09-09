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

            self.client.post(&url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            
            println!("Sent Telegram notification: {}", message);
            Ok(())
        } else {
            // Fallback or just ignore if not configured
            println!("Local Push Notification: {}", message);
            Ok(())
        }
    }
}
