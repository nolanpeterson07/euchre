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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(rename_all = "snake_case")]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(rename_all = "snake_case")]
pub enum Rank {
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

pub type Hand = Vec<Card>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct PlayedCard {
    pub player: usize,
    pub card: Card,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct Bidder {
    pub player: usize,
    pub alone: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct Team {
    pub players: [usize; 2],
    pub score: u8,
    pub tricks_won: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Lobby,
    Bidding1,
    Bidding2,
    AwaitingDiscard,
    Playing,
    HandOver,
    GameOver,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
pub struct Game {
    pub phase: Phase,
    pub turn: usize,
    pub dealer: usize,
    pub teams: [Team; 2],
    pub trump: Option<Suit>,
    pub upcard: Option<Card>,
    pub maker: Option<Bidder>,
    /// Server-only: never serialized, so hidden cards can't leak to clients.
    #[serde(skip)]
    pub hands: [Hand; 4],
    pub trick: Vec<PlayedCard>,
    /// The previous completed trick and who took it, so clients can show it briefly.
    pub last_trick: Vec<PlayedCard>,
    pub trick_winner: Option<usize>,
}

/// Frontend -> server
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Chat { text: String },
    StartGame,
    OrderUp { alone: bool },
    CallTrump { suit: Suit, alone: bool },
    Pass,
    Discard { card: Card },
    PlayCard { card: Card },
    NextHand,
}

/// Server -> frontend
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = BINDINGS_DIR)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined { room: RoomInfo, token: Uuid },
    PlayerJoined { name: String },
    PlayerLeft { name: String },
    Chat { from: String, text: String },
    GameState { game: Game, hand: Hand, hand_counts: [u8; 4] },
    Error { message: String },
}
