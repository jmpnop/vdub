pub mod chat;

use reqwest::Client;

#[derive(Debug, Clone)]
pub struct OpenAiClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub http: Client,
}

impl OpenAiClient {
    pub fn new(base_url: &str, api_key: &str, model: &str, proxy: Option<&str>) -> Self {
        let mut builder = Client::builder();
        if let Some(proxy_url) = proxy {
            if !proxy_url.is_empty() {
                if let Ok(p) = reqwest::Proxy::all(proxy_url) {
                    builder = builder.proxy(p);
                }
            }
        }
        let http = builder.build().unwrap_or_default();

        let base = if base_url.is_empty() {
            "https://api.openai.com/v1".to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Self {
            base_url: base,
            api_key: api_key.to_string(),
            model: model.to_string(),
            http,
        }
    }
}
