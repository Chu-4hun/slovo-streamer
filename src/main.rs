mod kick;
pub mod twitch;
pub mod vk;

use std::{sync::Arc, time::Duration};

use clap::Parser;
use kick_api::KickOAuth;
use rootcause::{option_ext::OptionExt, prelude::*};
use twitch_highway::{AccessToken, ClientId, streams::StreamsAPI, users::UserAPI};

use crate::{kick::get_kick_viewer_count, twitch::get_app_access_token, vk::get_vk_viewer_count};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env)]
    twitch_client_id: String,

    #[arg(long, env)]
    twitch_secret: String,

    #[arg(alias = "tuser", long, env)]
    twitch_user: String,

    #[arg(long, env)]
    kick_client_id: String,

    #[arg(long, env)]
    kick_secret: String,

    #[arg(alias = "kuser", long, env)]
    kick_user: String,

    #[arg(long, env)]
    vk_client_id: String,

    #[arg(long, env)]
    vk_secret: String,

    #[arg(alias = "vuser", long, env)]
    vk_user: String,
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    let http = reqwest::Client::new();
    dotenvy::dotenv()?;
    let args = Args::parse();

    let token = get_app_access_token(&http, &args.twitch_client_id, &args.twitch_secret).await?;

    let client = Arc::new(twitch_highway::Client::new(
        AccessToken::from(token),
        ClientId::from(args.twitch_client_id.clone()),
    ));

    // Initialize Kick OAuth client
    let oauth = KickOAuth::new_server(args.kick_client_id.clone(), args.kick_secret.clone())
        .map_err(|e| report!(e.to_string()))?;
    let oauth = Arc::new(oauth);
    let token = oauth
        .get_app_access_token()
        .await
        .map_err(|e| report!(e.to_string()))?;

    // Initialize Kick API client
    let kick_client = Arc::new(kick_api::KickApiClient::with_token(token.access_token));

    loop {
        let (twitch, kick, vk, _) = tokio::join!(
            get_twitch_viewer_count(client.clone(), &args),
            get_kick_viewer_count(kick_client.clone(), &args.kick_user),
            get_vk_viewer_count(&http, &args.vk_user, &args.vk_client_id, &args.vk_secret),
            tokio::time::sleep(Duration::from_secs(3))
        );
        // Get Twitch viewer count
        let twitch_views = twitch?;
        println!("🟣 twitch views {twitch_views}");

        // Get Kick viewer count
        let kick_views = kick?;
        println!("🟢 kick views {kick_views}");

        let vk = vk?;
        println!("🔵 vk views {vk}");
    }

    Ok(())
}

async fn get_twitch_viewer_count(
    client: Arc<twitch_highway::Client>,
    args: &Args,
) -> Result<u64, Report> {
    let res = client
        .get_users()
        .logins(&[args.twitch_user.as_str()])
        .send()
        .await?;

    let user_id = res.data.first().ok_or_report()?.id.clone();

    let res = client.get_streams().user_ids(&[user_id]).send().await?;
    let views = res
        .data
        .ok_or_report()?
        .first()
        .ok_or_report()?
        .viewer_count;

    Ok(views)
}
