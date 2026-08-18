use std::{sync::Arc, time::Duration};

use clap::Parser;
use rootcause::prelude::*;
use slovo::{ViewerCounts, fetch_viewer_counts, kick::Kick, twitch::Twitch, vk::VK, youtube::YouTube};
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env)]
    twitch_client_id: Option<String>,

    #[arg(long, env)]
    twitch_secret: Option<String>,

    #[arg(alias = "tuser", long, env)]
    twitch_user: Option<String>,

    #[arg(long, env)]
    kick_client_id: Option<String>,

    #[arg(long, env)]
    kick_secret: Option<String>,

    #[arg(alias = "kuser", long, env)]
    kick_user: Option<String>,

    #[arg(long, env)]
    vk_client_id: Option<String>,

    #[arg(long, env)]
    vk_secret: Option<String>,

    #[arg(alias = "vuser", long, env)]
    vk_user: Option<String>,

    #[arg(long, env)]
    youtube_api_key: Option<String>,

    /// Хэндл канала (`@channel`, с `@` или без)
    #[arg(alias = "yuser", long, env)]
    youtube_user: Option<String>,

    /// Timeout for each fetch
    #[arg(alias = "timeout", long, env, default_value_t = 5)]
    timeout_secs: u64,

    /// Cooldown time between updates in seconds
    #[arg(alias = "cooldown", long, env, default_value_t = 3)]
    cooldown_secs: u64,
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    dotenvy::dotenv()?;

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("не удалось установить rustls CryptoProvider");
    
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .init();
    let http = Arc::new(reqwest::Client::new());
    let args = Args::parse();

    // Создаём Twitch, если заданы все параметры
    let twitch = if let (Some(client_id), Some(secret), Some(_)) = (
        args.twitch_client_id.as_deref(),
        args.twitch_secret.as_deref(),
        args.twitch_user.as_deref(),
    ) {
        Some(Twitch::new(http.clone(), client_id, secret).await?)
    } else {
        warn!("⚠️  Twitch не настроен (пропущен)");
        None
    };

    // Создаём Kick, если заданы все параметры
    let kick = if let (Some(client_id), Some(secret), Some(_)) = (
        args.kick_client_id.as_deref(),
        args.kick_secret.as_deref(),
        args.kick_user.as_deref(),
    ) {
        Some(Kick::new(client_id.to_string(), secret.to_string()).await?)
    } else {
        warn!("⚠️  Kick не настроен (пропущен)");
        None
    };

    // Создаём VK, если заданы все параметры
    let vk = if let (Some(client_id), Some(secret), Some(_)) = (
        args.vk_client_id.as_deref(),
        args.vk_secret.as_deref(),
        args.vk_user.as_deref(),
    ) {
        Some(VK::new(
            http.clone(),
            client_id.to_string(),
            secret.to_string(),
        ))
    } else {
        warn!("⚠️  VK не настроен (пропущен)");
        None
    };

    // Создаём YouTube, если заданы все параметры
    let youtube = if let (Some(api_key), Some(_)) = (
        args.youtube_api_key.as_deref(),
        args.youtube_user.as_deref(),
    ) {
        Some(YouTube::new(api_key.to_string())?)
    } else {
        warn!("⚠️  YouTube не настроен (пропущен)");
        None
    };

    let timeout = Duration::from_secs(args.timeout_secs);

    let mut counts = ViewerCounts::default();

    loop {
        let new_counts = fetch_viewer_counts(
            twitch.as_ref(),
            kick.as_ref(),
            vk.as_ref(),
            youtube.as_ref(),
            args.twitch_user.as_deref(),
            args.kick_user.as_deref(),
            args.vk_user.as_deref(),
            args.youtube_user.as_deref(),
            timeout,
        )
        .await;

        if counts != new_counts {
            println!("{}", serde_json::to_string(&new_counts)?);
            counts = new_counts
        } else {
            info!("stats did not change");
        }

        tokio::time::sleep(Duration::from_secs(args.cooldown_secs)).await;
    }
}
