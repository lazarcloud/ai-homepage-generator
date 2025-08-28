#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ai_web::telemetry::init();
    let cfg = ai_web::config::Config::load()?;

    let (app, port) = ai_web::build_app(cfg.clone()).await;

    use tracing::info;
    let addr = std::net::SocketAddr::from(([0,0,0,0], port));
    info!(%addr, "server starting");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
