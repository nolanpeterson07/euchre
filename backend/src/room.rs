use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use shared::{ClientMessage, Game, MAX_PLAYERS, RoomInfo, ServerMessage};
use tokio::sync::{mpsc, oneshot};
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
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let handle = RoomHandle {
            info: Arc::new(Mutex::new(info.clone())),
            cmd: cmd_tx,
        };
        tokio::spawn(room_actor(
            self.clone(),
            info.id,
            handle.info.clone(),
            cmd_rx,
        ));
        self.0.lock().unwrap().insert(info.id, handle);
        info
    }

    pub fn list(&self) -> Vec<RoomInfo> {
        self.0
            .lock()
            .unwrap()
            .values()
            .map(|h| h.info.lock().unwrap().clone())
            .collect()
    }

    pub fn get(&self, id: Uuid) -> Option<RoomHandle> {
        self.0.lock().unwrap().get(&id).cloned()
    }
}

#[derive(Clone)]
pub struct RoomHandle {
    info: Arc<Mutex<RoomInfo>>,
    cmd: mpsc::Sender<Cmd>,
}

impl RoomHandle {
    pub async fn join(&self, name: String, out: mpsc::Sender<String>) -> Result<RoomInfo, String> {
        let (reply, rx) = oneshot::channel();
        self.cmd
            .send(Cmd::Join { name, out, reply })
            .await
            .map_err(|_| "room closed".to_string())?;
        rx.await.map_err(|_| "room closed".to_string())?
    }

    pub async fn send(&self, name: String, msg: ClientMessage) {
        let _ = self.cmd.send(Cmd::Msg { name, msg }).await;
    }

    pub async fn leave(&self, name: String) {
        let _ = self.cmd.send(Cmd::Leave { name }).await;
    }
}

enum Cmd {
    Join {
        name: String,
        out: mpsc::Sender<String>,
        reply: oneshot::Sender<Result<RoomInfo, String>>,
    },
    Leave {
        name: String,
    },
    Msg {
        name: String,
        msg: ClientMessage,
    },
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

async fn room_actor(
    lobby: Lobby,
    id: Uuid,
    info: Arc<Mutex<RoomInfo>>,
    mut rx: mpsc::Receiver<Cmd>,
) {
    let mut peers = Peers::new();
    let mut game = Game::default();
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Cmd::Join { name, out, reply } => {
                let joined = {
                    let mut info = info.lock().unwrap();
                    if info.players.len() < MAX_PLAYERS
                        && !peers.contains_key(&name)
                        && !name.is_empty()
                    {
                        info.players.push(name.clone());
                        Ok(info.clone())
                    } else {
                        Err("room full or name taken".to_string())
                    }
                };
                if joined.is_ok() {
                    peers.insert(name.clone(), out);
                }
                let ok = joined.is_ok();
                let _ = reply.send(joined);
                if ok {
                    broadcast(&peers, &ServerMessage::PlayerJoined { name });
                }
            }
            Cmd::Leave { name } => {
                peers.remove(&name);
                let empty = {
                    let mut info = info.lock().unwrap();
                    info.players.retain(|p| p != &name);
                    info.players.is_empty()
                };
                broadcast(&peers, &ServerMessage::PlayerLeft { name });
                if empty {
                    lobby.0.lock().unwrap().remove(&id);
                    return;
                }
            }
            Cmd::Msg {
                name,
                msg: ClientMessage::Chat { text },
            } => {
                broadcast(&peers, &ServerMessage::Chat { from: name, text });
            }
            Cmd::Msg { name, msg } => {
                let players = info.lock().unwrap().players.clone();
                match crate::game::apply(&mut game, &players, &name, &msg) {
                    Ok(m) => {
                        info.lock().unwrap().in_game = game.started;
                        broadcast(&peers, &m);
                    }
                    Err(message) => send_to(&peers, &name, &ServerMessage::Error { message }),
                }
            }
        }
    }
}
