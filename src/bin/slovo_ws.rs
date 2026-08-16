use std::{collections::HashMap, sync::Arc, time::Duration};

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use poem::{
    EndpointExt, IntoResponse, Request, Route, Server, get, handler,
    http::StatusCode,
    listener::TcpListener,
    post,
    web::{
        Data, Json, Path,
        websocket::{Message, WebSocket},
    },
};
use rootcause::prelude::*;
use serde::{Deserialize, Serialize};
use slovo::{ViewerCounts, fetch_viewer_counts, kick::Kick, twitch::Twitch, vk::VK};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Только адрес, на котором слушает сам сервер — не имеет отношения
/// к конфигурации платформ, та приходит целиком через POST /config.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env, default_value = "0.0.0.0:8080")]
    listen: String,
}

/// Тело запроса `POST /config`.
#[derive(Debug, Deserialize)]
struct ConfigRequest {
    twitch_client_id: Option<String>,
    twitch_secret: Option<String>,
    twitch_user: Option<String>,

    kick_client_id: Option<String>,
    kick_secret: Option<String>,
    kick_user: Option<String>,

    vk_client_id: Option<String>,
    vk_secret: Option<String>,
    vk_user: Option<String>,

    #[serde(default = "default_timeout_secs")]
    timeout_secs: u64,

    /// Как часто опрашивать платформы и слать обновления в сокет.
    #[serde(default = "default_poll_secs")]
    poll_secs: u64,
}

fn default_timeout_secs() -> u64 {
    5
}

fn default_poll_secs() -> u64 {
    6
}

#[derive(Debug, Serialize)]
struct ConfigResponse {
    /// Готовый адрес сокета — "протокольный апгрейд" http->ws / https->wss
    /// уже применён, просто открывай это значение.
    ws_url: String,
    twitch: bool,
    kick: bool,
    vk: bool,
}

/// Живые сервисы платформ + параметры опроса одного клиента.
struct Services {
    twitch: Option<Twitch>,
    kick: Option<Kick>,
    vk: Option<VK>,
    twitch_user: Option<String>,
    kick_user: Option<String>,
    vk_user: Option<String>,
    timeout: Duration,
    poll_interval: Duration,
}

/// id конфига -> сервисы этого клиента. `Arc`, потому что во время жизни
/// сокета данные нужны одновременно и в обработчике WS (чтобы опрашивать
/// платформы), и в мапе (чтобы не потерять их, если соединение оборвётся
/// раньше, чем ожидалось) — удаляются они из мапы только при закрытии
/// соединения, а не в момент подключения.
type Sessions = Arc<RwLock<HashMap<String, Arc<Services>>>>;

/// Заменяет схему запроса на websocket-эквивалент (http -> ws, https -> wss)
/// и собирает адрес сокета для конкретной сессии. Если хост в заголовке
/// `Host` — wildcard-адрес (`0.0.0.0`, `::`, `[::]`), на который сервер
/// слушает, но к которому нельзя *подключиться*, подменяем его на
/// `localhost`, сохраняя порт.
fn build_ws_url(req: &Request, id: &str) -> poem::Result<String> {
    let host = req.header("host").ok_or_else(|| {
        poem::Error::from_string("в запросе нет заголовка Host", StatusCode::BAD_REQUEST)
    })?;

    let host = normalize_host(host);

    let ws_scheme = match req.scheme().as_str() {
        "https" => "wss",
        _ => "ws",
    };

    Ok(format!("{ws_scheme}://{host}/ws/{id}"))
}

/// Меняет wildcard-адрес в `host[:port]` на `localhost[:port]`, чтобы
/// вернуть клиенту урл, к которому реально можно подключиться.
fn normalize_host(host: &str) -> String {
    let (addr, port) = match host.rsplit_once(':') {
        // IPv6 в квадратных скобках, например "[::]:8080" — разделяем
        // только если ':' идёт после закрывающей скобки.
        Some((addr, port)) if addr.ends_with(']') || !addr.contains(':') => (addr, Some(port)),
        _ => (host, None),
    };

    let is_wildcard = matches!(addr, "0.0.0.0" | "::" | "[::]");
    let addr = if is_wildcard { "localhost" } else { addr };

    match port {
        Some(port) => format!("{addr}:{port}"),
        None => addr.to_string(),
    }
}

