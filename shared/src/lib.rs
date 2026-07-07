use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_PLAYERS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: Uuid,
    pub name: String,
    pub players: Vec<String>,
    pub in_game: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Game {
    pub started: bool,
}

/// Frontend -> server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Chat { text: String },
}

/// Server -> frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined { room: RoomInfo },
    PlayerJoined { name: String },
    PlayerLeft { name: String },
    Chat { from: String, text: String },
    Error { message: String },
}
