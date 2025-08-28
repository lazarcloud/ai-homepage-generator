use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use dotenvy::dotenv;
use html_escape;
use once_cell::sync::Lazy;
use rand::{rng, seq::SliceRandom};
use regex::Regex;
use std::{
    collections::{HashMap, VecDeque},
    env,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use time::OffsetDateTime;
use uuid::Uuid;

use groq_api_rust::{
    AsyncGroqClient, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionRoles,
};

const PAGE_TTL: Duration = Duration::from_secs(60);
const CLEANUP_EVERY: Duration = Duration::from_secs(5);
const MAX_BUCKET_PAGES: usize = 200;
static THINK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<\s*think\s*>.*?<\s*/\s*think\s*>").expect("valid regex")
});
static FENCE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)```(?:html)?\s*(.*?)\s*```").expect("valid regex")
});

const CANDIDATE_MODELS: &[&str] = &[
    "gemma2-9b-it",
    "qwen/qwen3-32b",
    "deepseek-r1-distill-llama-70b",
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
];

#[derive(Clone)]
struct Page {
    id: Uuid,
    html: String,
    created_at: Instant,
    seen_count: usize,
}

#[derive(Clone)]
struct AppState {
    client: Arc<Mutex<AsyncGroqClient>>,
    pages: Arc<Mutex<VecDeque<Page>>>,
    user_seen: Arc<Mutex<HashMap<String, VecDeque<(Uuid, Instant)>>>>,
    user_hits: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    per_minute_limit: usize,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).compact())
        .init();

    let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set in .env");
    let limit: usize = env::var("RATE_LIMIT_PER_MINUTE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    let client = AsyncGroqClient::new(api_key, None).await;

    let state = AppState {
        client: Arc::new(Mutex::new(client)),
        pages: Arc::new(Mutex::new(VecDeque::new())),
        user_seen: Arc::new(Mutex::new(HashMap::new())),
        user_hits: Arc::new(Mutex::new(HashMap::new())),
        per_minute_limit: limit,
    };

    {
        let st = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(CLEANUP_EVERY).await;
                cleanup(&st).await;
            }
        });
    }

    prewarm_pages(&state, 5).await;

    let app = Router::new().route("/", get(index)).with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!(%addr, "server starting");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

fn maybe_trim_bucket(pages: &mut VecDeque<Page>) {
    while pages.len() > MAX_BUCKET_PAGES {
        if let Some(pos) = pages.iter().position(|p| p.seen_count == 0) {
            pages.remove(pos);
        } else {
            pages.pop_front();
        }
    }
}

async fn prewarm_pages(state: &AppState, count: usize) {
    for _ in 0..count {
        match generate_page(state).await {
            Ok((html, model)) => {
                let page = Page {
                    id: Uuid::new_v4(),
                    html,
                    created_at: Instant::now(),
                    seen_count: 0,
                };
                {
                    let mut pages = state.pages.lock().await;
                    pages.push_back(page);
                }
                info!(%model, "prewarmed page");
            }
            Err(e) => warn!(error=?e, "failed to prewarm one page"),
        }
    }
}

async fn index(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let now = OffsetDateTime::now_utc();

    let (jar, user_id) = get_or_issue_user_id(jar);

    if !hit_ok(&state, &user_id).await {
        warn!(%user_id, limit = state.per_minute_limit, "rate limit exceeded");
        return (
            jar,
            (StatusCode::TOO_MANY_REQUESTS, Html(rate_limited_page(&user_id))),
        );
    }

    match serve_or_generate(&state, &user_id).await {
        Ok(html) => {
            info!(%user_id, time = %now, "served page");
            (jar, (StatusCode::OK, Html(html)))
        }
        Err(e) => {
            error!(%user_id, error=?e, "failed to serve");
            (jar, (StatusCode::INTERNAL_SERVER_ERROR, Html(error_page(&e))))
        }
    }
}

