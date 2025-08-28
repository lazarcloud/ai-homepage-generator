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
    let limiter = RateLimiter::new(cfg.rate_limit_per_minute);

    let storage_clone = storage.clone();
    let mut limiter_for_sweep = RateLimiter::new(cfg.rate_limit_per_minute);
    let sweep = move || { limiter_for_sweep.sweep(); };

    housekeeping::prewarm(storage.clone(), groq.clone(), 5).await;
    housekeeping::start_cleanup(storage_clone, sweep).await;

    let state = AppState {
        groq: std::sync::Arc::new(groq),
        storage: std::sync::Arc::new(storage),
        limiter: std::sync::Arc::new(tokio::sync::Mutex::new(limiter)),
    };

    (crate::web::router::build_router(state), cfg.port)
}
