use crate::domain::page::{Page, PAGE_TTL, CLEANUP_EVERY};
use crate::services::{generator::generate_page, storage::Storage};
use crate::clients::groq::GroqClient;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use tracing::info;

pub async fn prewarm(storage: Storage, groq: GroqClient, count: usize) {
    for _ in 0..count {
        match generate_page(&groq).await {
            Ok((html, model)) => {
                let page = Page { id: Uuid::new_v4(), html, created_at: std::time::Instant::now(), seen_count: 0 };
                storage.push_page(page).await;
                info!(%model, "prewarmed page");
            }
            Err(e) => tracing::warn!(error=?e, "failed to prewarm one page"),
        }
    }
}

pub async fn start_cleanup(storage: Storage, mut sweep_hits: impl FnMut() + Send + 'static) {
    tokio::spawn(async move {
        loop {
            sleep(CLEANUP_EVERY).await;
            cleanup_pages(&storage).await;
            sweep_hits();
        }
    });
}

pub(crate) async fn cleanup_pages(storage: &Storage) {
    use std::time::Instant;
    let now = Instant::now();
    let mut pages = storage.pages.lock().await;
    let before = pages.len();

    while let Some(front) = pages.front() {
        let old = now.duration_since(front.created_at) > PAGE_TTL;
        let seen = front.seen_count > 0;
        if old && seen { pages.pop_front(); } else { break; }
    }
    super::storage::trim(&mut pages);
    let after = pages.len();
    if before != after {
        info!(removed = before as i64 - after as i64, remaining = after, "cleaned pages");
    }

    let mut seen_map = storage.user_seen.lock().await;
    for dq in seen_map.values_mut() {
        let now = Instant::now();
        while let Some(&(_, t)) = dq.front() {
            if now.duration_since(t) > PAGE_TTL * 2 { dq.pop_front(); } else { break; }
        }
    }
}
