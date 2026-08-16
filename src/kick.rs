use std::sync::Arc;

use kick_api::{KickApiClient, KickOAuth};
use rootcause::{Report, option_ext::OptionExt, report};

use crate::ParsePlatform;

pub struct Kick {
    client: Arc<KickApiClient>,
}

impl ParsePlatform for Kick {
    async fn get_viewer_count(&self, user: &str) -> Result<u64, Report> {
        let stream = self
            .client
            .channels()
            .get(user)
            .await?
            .stream
            .ok_or_report()?;
        Ok(stream.viewer_count as u64)
    }
}

impl Kick {
    pub async fn new(kick_client_id: String, kick_secret: String) -> Result<Self, Report> {
        let oauth = KickOAuth::new_server(kick_client_id, kick_secret)
            .map_err(|e| report!(e.to_string()))?;
        let token = oauth
            .get_app_access_token()
            .await
            .map_err(|e| report!(e.to_string()))?;

        let kick_client: Arc<KickApiClient> =
            Arc::new(KickApiClient::with_token(token.access_token));

        Ok(Self {
            client: kick_client,
        })
    }
    // pub async fn get_viewer_count(&self, user: &str) -> Result<u64, Report> {
    //     let stream = self
    //         .client
    //         .channels()
    //         .get(user)
    //         .await?
    //         .stream
    //         .ok_or_report()?;
    //     Ok(stream.viewer_count as u64)
    // }
}
