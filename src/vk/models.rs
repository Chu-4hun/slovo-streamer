// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::Data;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: Data = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub data: DataClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataClass {
    pub channel: Channel,
    pub owner: Owner,
    pub streams: Vec<Stream>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub url: String,
    pub cover_url: String,
    pub status: String,
    pub counters: ChannelCounters,
    pub id: i64,
    pub avatar_url: String,
    pub nick: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCounters {
    pub subscribers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub avatar_url: String,
    pub nick: String,
    pub id: i64,
    pub is_verified_streamer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub id: String,
    pub title: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub counters: StreamCounters,
    pub reactions: Vec<Reaction>,
    pub preview_url: String,
    pub video_id: String,
    pub slot: Slot,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCounters {
    pub viewers: i64,
    pub views: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    #[serde(rename = "type")]
    pub reaction_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub id: i64,
    pub url: String,
}
