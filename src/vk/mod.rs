use std::sync::Arc;

use base64::prelude::*;
use reqwest::Client;
use rootcause::{Report, option_ext::OptionExt};

pub mod models;

pub struct VK {
    http: Arc<Client>,
    client_id: String,
    client_secret: String,
}

impl VK {
    pub fn new(http: Arc<Client>, client_id: String, client_secret: String) -> Self {
        Self {
            http,
            client_id,
            client_secret,
        }
    }

    pub async fn get_vk_viewer_count(&self, user: &str) -> Result<u64, Report> {
        let url = "https://apidev.live.vkvideo.ru/v1/channel";

        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        let encoded = BASE64_STANDARD.encode(credentials);

        let response = self
            .http
            .get(url)
            .header("Authorization", format!("Basic {}", encoded))
            .query(&[("channel_url", &user)])
            .send()
            .await?;

        let json: models::Data = response.json().await?;

        let viewers = json.data.streams.first().ok_or_report()?.counters.viewers as u64;

        Ok(viewers)
    }
}
