use std::sync::Arc;

use kick_api::KickApiClient;
use rootcause::{Report, option_ext::OptionExt};

pub async fn get_kick_viewer_count(client: Arc<KickApiClient>, user: &str) -> Result<u32, Report> {
    let stream = client.channels().get(user).await?.stream.ok_or_report()?;
    Ok(stream.viewer_count)
}
