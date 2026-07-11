use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use shared::{ClientMessage, Game, MAX_PLAYERS, RoomInfo, ServerMessage};
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
            peers: HashMap::new(),
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
}

/// All the state for one room, behind a single per-room lock.
struct Room {
    info: RoomInfo,
    game: Game,
    peers: HashMap<String, mpsc::Sender<String>>,
}

#[derive(Clone)]
pub struct RoomHandle(Arc<Mutex<Room>>);

impl RoomHandle {
    pub fn join(&self, name: String, out: mpsc::Sender<String>) -> Result<RoomInfo, String> {
        let mut room = self.0.lock().unwrap();
        let rejected = name.is_empty()
            || room.peers.contains_key(&name)
            || room.info.players.len() >= MAX_PLAYERS;
        if rejected {
            return Err("room full or name taken".to_string());
        }
        room.peers.insert(name.clone(), out);
        room.info.players.push(name.clone());
        broadcast(&room.peers, &ServerMessage::PlayerJoined { name });
        Ok(room.info.clone())
    }

    /// Returns true if the room is now empty and should be removed from the lobby.
    pub fn leave(&self, name: String) -> bool {
        let mut room = self.0.lock().unwrap();
        room.peers.remove(&name);
        room.info.players.retain(|p| p != &name);
        broadcast(&room.peers, &ServerMessage::PlayerLeft { name });
        room.info.players.is_empty()
    }

    pub fn send(&self, name: String, msg: ClientMessage) {
        let mut room = self.0.lock().unwrap();
        let Room { info, game, peers } = &mut *room;
        match crate::game::apply(game, &info.players, &name, &msg) {
            Ok(reply) => {
                info.in_game = game.started;
                broadcast(peers, &reply);
            }
            Err(message) => send_to(peers, &name, &ServerMessage::Error { message }),
        }
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
