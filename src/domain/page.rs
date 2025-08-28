use std::time::{Duration, Instant};
use uuid::Uuid;

pub const PAGE_TTL: Duration = Duration::from_secs(60);
pub const CLEANUP_EVERY: Duration = Duration::from_secs(5);
pub const MAX_BUCKET_PAGES: usize = 200;

#[derive(Clone)]
pub struct Page {
    pub id: Uuid,
    pub html: String,
    pub created_at: Instant,
    pub seen_count: usize,
}
