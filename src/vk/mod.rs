use base64::prelude::*;
use reqwest::Client;
use rootcause::{Report, option_ext::OptionExt, report};
use serde_json::Value;

pub mod models;

pub async fn get_vk_app_access_token(
    http: &Client,
    client_id: &str,
    client_secret: &str,
) -> Result<String, Report> {
    // Формируем Basic-заголовок
    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded = BASE64_STANDARD.encode(credentials);
    let auth_header = format!("Basic {}", encoded);

    let response = http
        .post("https://api.live.vkvideo.ru/oauth/server/token")
        .header("Authorization", auth_header)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials")
        .send()
        .await?;

    let json: Value = response.json().await?;

    // Проверяем на ошибки
    if let Some(error) = json.get("error") {
        return Err(report!("VK auth error: {:?}", error));
    }

    let token = json["access_token"].as_str().ok_or_report()?.to_string();
    Ok(token)
}

// ---- VK Video: получение количества зрителей через метод stream ----

pub async fn get_vk_viewer_count(
    http: &Client,
    user: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<u64, Report> {
    let url = "https://apidev.live.vkvideo.ru/v1/channel";

    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded = BASE64_STANDARD.encode(credentials);

    let response = http
        .get(url)
        .header("Authorization", format!("Basic {}", encoded))
        .query(&[("channel_url", &user)]) // можно заменить на user_id
        .send()
        .await?;

    let json: models::Data = response.json().await?;

    // Извлекаем количество зрителей из ответа
    let viewers = json
        .data
        .streams
        .first()
        .ok_or_report()?
        .counters
        .viewers as u64;

    Ok(viewers)
}
