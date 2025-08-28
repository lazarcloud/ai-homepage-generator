use crate::clients::groq::GroqClient;
use crate::services::storage::Storage;
use crate::services::rate_limit::RateLimiter;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub groq: Arc<GroqClient>,
    pub storage: Arc<Storage>,
    pub limiter: Arc<Mutex<RateLimiter>>,
}
