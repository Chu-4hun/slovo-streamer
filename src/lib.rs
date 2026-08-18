pub mod kick;
pub mod twitch;
pub mod vk;
pub mod youtube;

use std::time::Duration;

use rootcause::{Report, prelude::ResultExt};
use serde::Serialize;
use tracing::error;

use crate::{kick::Kick, twitch::Twitch, vk::VK, youtube::YouTube};

pub trait ParsePlatform {
    fn get_viewer_count(
        &self,
        user: &str,
    ) -> impl std::future::Future<Output = Result<u64, Report>> + Send;
}

/// Результаты запросов к четырём платформам
#[derive(Debug, Serialize, PartialEq, Eq, Default)]
pub struct ViewerCounts {
    #[serde(skip_serializing_if = "Option::is_none")]
    twitch: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kick: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vk: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    youtube: Option<u64>,
}

async fn call_with_timeout(
    platform: &Option<&impl ParsePlatform>,
    user: &Option<&str>,
    timeout: &Duration,
) -> Option<u64> {
    match (platform, user.as_ref()) {
        (Some(t), Some(user)) => {
            match tokio::time::timeout(*timeout, t.get_viewer_count(user))
                .await
                .context("Fetch timed out - check your network or restart later")
            {
                Ok(Ok(res)) => Some(res),
                Err(err) => {
                    error!(?err);
                    None
                }
                Ok(Err(err)) => {
                    error!(?err);
                    None
                }
            }
        }
        _ => None,
    }
}

/// Получает количество зрителей для всех четырёх платформ параллельно.
/// Для отсутствующих сервисов возвращается ошибка "not configured".
pub async fn fetch_viewer_counts(
    twitch: Option<&Twitch>,
    kick: Option<&Kick>,
    vk: Option<&VK>,
    youtube: Option<&YouTube>,
    twitch_user: Option<&str>,
    kick_user: Option<&str>,
    vk_user: Option<&str>,
    youtube_user: Option<&str>,
    timeout: Duration,
) -> ViewerCounts {
    let twitch_fut = call_with_timeout(&twitch, &twitch_user, &timeout);
    let kick_fut = call_with_timeout(&kick, &kick_user, &timeout);
    let vk_fut = call_with_timeout(&vk, &vk_user, &timeout);
    let youtube_fut = call_with_timeout(&youtube, &youtube_user, &timeout);

    let (twitch_res, kick_res, vk_res, youtube_res) =
        tokio::join!(twitch_fut, kick_fut, vk_fut, youtube_fut);

    ViewerCounts {
        twitch: twitch_res,
        kick: kick_res,
        vk: vk_res,
        youtube: youtube_res,
    }
}
