# slovo

Агрегатор количества зрителей с Twitch, Kick и VK Видео. Есть два режима
работы:

- **CLI** (`slovo_cli`) — конфиг из CLI-аргументов/`.env`, раз в несколько
  секунд печатает JSON со счётчиками в stdout.
- **WS-сервер** (`slovo_ws`) — конфиг присылается через `POST /config`,
  а счётчики стримятся по WebSocket, отдельно для каждого клиента.

## Где взять токены

### Twitch

https://dev.twitch.tv/console

### Kick

https://dev.kick.com/dashboard

### VK Видео

https://dev.live.vkvideo.ru/apps

## CLI-режим

Настройки передаются флагами или переменными окружения (`.env`).

`.env`:

```env
TWITCH_CLIENT_ID=
TWITCH_SECRET=
TWITCH_USER=

KICK_CLIENT_ID=
KICK_SECRET=
KICK_USER=

VK_CLIENT_ID=
VK_SECRET=
VK_USER=

TIMEOUT_SECS=5
```

Платформу можно не настраивать — она просто пропускается с предупреждением
в логе. Юзернейм платформы можно передать и напрямую флагом (`--tuser`,
`--kuser`, `--vuser`), не трогая `.env`:

```
$ cargo r --bin slovo -- --tuser qadrat --kuser deenthegreat
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.27s
     Running `target\debug\slovo.exe --tuser qadrat --kuser deenthegreat`
2026-08-16T22:57:58.738873Z  WARN slovo: src\main.rs:94: ⚠️  VK не настроен (пропущен)
{"twitch":97,"kick":17232}
```

Новая строка JSON печатается только когда значения действительно
изменились.

## WS-сервер

Запуск:

```
cargo r --bin ws_server
```

По умолчанию слушает `0.0.0.0:8080`, адрес можно поменять флагом `--listen`
или переменной окружения `LISTEN`.

### 1. Отправить конфиг

Ключи платформ передаются не через `.env`, а в теле POST-запроса — под
каждого клиента создаётся свой изолированный "пузырь" с конфигом:

```http
POST http://localhost:8080/config HTTP/1.1
Content-Type: application/json

{
  "twitch_client_id": "dbrd41mjqvqzfoblwhh1bnlfavst1j",
  "twitch_secret": "bf31itz5ata8cz08rj2gvxfmqzet82",
  "twitch_user": "zentreya"
}
```

Необязательные поля запроса: `kick_client_id` / `kick_secret` / `kick_user`,
`vk_client_id` / `vk_secret` / `vk_user`, `timeout_secs` (по умолчанию 5) и
`poll_secs` (по умолчанию 3, как часто опрашивать платформы).

Ответ:

```
HTTP/1.1 200 OK
content-type: application/json; charset=utf-8
connection: close
content-length: 110
date: Sun, 16 Aug 2026 22:54:12 GMT

{
  "ws_url": "ws://localhost:8080/ws/ccc68bef-5d55-4c1b-9c97-22c09b3a665a",
  "twitch": true,
  "kick": false,
  "vk": false
}
```

`ws_url` — готовый адрес, на который нужно подключиться, чтобы получить
именно этот конфиг. `twitch`/`kick`/`vk` показывают, какие платформы
успешно инициализировались.

### 2. Подключиться по WebSocket

```
websocat ws://localhost:8080/ws/ccc68bef-5d55-4c1b-9c97-22c09b3a665a
```

Пока сокет открыт, сервер раз в `poll_secs` опрашивает платформы и шлёт в
сокет JSON с новыми счётчиками — но только если они изменились. Конфиг живёт
в памяти сервера, пока соединение открыто, и удаляется сразу после его
закрытия (штатного или по обрыву связи) — повторно подключиться тем же
`id` уже нельзя, нужно заново постить конфиг.