/// `POST /config` — принимает JSON с настройками платформ, создаёт для
/// клиента отдельный "пузырь" и возвращает адрес WS, на который нужно
/// подключиться, чтобы получить именно этот конфиг.
#[handler]
async fn set_config(
    req: &Request,
    Json(cfg): Json<ConfigRequest>,
    Data(sessions): Data<&Sessions>,
) -> poem::Result<Json<ConfigResponse>> {
    let http = Arc::new(reqwest::Client::new());

    let twitch = if let (Some(client_id), Some(secret), Some(_)) = (
        cfg.twitch_client_id.as_deref(),
        cfg.twitch_secret.as_deref(),
        cfg.twitch_user.as_deref(),
    ) {
        match Twitch::new(http.clone(), client_id, secret).await {
            Ok(t) => Some(t),
            Err(err) => {
                error!("не удалось инициализировать Twitch: {err}");
                None
            }
        }
    } else {
        warn!("⚠️  Twitch не настроен (пропущен)");
        None
    };

    let kick = if let (Some(client_id), Some(secret), Some(_)) = (
        cfg.kick_client_id.as_deref(),
        cfg.kick_secret.as_deref(),
        cfg.kick_user.as_deref(),
    ) {
        match Kick::new(client_id.to_string(), secret.to_string()).await {
            Ok(k) => Some(k),
            Err(err) => {
                error!("не удалось инициализировать Kick: {err}");
                None
            }
        }
    } else {
        warn!("⚠️  Kick не настроен (пропущен)");
        None
    };

    let vk = if let (Some(client_id), Some(secret), Some(_)) = (
        cfg.vk_client_id.as_deref(),
        cfg.vk_secret.as_deref(),
        cfg.vk_user.as_deref(),
    ) {
        Some(VK::new(http.clone(), client_id.to_string(), secret.to_string()))
    } else {
        warn!("⚠️  VK не настроен (пропущен)");
        None
    };

    let id = Uuid::new_v4().to_string();
    let ws_url = build_ws_url(req, &id)?;

    let response = ConfigResponse {
        ws_url,
        twitch: twitch.is_some(),
        kick: kick.is_some(),
        vk: vk.is_some(),
    };

    let services = Arc::new(Services {
        twitch,
        kick,
        vk,
        twitch_user: cfg.twitch_user,
        kick_user: cfg.kick_user,
        vk_user: cfg.vk_user,
        timeout: Duration::from_secs(cfg.timeout_secs),
        poll_interval: Duration::from_secs(cfg.poll_secs),
    });

    sessions.write().await.insert(id.clone(), services);
    info!(
        "создан конфиг {id}: twitch={} kick={} vk={}",
        response.twitch, response.kick, response.vk
    );

    Ok(Json(response))
}

/// `GET /ws/:id` — апгрейд до WebSocket для конкретного `id`, выданного
/// `POST /config`. Конфиг остаётся в общей мапе, пока сокет открыт, и
/// удаляется оттуда только когда соединение закрывается — штатно (клиент
/// прислал `Close`) или из-за ошибки/обрыва сети.
#[handler]
async fn ws(
    Path(id): Path<String>,
    ws: WebSocket,
    Data(sessions): Data<&Sessions>,
) -> poem::Result<impl IntoResponse> {
    let services = sessions.read().await.get(&id).cloned().ok_or_else(|| {
        poem::Error::from_string(
            "неизвестный или уже использованный config id",
            StatusCode::NOT_FOUND,
        )
    })?;

    let sessions = sessions.clone();

    Ok(ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        let mut counts = ViewerCounts::default();
        let timeout = services.timeout;
        let poll_interval = services.poll_interval;

        info!("сокет {id} открыт");

        loop {
            tokio::select! {
                // Клиент закрыл соединение или прислал Close — выходим.
                incoming = stream.next() => {
                    match incoming {
                        Some(Ok(Message::Close(_))) | None => {
                            info!("клиент {id} закрыл ws-соединение");
                            break;
                        }
                        Some(Err(err)) => {
                            warn!("ошибка ws у {id}: {err}");
                            break;
                        }
                        // Ping/Pong/Text/Binary от клиента игнорируем.
                        Some(Ok(_)) => {}
                    }
                }

                _ = tokio::time::sleep(poll_interval) => {
                    let new_counts = fetch_viewer_counts(
                        services.twitch.as_ref(),
                        services.kick.as_ref(),
                        services.vk.as_ref(),
                        services.twitch_user.as_deref(),
                        services.kick_user.as_deref(),
                        services.vk_user.as_deref(),
                        timeout,
                    )
                    .await;

                    if counts != new_counts {
                        let payload = match serde_json::to_string(&new_counts) {
                            Ok(p) => p,
                            Err(err) => {
                                error!("не удалось сериализовать ViewerCounts: {err}");
                                continue;
                            }
                        };

                        if sink.send(Message::text(payload)).await.is_err() {
                            info!("не удалось отправить сообщение, клиент {id} отключился");
                            break;
                        }

                        counts = new_counts;
                    } else {
                        info!("stats did not change ({id})");
                    }
                }
            }
        }

        // Соединение закрылось (любым путём) — конфиг больше никому не
        // нужен, убираем его из общей мапы.
        sessions.write().await.remove(&id);
        info!("конфиг {id} удалён");
    }))
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));

    let app = Route::new()
        .at("/config", post(set_config))
        .at("/ws/:id", get(ws))
        .data(sessions);

    info!("слушаю на {}", args.listen);
    Server::new(TcpListener::bind(&args.listen))
        .run(app)
        .await
        .map_err(Report::new)?;

    Ok(())
}