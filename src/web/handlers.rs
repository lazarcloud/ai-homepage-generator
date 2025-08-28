use axum::{extract::State, http::StatusCode, response::{Html, IntoResponse}};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::Result;
use crate::state::AppState;
use crate::services::{generator, storage::Storage};
use crate::web::utils::{error_page, rate_limited_page};

pub async fn index(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let (jar, user_id) = get_or_issue_user_id(jar);

    {
        let mut limiter = state.limiter.lock().await;
        if !limiter.hit_ok(&user_id) {
            return (jar, (StatusCode::TOO_MANY_REQUESTS, Html(rate_limited_page(&user_id))));
        }
    }

    match serve_or_generate(&state, &user_id).await {
        Ok(html) => (jar, (StatusCode::OK, Html(html))),
        Err(e) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            let html = error_page(&e.to_string());
            (jar, (status, Html(html)))
        }
    }
}

fn get_or_issue_user_id(jar: CookieJar) -> (CookieJar, String) {
    if let Some(val) = jar.get("user_id").map(|c| c.value().to_string()) { return (jar, val); }
    let user_id = Uuid::new_v4().to_string();
    let cookie = Cookie::build(("user_id", user_id.clone()))
        .path("/").http_only(true).same_site(SameSite::Lax)
        .max_age(time::Duration::days(365)).build();
    (jar.add(cookie), user_id)
}

async fn serve_or_generate(state: &AppState, user_id: &str) -> Result<String> {
    use std::collections::HashSet;
    let seen_ids: HashSet<_> = {
        let mut seen = state.storage.user_seen.lock().await;
        let dq = seen.entry(user_id.to_string()).or_default();
        super::super::services::housekeeping::cleanup_pages(&state.storage).await;
        dq.iter().map(|(id, _)| *id).collect()
    };

    let chosen = {
        let pages = state.storage.pages.lock().await;
        if pages.is_empty() { None } else {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new(); user_id.hash(&mut h);
            let offset = (h.finish() as usize) % pages.len();

            let len = pages.len();
            let mut picked = None;
            for i in 0..len {
                let idx = (offset + i) % len;
                let p = &pages[idx];
                if !seen_ids.contains(&p.id) { picked = Some(p.clone()); break; }
            }
            picked
        }
    };

    if let Some(page) = chosen {
        state.storage.mark_seen(user_id, page.id).await;
        state.storage.update_seen_count(page.id).await;
        tracing::info!(%user_id, page_id=%page.id, time=%OffsetDateTime::now_utc(), "served from bucket");
        return Ok(page.html);
    }

    let (html, model) = generator::generate_page(&state.groq).await?;
    let page = crate::domain::page::Page {
        id: uuid::Uuid::new_v4(), html: html.clone(),
        created_at: std::time::Instant::now(), seen_count: 1,
    };
    state.storage.push_page(page.clone()).await;
    state.storage.mark_seen(user_id, page.id).await;

    tracing::info!(%user_id, page_id=%page.id, %model, "generated new page");
    Ok(html)
}
