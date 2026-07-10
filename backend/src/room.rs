use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use shared::{ClientMessage, Game, MAX_PLAYERS, RoomInfo, ServerMessage};
use tokio::sync::{broadcast, mpsc, oneshot};
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
    pub async fn join(
        &self,
        name: String,
    ) -> Result<(broadcast::Receiver<String>, RoomInfo), String> {
        let (reply, rx) = oneshot::channel();
        self.request(Cmd::Join { name, reply }, rx).await?
    }

    pub async fn send(&self, name: String, msg: ClientMessage) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.request(Cmd::Msg { name, msg, reply }, rx).await?
    }

    pub async fn leave(&self, name: String) {
        let _ = self.cmd.send(Cmd::Leave { name }).await;
    }

    async fn request<T>(&self, cmd: Cmd, rx: oneshot::Receiver<T>) -> Result<T, String> {
        let closed = || "room closed".to_string();
        self.cmd.send(cmd).await.map_err(|_| closed())?;
        rx.await.map_err(|_| closed())
    }
}

enum Cmd {
    Join {
        name: String,
        reply: oneshot::Sender<Result<(broadcast::Receiver<String>, RoomInfo), String>>,
    },
    Leave {
        name: String,
    },
    Msg {
        name: String,
        msg: ClientMessage,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

fn send_to_room(tx: &broadcast::Sender<String>, msg: &ServerMessage) {
    let _ = tx.send(serde_json::to_string(msg).unwrap());
}

async fn room_actor(
    lobby: Lobby,
    id: Uuid,
    info: Arc<Mutex<RoomInfo>>,
    mut rx: mpsc::Receiver<Cmd>,
) {
    let (tx, _) = broadcast::channel(64);
    let mut game = Game::default();
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Cmd::Join { name, reply } => {
                let joined = {
                    let mut info = info.lock().unwrap();
                    if info.players.len() < MAX_PLAYERS
                        && !info.players.contains(&name)
                        && !name.is_empty()
                    {
                        info.players.push(name.clone());
                        Ok((tx.subscribe(), info.clone()))
                    } else {
                        Err("room full or name taken".to_string())
                    }
                };
                let ok = joined.is_ok();
                let _ = reply.send(joined);
                if ok {
                    send_to_room(&tx, &ServerMessage::PlayerJoined { name });
                }
            }
            Cmd::Leave { name } => {
                let empty = {
                    let mut info = info.lock().unwrap();
                    info.players.retain(|p| p != &name);
                    info.players.is_empty()
                };
                send_to_room(&tx, &ServerMessage::PlayerLeft { name });
                if empty {
                    lobby.0.lock().unwrap().remove(&id);
                    return;
                }
            }
            Cmd::Msg {
                name,
                msg: ClientMessage::Chat { text },
                reply,
            } => {
                send_to_room(&tx, &ServerMessage::Chat { from: name, text });
                let _ = reply.send(Ok(()));
            }
            Cmd::Msg { name, msg, reply } => {
                let players = info.lock().unwrap().players.clone();
                match crate::game::apply(&mut game, &players, &name, &msg) {
                    Ok(m) => {
                        info.lock().unwrap().in_game = game.started;
                        send_to_room(&tx, &m);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
        }
    }
}
