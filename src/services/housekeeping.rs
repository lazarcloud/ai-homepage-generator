use crate::domain::page::{Page, PAGE_TTL, CLEANUP_EVERY};
use crate::services::{generator::generate_page, storage::Storage, rate_limit::RateLimiter};
use crate::clients::groq::GroqClient;
use tokio::time::sleep;
use uuid::Uuid;
use tracing::info;

pub async fn prewarm(storage: Storage, groq: GroqClient, count: usize) {
    let mut tasks = Vec::with_capacity(count);
    for _ in 0..count {
        let storage = storage.clone();
        let groq = groq.clone();
        tasks.push(tokio::spawn(async move {
            generate_page(&groq).await.map(|(html, model)| (storage, html, model))
        }));
    }
    for t in tasks {
        match t.await {
            Ok(Ok((storage, html, model))) => {
                let page = Page { id: Uuid::new_v4(), html, created_at: std::time::Instant::now(), seen_count: 0 };
                storage.push_page(page).await;
                tracing::info!(%model, "prewarmed page");
            }
            _ => tracing::warn!("failed to prewarm one page"),
        }
    }
}

pub async fn start_cleanup(
    storage: Storage,
    limiter: std::sync::Arc<tokio::sync::Mutex<RateLimiter>>,
) {
    tokio::spawn(async move {
        loop {
            sleep(CLEANUP_EVERY).await;
            cleanup_pages(&storage).await;
            let mut guard = limiter.lock().await;
            guard.sweep();
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
