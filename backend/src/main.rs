use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::routing::{any, get};
use axum::{Json, Router};
use serde::Deserialize;
use shared::{ClientMessage, MAX_PLAYERS, RoomInfo, ServerMessage};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

struct Room {
    info: RoomInfo,
    tx: broadcast::Sender<String>,
}

type Rooms = Arc<Mutex<HashMap<Uuid, Room>>>;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(Rooms::default())).await.unwrap();
}

fn router(rooms: Rooms) -> Router {
    let dist = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/dist");
    let frontend = ServeDir::new(dist).fallback(ServeFile::new(format!("{dist}/index.html")));

    Router::new()
        .route("/rooms", get(list_rooms).post(create_room))
        .route("/ws/{id}", any(ws_handler))
        .fallback_service(frontend)
        .layer(CorsLayer::permissive())
        .with_state(rooms)
}

async fn list_rooms(State(rooms): State<Rooms>) -> Json<Vec<RoomInfo>> {
    Json(rooms.lock().unwrap().values().map(|r| r.info.clone()).collect())
}

#[derive(Deserialize)]
struct CreateRoom {
    name: String,
}

async fn create_room(State(rooms): State<Rooms>, Json(req): Json<CreateRoom>) -> Json<RoomInfo> {
    let info = RoomInfo {
        id: Uuid::new_v4(),
        name: req.name,
        players: vec![],
        in_game: false,
    };
    let (tx, _) = broadcast::channel(64);
    rooms.lock().unwrap().insert(info.id, Room { info: info.clone(), tx });
    Json(info)
}

#[derive(Deserialize)]
struct JoinQuery {
    name: String,
}

/// Join a room by UUID: ws://host/ws/{room_id}?name={player}
async fn ws_handler(
    State(rooms): State<Rooms>,
    Path(id): Path<Uuid>,
    Query(q): Query<JoinQuery>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(socket, rooms, id, q.name))
}

fn send_to_room(tx: &broadcast::Sender<String>, msg: &ServerMessage) {
    let _ = tx.send(serde_json::to_string(msg).unwrap());
}

async fn handle_socket(mut socket: WebSocket, rooms: Rooms, id: Uuid, name: String) {
    let joined = {
        let mut rooms = rooms.lock().unwrap();
        match rooms.get_mut(&id) {
            Some(room)
                if room.info.players.len() < MAX_PLAYERS
                    && !room.info.players.contains(&name) && name.len() > 0 =>
            {
                room.info.players.push(name.clone());
                Some((room.tx.clone(), room.tx.subscribe(), room.info.clone()))
            }
            _ => None,
        }
    };
    let Some((tx, mut rx, info)) = joined else {
        let err = ServerMessage::Error {
            message: "room not found, full, or name taken".into(),
        };
        let _ = socket
            .send(Message::Text(serde_json::to_string(&err).unwrap().into()))
            .await;
        return;
    };

    let joined_msg = serde_json::to_string(&ServerMessage::Joined { room: info }).unwrap();
    let _ = socket.send(Message::Text(joined_msg.into())).await;
    send_to_room(&tx, &ServerMessage::PlayerJoined { name: name.clone() });

    loop {
        tokio::select! {
            msg = rx.recv() => match msg {
                Ok(text) => {
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(ClientMessage::Chat { text }) = serde_json::from_str(&text) {
                        send_to_room(&tx, &ServerMessage::Chat { from: name.clone(), text });
                    }
                }
                Some(Ok(_)) => {} // ignore binary/ping/pong
                _ => break,       // closed or errored
            },
        }
    }

    {
        let mut rooms = rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&id) {
            room.info.players.retain(|p| p != &name);
            if room.info.players.is_empty() {
                rooms.remove(&id);
            }
        }
    }
    send_to_room(&tx, &ServerMessage::PlayerLeft { name });
}
