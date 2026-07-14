use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shared::{ClientMessage, Game, MAX_PLAYERS, Phase, RoomInfo, ServerMessage};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct Lobby(Arc<Mutex<HashMap<Uuid, RoomHandle>>>);

impl Lobby {
    pub fn create(&self, name: String) -> RoomInfo {
        let info = RoomInfo {
            id: Uuid::new_v4(),
            name,
            players: vec![],
            in_game: false,
        };
        let room = Room {
            info: info.clone(),
            game: Game::default(),
            tokens: vec![],
            peers: HashMap::new(),
            last_active: Instant::now(),
        };
        self.0
            .lock()
            .unwrap()
            .insert(info.id, RoomHandle(Arc::new(Mutex::new(room))));
        info
    }

    pub fn list(&self) -> Vec<RoomInfo> {
        self.0
            .lock()
            .unwrap()
            .values()
            .map(|h| h.0.lock().unwrap().info.clone())
            .collect()
    }

    pub fn get(&self, id: Uuid) -> Option<RoomHandle> {
        self.0.lock().unwrap().get(&id).cloned()
    }

    pub fn remove(&self, id: Uuid) {
        self.0.lock().unwrap().remove(&id);
    }

    /// Drop rooms with no activity (join, leave, or client message) for longer than `max_idle`
    pub fn remove_idle(&self, max_idle: Duration) {
        self.0.lock().unwrap().retain(|_, h| {
            let mut room = h.0.lock().unwrap();
            let live = room.last_active.elapsed() < max_idle;
            if !live {
                room.peers.clear();
            }
            live
        });
    }
}

/// All the state for one room, behind a single per-room lock.
struct Room {
    info: RoomInfo,
    game: Game,
    tokens: Vec<Uuid>,
    peers: HashMap<String, mpsc::Sender<String>>,
    last_active: Instant,
}

#[derive(Clone)]
pub struct RoomHandle(Arc<Mutex<Room>>);

impl RoomHandle {
    pub fn info(&self) -> RoomInfo {
        self.0.lock().unwrap().info.clone()
    }

    pub fn join(
        &self,
        name: String,
        token: Option<Uuid>,
        out: mpsc::Sender<String>,
    ) -> Result<(RoomInfo, Uuid), String> {
        let mut room = self.0.lock().unwrap();
        
        if let Some(token) = token {
            let seat = room.tokens.iter().position(|t| *t == token);
            if seat.is_some_and(|s| room.info.players[s] == name) {
                room.last_active = Instant::now();
                room.peers.insert(name, out);
                return Ok((room.info.clone(), token));
            }
        }

        if name.is_empty() || name.len() > 32 {
            return Err("name must be 1-32 characters".to_string());
        }
        let rejected = room.game.phase != Phase::Lobby
            || room.peers.contains_key(&name)
            || room.info.players.len() >= MAX_PLAYERS;
        if rejected {
            return Err("room full, in game, or name taken".to_string());
        }
        room.last_active = Instant::now();
        let token = Uuid::new_v4();
        room.tokens.push(token);
        room.peers.insert(name.clone(), out);
        room.info.players.push(name.clone());
        broadcast(&room.peers, &ServerMessage::PlayerJoined { name });
        Ok((room.info.clone(), token))
    }

    /// Returns true if the room is now empty and should be removed from the lobby
    pub fn leave(&self, name: String, out: &mpsc::Sender<String>) -> bool {
        let mut room = self.0.lock().unwrap();
        room.last_active = Instant::now();
        if !room.peers.get(&name).is_some_and(|cur| cur.same_channel(out)) {
            return false;
        }
        room.peers.remove(&name);
        if room.game.phase != Phase::Lobby {
            return false;
        }
        if let Some(seat) = room.info.players.iter().position(|p| p == &name) {
            room.info.players.remove(seat);
            room.tokens.remove(seat);
        }
        broadcast(&room.peers, &ServerMessage::PlayerLeft { name });
        room.info.players.is_empty()
    }

    /// Push the current game state (redacted for `name`) to just that peer
    pub fn sync(&self, name: &str) {
        let room = self.0.lock().unwrap();
        if room.game.phase == Phase::Lobby {
            return;
        }
        let seat = room.info.players.iter().position(|p| p == name);
        send_to(&room.peers, name, &state_for(&room.game, seat));
    }

    pub fn send(&self, name: String, msg: ClientMessage) {
        let mut room = self.0.lock().unwrap();
        room.last_active = Instant::now();
        let Room {
            info, game, peers, ..
        } = &mut *room;
        match crate::game::apply(game, &info.players, &name, &msg) {
            Ok(None) => {
                info.in_game = game.phase != Phase::Lobby;
                for (peer, out) in peers.iter() {
                    let seat = info.players.iter().position(|p| p == peer);
                    let view = state_for(game, seat);
                    let _ = out.try_send(serde_json::to_string(&view).unwrap());
                }
            }
            Ok(Some(reply)) => broadcast(peers, &reply),
            Err(message) => send_to(peers, &name, &ServerMessage::Error { message }),
        }
    }
}

/// The game as `seat` is allowed to see it: their own cards, counts for everyone.
/// `game.hands` itself is `#[serde(skip)]`
fn state_for(game: &Game, seat: Option<usize>) -> ServerMessage {
    ServerMessage::GameState {
        game: game.clone(),
        hand: seat.map(|s| game.hands[s].clone()).unwrap_or_default(),
        hand_counts: game.hands.each_ref().map(|h| h.len() as u8),
    }
}

type Peers = HashMap<String, mpsc::Sender<String>>;

fn broadcast(peers: &Peers, msg: &ServerMessage) {
    let text = serde_json::to_string(msg).unwrap();
    for out in peers.values() {
        let _ = out.try_send(text.clone());
    }
}

fn send_to(peers: &Peers, name: &str, msg: &ServerMessage) {
    if let Some(out) = peers.get(name) {
        let _ = out.try_send(serde_json::to_string(msg).unwrap());
    }
}
