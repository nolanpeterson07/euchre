use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use log::warn;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Default)]
pub struct RateLimiter(Arc<Mutex<HashMap<IpAddr, Bucket>>>);

struct Bucket {
    tokens: f64,
    last: Instant,
}

const RATE: f64 = 10.0;
const BURST: f64 = 30.0;

impl RateLimiter {
    pub fn allow(&self, ip: IpAddr) -> bool {
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

pub async fn rate_limit(
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
