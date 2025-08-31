pub mod clients { pub mod groq; }
pub mod config;
pub mod telemetry;
pub mod state;
pub mod error;
pub mod domain { pub mod page; }
pub mod services { pub mod generator; pub mod housekeeping; pub mod storage; pub mod rate_limit; }
pub mod web { pub mod router; pub mod handlers; pub mod utils; }

use crate::clients::groq::GroqClient;
use crate::services::{storage::Storage, rate_limit::RateLimiter, housekeeping};
use crate::state::AppState;

pub async fn build_app(cfg: crate::config::Config) -> (axum::Router, u16) {
    let groq = GroqClient::new(cfg.groq_api_key).await;
    let storage = Storage::new();
    let limiter = std::sync::Arc::new(tokio::sync::Mutex::new(
        RateLimiter::new(cfg.rate_limit_per_minute),
    ));

    let storage_clone = storage.clone();
    let limiter_for_cleanup = limiter.clone();

    housekeeping::prewarm(storage.clone(), groq.clone(), 5).await;
    housekeeping::start_cleanup(storage_clone, limiter_for_cleanup).await;

    let state = AppState {
        groq: std::sync::Arc::new(groq),
        storage: std::sync::Arc::new(storage),
        limiter,
    };

    (crate::web::router::build_router(state), cfg.port)
}
