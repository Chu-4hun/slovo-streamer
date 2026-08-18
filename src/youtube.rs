//! Get the current concurrent-viewer count of a YouTube live broadcast.
//!
//! Использует `google-youtube3` (сгенерированный клиент YouTube Data API v3).
//! Авторизация — только по API-ключу (`clear_scopes()` + `param("key", ..)`),
//! OAuth не нужен, т.к. достаточно публичных данных.
//!
//! `user` — это YouTube-хэндл канала (например `@somechannel`, с `@` или без).
//! Хэндл резолвится в channel id один раз и кэшируется на всё время жизни
//! структуры, чтобы не тратить лишнюю квоту на `channels.list` при каждом опросе.

use google_youtube3::{YouTube as YouTubeHub, common::auth::NoToken, hyper_rustls, hyper_util};
use rootcause::{Report, option_ext::OptionExt, prelude::ResultExt, report};
use tokio::sync::OnceCell;

use crate::ParsePlatform;

type Connector = hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;
type Hub = YouTubeHub<Connector>;

pub struct YouTube {
    hub: Hub,
    api_key: String,
    /// Кэш `channel id`, зарезолвленного из хэндла при первом запросе.
    channel_id: OnceCell<String>,
}

impl ParsePlatform for YouTube {
    async fn get_viewer_count(&self, user: &str) -> Result<u64, Report> {
        let channel_id = self.resolve_channel_id(user).await?;

        // 1. Ищем текущую live-трансляцию канала.
        let (_, search) = self
            .hub
            .search()
            .list(&vec!["id".to_string()])
            .channel_id(&channel_id)
            .event_type("live")
            .add_type("video")
            .clear_scopes()
            .param("key", &self.api_key)
            .doit()
            .await?;

        let video_id = search
            .items
            .ok_or_report()
            .context(format!("{user}, is not streaming"))?
            .first()
            .ok_or_report()
            .context(format!("{user}, is not streaming"))?
            .id
            .as_ref()
            .ok_or_report()?
            .video_id
            .clone()
            .ok_or_report()?;

        // 2. Достаём concurrentViewers из liveStreamingDetails найденного видео.
        let (_, videos) = self
            .hub
            .videos()
            .list(&vec!["liveStreamingDetails".to_string()])
            .add_id(&video_id)
            .clear_scopes()
            .param("key", &self.api_key)
            .doit()
            .await?;

        let viewers = videos
            .items
            .ok_or_report()
            .context(format!("{user}, is not streaming"))?
            .first()
            .ok_or_report()
            .context(format!("{user}, is not streaming"))?
            .live_streaming_details
            .as_ref()
            .ok_or_report()?
            .concurrent_viewers
            .ok_or_report()
            .context(format!("{user}, viewer count is hidden"))?;

        Ok(viewers)
    }
}

impl YouTube {
    pub fn new(api_key: String) -> Result<Self, Report> {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|err| report!(err.to_string()))?
            .https_or_http()
            .enable_http2()
            .build();

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build(connector);

        // Только API-ключ, без OAuth: NoToken никогда не отдаёт токен, а все
        // вызовы ниже дополнительно снимают скоупы через `clear_scopes()`.
        let hub = YouTubeHub::new(client, NoToken);

        Ok(Self {
            hub,
            api_key,
            channel_id: OnceCell::new(),
        })
    }

    /// Резолвит хэндл канала (`@channel`) в channel id. Результат кэшируется
    /// в `self.channel_id`, так что реальный запрос уйдёт только один раз.
    async fn resolve_channel_id(&self, handle: &str) -> Result<String, Report> {
        let id = self
            .channel_id
            .get_or_try_init(|| async {
                let (_, res) = self
                    .hub
                    .channels()
                    .list(&vec!["id".to_string()])
                    .for_handle(handle)
                    .clear_scopes()
                    .param("key", &self.api_key)
                    .doit()
                    .await?;

                let id = res
                    .items
                    .ok_or_report()
                    .context(format!("{handle}, канал не найден"))?
                    .first()
                    .ok_or_report()
                    .context(format!("{handle}, канал не найден"))?
                    .id
                    .clone()
                    .ok_or_report()?;

                Ok::<String, Report>(id)
            })
            .await?;

        Ok(id.clone())
    }
}
