use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub const MAX_PLAYERS: usize = 4;

const BINDINGS_DIR: &str = "../../frontend/src/lib/bindings/";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct RoomInfo {
    pub id: Uuid,
    pub name: String,
    pub players: Vec<String>,
    pub in_game: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct Game {
    pub started: bool,
    /// Index into RoomInfo.players of whose turn it is.
    pub turn: usize,
}

/// Frontend -> server
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Chat { text: String },
    StartGame,
}

/// Server -> frontend
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined { room: RoomInfo },
    PlayerJoined { name: String },
    PlayerLeft { name: String },
    Chat { from: String, text: String },
    GameState { game: Game },
    Error { message: String },
}
