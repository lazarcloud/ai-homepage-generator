use crate::domain::page::{Page, MAX_BUCKET_PAGES};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct Storage {
    pub pages: Arc<Mutex<VecDeque<Page>>>,
    pub user_seen: Arc<Mutex<HashMap<String, VecDeque<(Uuid, std::time::Instant)>>>>,
}

impl Storage {
    pub fn new() -> Self { Self::default() }

    pub async fn push_page(&self, page: Page) {
        let mut dq = self.pages.lock().await;
        dq.push_back(page);
        trim(&mut dq);
    }

    pub async fn mark_seen(&self, user: &str, id: Uuid) {
        let mut map = self.user_seen.lock().await;
        let dq = map.entry(user.to_string()).or_default();
        dq.push_back((id, std::time::Instant::now()));
    }

    pub async fn update_seen_count(&self, id: Uuid) {
        let mut dq = self.pages.lock().await;
        if let Some(p) = dq.iter_mut().find(|p| p.id == id) {
            p.seen_count += 1;
        }
    }

    pub async fn pages_snapshot(&self) -> Vec<Page> {
        self.pages.lock().await.iter().cloned().collect()
    }
}

pub(crate) fn trim(pages: &mut VecDeque<Page>) {
    while pages.len() > MAX_BUCKET_PAGES {
        if let Some(pos) = pages.iter().position(|p| p.seen_count == 0) {
            pages.remove(pos);
        } else { pages.pop_front(); }
    }
}
