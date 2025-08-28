use std::sync::Arc;
use groq_api_rust::AsyncGroqClient;

#[derive(Clone)]
pub struct GroqClient(pub Arc<AsyncGroqClient>);

impl GroqClient {
    pub async fn new(api_key: String) -> Self {
        let inner = AsyncGroqClient::new(api_key, None).await;
        Self(Arc::new(inner))
    }
}
