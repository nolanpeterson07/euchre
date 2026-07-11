mod game;
mod room;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::routing::{any, get};
use axum::{Json, Router};
use log::info;
use serde::Deserialize;
use shared::{RoomInfo, ServerMessage};
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::room::Lobby;

#[tokio::main]
async fn main() {
    env_logger::init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(Lobby::default()))
        .await
        .unwrap();
}

fn router(lobby: Lobby) -> Router {
    let dist = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/dist");
    let frontend = ServeDir::new(dist).fallback(ServeFile::new(format!("{dist}/index.html")));

    Router::new()
        .route("/rooms", get(list_rooms).post(create_room))
        .route("/ws/{id}", any(ws_handler))
        .fallback_service(frontend)
        .layer(CorsLayer::permissive())
        .with_state(lobby)
}

async fn list_rooms(State(lobby): State<Lobby>) -> Json<Vec<RoomInfo>> {
    Json(lobby.list())
}

#[derive(Deserialize)]
struct CreateRoom {
    name: String,
}

async fn create_room(State(lobby): State<Lobby>, Json(req): Json<CreateRoom>) -> Json<RoomInfo> {
    Json(lobby.create(req.name))
}

#[derive(Deserialize)]
struct JoinQuery {
    name: String,
}

/// Join a room by UUID: ws://host/ws/{room_id}?name={player}
async fn ws_handler(
    State(lobby): State<Lobby>,
    Path(id): Path<Uuid>,
    Query(q): Query<JoinQuery>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby, id, q.name))
}

async fn send(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).unwrap();
    socket.send(Message::Text(text.into())).await
}

/// Pure pipe between one websocket and its room actor.
async fn handle_socket(mut socket: WebSocket, lobby: Lobby, id: Uuid, name: String) {
    let (out_tx, mut out_rx) = mpsc::channel(64);
    let joined = match lobby.get(id) {
        None => Err("room not found".to_string()),
        Some(room) => room.join(name.clone(), out_tx).await.map(|info| (room, info)),
    };
    let (room, info) = match joined {
        Ok(j) => j,
        Err(message) => {
            let _ = send(&mut socket, &ServerMessage::Error { message }).await;
            return;
        }
    };

    let _ = send(&mut socket, &ServerMessage::Joined { room: info }).await;

    loop {
        tokio::select! {
            msg = out_rx.recv() => match msg {
                Some(text) => {
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                None => break, // room gone
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(text))) => {
                    let Ok(msg) = serde_json::from_str(&text) else { continue }; // ignore malformed
                    room.send(name.clone(), msg).await;
                }
                Some(Ok(_)) => {} // ignore binary/ping/pong
                _ => break,       // closed or errored
            },
        }
    }

    room.leave(name).await;
}
