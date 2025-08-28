use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub groq_api_key: String,
    #[serde(default = "default_limit")]
    pub rate_limit_per_minute: usize,
    #[serde(default = "default_port")]
    pub port: u16,
}
fn default_limit() -> usize { 10 }
fn default_port() -> u16 { 8080 }

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let groq_api_key = std::env::var("GROQ_API_KEY")?;
        let rate_limit_per_minute = std::env::var("RATE_LIMIT_PER_MINUTE")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(default_limit());
        let port = std::env::var("PORT")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(default_port());
        Ok(Self { groq_api_key, rate_limit_per_minute, port })
    }
}
