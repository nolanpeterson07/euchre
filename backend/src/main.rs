mod game;
mod room;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use log::{info, warn};
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
    let frontend = ServeDir::new(dist).fallback(ServeFile::new(format!("{dist}/index.html")));

    Router::new()
        .route("/rooms", get(list_rooms).post(create_room))
        .route("/ws/{id}", any(ws_handler))
        .fallback_service(frontend)
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn_with_state(
            RateLimiter::default(),
            rate_limit,
        ))
        .with_state(lobby)
}

#[derive(Clone, Default)]
struct RateLimiter(Arc<Mutex<HashMap<IpAddr, Bucket>>>);

struct Bucket {
    tokens: f64,
    last: Instant,
}

const RATE: f64 = 10.0;
const BURST: f64 = 30.0;

impl RateLimiter {
    fn allow(&self, ip: IpAddr) -> bool {
        let mut map = self.0.lock().unwrap();

        if map.len() > 10_000 {
            map.retain(|_, b| b.last.elapsed().as_secs_f64() * RATE < BURST);
        }
        let now = Instant::now();
        let b = map.entry(ip).or_insert(Bucket {
            tokens: BURST,
            last: now,
        });

        b.tokens = (b.tokens + now.duration_since(b.last).as_secs_f64() * RATE).min(BURST);
        b.last = now;

        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

async fn rate_limit(
    State(limiter): State<RateLimiter>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    if limiter.allow(addr.ip()) {
        next.run(req).await
    } else {
        warn!("rate limited {}", addr.ip());
        StatusCode::TOO_MANY_REQUESTS.into_response()
    }
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

/// Pumps messages between one websocket and its room.
async fn handle_socket(mut socket: WebSocket, lobby: Lobby, id: Uuid, name: String) {
    let (out_tx, mut out_rx) = mpsc::channel(64);
    let joined = match lobby.get(id) {
        None => Err("room not found".to_string()),
        Some(room) => room.join(name.clone(), out_tx).map(|info| (room, info)),
    };
    let (room, info) = match joined {
        Ok(j) => j,
        Err(message) => {
            let _ = send(&mut socket, &ServerMessage::Error { message }).await;
            return;
        }
    };

    let _ = send(&mut socket, &ServerMessage::Joined { room: info }).await;

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
                    let Ok(msg) = serde_json::from_str(&text) else { continue }; // ignore malformed
                    room.send(name.clone(), msg);
                }
                Some(Ok(_)) => {} // ignore binary/ping/pong
                _ => break,       // closed or errored
            },
        }
    }

    if room.leave(name) {
        lobby.remove(id);
    }
}
