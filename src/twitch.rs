//! Get the current viewer count of a Twitch stream — Helix REST, one call.
//!
//! Cargo.toml:
//! ```toml
//! [dependencies]
//! reqwest = { version = "0.12", features = ["json"] }
//! serde = { version = "1", features = ["derive"] }
//! tokio = { version = "1", features = ["full"] }
//! ```

use std::sync::Arc;

use reqwest::Client;
use rootcause::{Report, option_ext::OptionExt, prelude::ResultExt};
use serde::Deserialize;
use twitch_highway::{AccessToken, ClientId, streams::StreamsAPI, users::UserAPI};

use crate::ParsePlatform;

#[derive(Debug, Deserialize)]
struct AppTokenResponse {
    access_token: String,
}

pub struct Twitch {
    client: Arc<twitch_highway::Client>,
}

impl ParsePlatform for Twitch {
    async fn get_viewer_count(&self, user: &str) -> Result<u64, Report> {
        let res = self.client.get_users().logins(&[user]).send().await?;

        let user_id = res.data.first().ok_or_report()?.id.clone();

        let res = self
            .client
            .get_streams()
            .user_ids(&[user_id])
            .send()
            .await?;
        let views = res
            .data
            .ok_or_report()
            .context(format!("{user}, is not streaming"))?
            .first()
            .ok_or_report()
            .context(format!("{user}, is not streaming"))?
            .viewer_count;

        Ok(views)
    }
}

impl Twitch {
    pub async fn new(
        http: Arc<Client>,
        twitch_client_id: &str,
        twitch_secret: &str,
    ) -> Result<Self, Report> {
        let token = Self::get_app_access_token(&http, twitch_client_id, twitch_secret).await?;

        let client = Arc::new(twitch_highway::Client::new(
            AccessToken::from(token),
            ClientId::from(twitch_client_id),
        ));

        Ok(Self { client })
    }

    async fn get_app_access_token(
        http: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
    ) -> Result<String, reqwest::Error> {
        let resp: AppTokenResponse = http
            .post("https://id.twitch.tv/oauth2/token")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.access_token)
    }
}
