mod game;
mod rate_limiter;
mod room;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::rate_limiter::{RateLimiter, rate_limit};
use crate::room::Lobby;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{any, get};
use axum::{Json, Router};
use log::{info, warn};
use serde::Deserialize;
use shared::{ClientMessage, RoomInfo, ServerMessage};
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    env_logger::init();

    let lobby = Lobby::default();
    tokio::spawn({
        let lobby = lobby.clone();
        async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                lobby.remove_idle(Duration::from_secs(10 * 60));
            }
        }
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        router(lobby).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn router(lobby: Lobby) -> Router {
    let dist = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/dist");
    let index = ServeFile::new(format!("{dist}/index.html"));

    let frontend =
        ServeDir::new(dist).fallback(SetStatus::new(index.clone(), StatusCode::NOT_FOUND));

    Router::new()
        .route("/rooms", get(list_rooms).post(create_room))
        .route("/rooms/{id}", get(get_room))
        .route_service("/room/{id}", index)
        .route("/ws/{id}", any(ws_handler))
        .fallback_service(frontend)
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn_with_state(
            RateLimiter::default(),
            rate_limit,
        ))
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

async fn get_room(
    State(lobby): State<Lobby>,
    Path(id): Path<Uuid>,
) -> Result<Json<RoomInfo>, StatusCode> {
    lobby
        .get(id)
        .map(|r| Json(r.info()))
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct JoinQuery {
    name: String,
    token: Option<Uuid>,
}

/// Join a room by UUID: ws://host/ws/{room_id}?name={player}&token={seat_token}
async fn ws_handler(
    State(lobby): State<Lobby>,
    Path(id): Path<Uuid>,
    Query(q): Query<JoinQuery>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby, id, q))
}

async fn send(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).unwrap();
    socket.send(Message::Text(text.into())).await
}

/// Pumps messages between one websocket and its room.
async fn handle_socket(mut socket: WebSocket, lobby: Lobby, id: Uuid, q: JoinQuery) {
    let name = q.name;
    let (out_tx, mut out_rx) = mpsc::channel(64);
    let joined = match lobby.get(id) {
        None => Err("room not found".to_string()),
        Some(room) => room
            .join(name.clone(), q.token, out_tx.clone())
            .map(|(info, token)| (room, info, token)),
    };
    let (room, info, token) = match joined {
        Ok(j) => j,
        Err(message) => {
            let _ = send(&mut socket, &ServerMessage::Error { message }).await;
            return;
        }
    };

    let _ = send(&mut socket, &ServerMessage::Joined { room: info, token }).await;
    room.sync(&name);

    let mut tokens: f64 = 10.0;
    let mut last = Instant::now();

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
                    let now = Instant::now();
                    tokens = (tokens + now.duration_since(last).as_secs_f64() * 5.0).min(10.0);
                    last = now;
                    if tokens < 1.0 {
                        warn!("dropping message from {name}: rate limited");
                        continue;
                    }
                    tokens -= 1.0;

                    if text.len() > 1024 {
                        warn!("dropping message from {name}: too long");
                        continue;
                    }

                    let Ok(msg) = serde_json::from_str(&text) else { continue }; // ignore malformed

                    if let ClientMessage::Chat { text: chat } = &msg && chat.len() > 256 {
                        warn!("dropping message from {name}: chat too long");
                        continue;
                    }

                    room.send(name.clone(), msg);
                }
                Some(Ok(_)) => {} // ignore binary/ping/pong
                _ => break,       // closed or errored
            },
        }
    }

    if room.leave(name, &out_tx) {
        lobby.remove(id);
    }
}
