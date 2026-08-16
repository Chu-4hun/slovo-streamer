//! Get the current viewer count of a Twitch stream — Helix REST, one call.
//!
//! Cargo.toml:
//! ```toml
//! [dependencies]
//! reqwest = { version = "0.12", features = ["json"] }
//! serde = { version = "1", features = ["derive"] }
//! tokio = { version = "1", features = ["full"] }
//! ```

use serde::Deserialize;

const HELIX_BASE: &str = "https://api.twitch.tv/helix";

#[derive(Debug, Deserialize)]
struct AppTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct StreamsResponse {
    data: Vec<StreamEntry>,
}

#[derive(Debug, Deserialize)]
struct StreamEntry {
    viewer_count: u64,
}

/// Gets a free app-only access token (client_credentials grant).
/// Twitch requires this on every Helix call — there is no anonymous tier.
pub async fn get_app_access_token(
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

/// Returns the current viewer count for `login`, or `None` if the channel is offline.
pub async fn get_viewer_count(
    http: &reqwest::Client,
    client_id: &str,
    access_token: &str,
    login: &str,
) -> Result<Option<u64>, reqwest::Error> {
    let resp: StreamsResponse = http
        .get(format!("{HELIX_BASE}/streams"))
        .bearer_auth(access_token)
        .header("Client-Id", client_id)
        .query(&[("user_login", login)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(resp.data.into_iter().next().map(|s| s.viewer_count))
}

// Example usage:
//
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let http = reqwest::Client::new();
//     let token = get_app_access_token(&http, "YOUR_CLIENT_ID", "YOUR_CLIENT_SECRET").await?;
//
//     match get_viewer_count(&http, "YOUR_CLIENT_ID", &token, "some_streamer").await? {
//         Some(count) => println!("{count} viewers watching right now"),
//         None => println!("Channel is offline"),
//     }
//     Ok(())
// }