fn get_or_issue_user_id(jar: CookieJar) -> (CookieJar, String) {
    if let Some(val) = jar.get("user_id").map(|c| c.value().to_string()) {
        return (jar, val);
    }

    let user_id = Uuid::new_v4().to_string();
    let cookie = Cookie::build(("user_id", user_id.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(365))
        .build();

    (jar.add(cookie), user_id)
}

async fn serve_or_generate(state: &AppState, user_id: &str) -> Result<String, String> {
    use std::collections::HashSet;
    let seen_ids: HashSet<Uuid> = {
        let mut seen = state.user_seen.lock().await;
        let seen_list = ensure_seen_list(&mut seen, user_id);
        prune_seen(seen_list);
        seen_list.iter().map(|(id, _)| *id).collect()
    };

    let chosen: Option<Page> = {
        let pages = state.pages.lock().await;
        if pages.is_empty() {
            None
        } else {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            user_id.hash(&mut h);
            let offset = (h.finish() as usize) % pages.len();

            let len = pages.len();
            let mut picked = None;
            for i in 0..len {
                let idx = (offset + i) % len;
                let p = &pages[idx];
                if !seen_ids.contains(&p.id) {
                    picked = Some(p.clone());
                    break;
                }
            }
            picked
        }
    };

    if let Some(page) = chosen {
        {
            let mut seen = state.user_seen.lock().await;
            let seen_list = ensure_seen_list(&mut seen, user_id);
            seen_list.push_back((page.id, Instant::now()));
        }
        {
            let mut pages_mut = state.pages.lock().await;
            if let Some(slot) = pages_mut.iter_mut().find(|p| p.id == page.id) {
                slot.seen_count += 1;
            }
        }
        info!(user_id=%user_id, page_id=%page.id, "served from bucket");
        return Ok(page.html);
    }

    let (html, model) = generate_page(state).await.map_err(|e| e.to_string())?;
    let page = Page {
        id: Uuid::new_v4(),
        html: html.clone(),
        created_at: Instant::now(),
        seen_count: 1,
    };

    {
        let mut pages = state.pages.lock().await;
        pages.push_back(page.clone());
        maybe_trim_bucket(&mut pages);
    }
    {
        let mut seen = state.user_seen.lock().await;
        let seen_list = ensure_seen_list(&mut seen, user_id);
        seen_list.push_back((page.id, Instant::now()));
    }

    info!(user_id=%user_id, page_id=%page.id, model=%model, "generated new page");
    Ok(page.html)
}

async fn generate_page(state: &AppState) -> Result<(String, String), String> {
    let mut models = CANDIDATE_MODELS.to_vec();
    models.shuffle(&mut rng());

    let now = OffsetDateTime::now_utc();
    let now_str = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown-time".into());

    for model in models {
        let prompt = format!(
            r#"You are to generate a COMPLETE, self-contained HTML5 landing page.
Return ONLY raw HTML (no Markdown, no code fences), starting with <!doctype html>, and MUST end with </html>.

Hard limits:
- Keep the entire HTML (including inline CSS+JS) under ~12 KB of text.
- No external JS frameworks and no images. You may include ONE Google Fonts <link> if desired.

Content & style:
- <title>: "AI Web Demo".
- You might include current date and time (now: {now_str})
- Include <meta charset="utf-8"> and <meta name="viewport" content="width=device-width, initial-scale=1">.
- Unique aesthetic with inlined CSS only.
- Hero: big headline + short paragraph explaining the page was generated at request time by AI, and it won't be the same twice.
- Some dynamic feature using javascript.
- Footer: include the current year and:
   Generated by {model}
   This project is created by lazar — https://bylazar.com
   Link to this project's GitHub: https://github.com/lazarcloud/ai-based-website
- Make all content visible with proper contrast.

Deliver polished, modern, whimsical vibes. Keep the HTML readable with clear section comments."#
        );

        let messages = vec![ChatCompletionMessage {
            role: ChatCompletionRoles::User,
            content: prompt,
            name: None,
        }];

        let mut req = ChatCompletionRequest::new(model, messages);
        req.max_tokens = Some(4000);
        req.temperature = Some(0.85);

        let client = state.client.lock().await;
        match client.chat_completion(req).await {
            Ok(resp) => {
                let raw_html = resp.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_else(
                    || {
                        format!(
                            "<!doctype html><html><body><h1>No content</h1><p>Model used: {}</p></body></html>",
                            model
                        )
                    },
                );
                let html = strip_think_tags(&raw_html);
                return Ok((html, model.to_string()));
            }
            Err(err) => {
                warn!(model=%model, error=?err, "model failed, trying next");
            }
        }
    }
    Err("all models failed".into())
}

async fn hit_ok(state: &AppState, user_id: &str) -> bool {
    let mut map = state.user_hits.lock().await;
    let hits = map.entry(user_id.to_string()).or_insert_with(VecDeque::new);
    let now = Instant::now();
    while let Some(&t) = hits.front() {
        if now.duration_since(t) > Duration::from_secs(60) {
            hits.pop_front();
        } else {
            break;
        }
    }
    if hits.len() >= state.per_minute_limit {
        return false;
    }
    hits.push_back(now);
    true
}

async fn cleanup(state: &AppState) {
    let now = Instant::now();

    {
        let mut pages = state.pages.lock().await;
        let before = pages.len();

        while let Some(front) = pages.front() {
            let old_enough = now.duration_since(front.created_at) > PAGE_TTL;
            let has_been_seen = front.seen_count > 0;
            if old_enough && has_been_seen {
                pages.pop_front();
            } else if old_enough && !has_been_seen {
                break;
            } else {
                break;
            }
        }

        maybe_trim_bucket(&mut pages);

        let after = pages.len();
        if before != after {
            info!(removed = before as i64 - after as i64, remaining = after, "cleaned pages");
        }
    }

    {
        let mut seen = state.user_seen.lock().await;
        for (_uid, dq) in seen.iter_mut() {
            prune_seen(dq);
        }
    }

    {
        let mut hits = state.user_hits.lock().await;
        for (_uid, dq) in hits.iter_mut() {
            while let Some(&t) = dq.front() {
                if now.duration_since(t) > Duration::from_secs(60) {
                    dq.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}

fn strip_think_tags(s: &str) -> String {
    let no_think = THINK_RE.replace_all(s, "").to_string();
    if let Some(caps) = FENCE_RE.captures(&no_think) {
        caps.get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or(no_think)
    } else {
        no_think
    }
}

fn ensure_seen_list<'a>(
    seen: &'a mut HashMap<String, VecDeque<(Uuid, Instant)>>,
    user_id: &str,
) -> &'a mut VecDeque<(Uuid, Instant)> {
    seen.entry(user_id.to_string()).or_insert_with(VecDeque::new)
}

fn prune_seen(dq: &mut VecDeque<(Uuid, Instant)>) {
    let now = Instant::now();
    while let Some(&(_, t)) = dq.front() {
        if now.duration_since(t) > PAGE_TTL * 2 {
            dq.pop_front();
        } else {
            break;
        }
    }
}

fn rate_limited_page(user_id: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Too many</title></head>
<body><h1>Slow down</h1><p>User {user_id} hit the per-minute limit. Please retry shortly.</p></body></html>"#
    )
}
fn error_page(err: &str) -> String {
    let safe = html_escape::encode_text(err);
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Error</title></head>
<body><h1>Oops!</h1><p>Failed to generate HTML.</p><pre style="white-space:pre-wrap">{safe}</pre></body></html>"#
    )
}
