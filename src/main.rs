use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use dotenvy::dotenv;
use groq_api_rust::{
    AsyncGroqClient, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionRoles,
};
use std::{env, net::SocketAddr, sync::Arc};

#[derive(Clone)]
struct AppState {
    client: Arc<AsyncGroqClient>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let api_key = env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set in .env file");

    let state = AppState {
        client: Arc::new(AsyncGroqClient::new(api_key, None).await),
    };

    let app = Router::new()
        .route("/", get(index))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Server running at http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let messages = vec![ChatCompletionMessage {
        role: ChatCompletionRoles::User,
        content: r#"Generate a short, playful landing page as a COMPLETE HTML5 document.
- Include <html>, <head>, <body>, and a <style> with minimal inline CSS.
- Title: "Groq Web Demo"
- Show a hero headline and a short paragraph explaining this page was generated at request time.
- Add a small footer with the current year.
Return ONLY HTML (no Markdown, no code fences)."#.to_string(),
        name: None,
    }];

    let req = ChatCompletionRequest::new("llama3-70b-8192", messages);

    match state.client.chat_completion(req).await {
        Ok(resp) => {
            let html = resp
                .choices
                .get(0)
                .map(|c| c.message.content.clone())
                .unwrap_or_else(|| "<!doctype html><html><body><h1>No content</h1></body></html>".into());

            Html(html)
        }
        Err(err) => {
            let err_string = err.to_string();
            let safe = html_escape::encode_text(&err_string);

            Html(format!(
                r#"<!doctype html>
<html>
  <head><meta charset="utf-8"><title>Error</title></head>
  <body><h1>Oops!</h1><p>Failed to get AI content: {}</p></body>
</html>"#,
                safe
            ))
        }
    }
}