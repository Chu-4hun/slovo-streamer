mod kick;
pub mod twitch;
pub mod vk;

use std::{sync::Arc, time::Duration};

use clap::Parser;
use kick_api::KickOAuth;
use rootcause::{option_ext::OptionExt, prelude::*};
use twitch_highway::{AccessToken, ClientId, streams::StreamsAPI, users::UserAPI};

use crate::{kick::Kick, twitch::Twitch, vk::VK};

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
    dotenvy::dotenv()?;
    let http = Arc::new(reqwest::Client::new());
    let args = Args::parse();

    let twitch = Twitch::new(http.clone(), &args.twitch_client_id, &args.twitch_secret).await?;
    let kick = Kick::new(args.kick_client_id, args.kick_secret).await?;
    let vk = VK::new(http.clone(), args.vk_client_id, args.vk_secret);

    loop {
        let (twitch, kick, vk, _) = tokio::join!(
            twitch.get_twitch_viewer_count(&args.twitch_user),
            kick.get_viewer_count(&args.kick_user),
            vk.get_vk_viewer_count(&args.vk_user),
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
