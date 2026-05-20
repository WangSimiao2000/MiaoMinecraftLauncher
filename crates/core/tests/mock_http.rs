use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use miao_core::http::HttpClient;
use serde::de::DeserializeOwned;

pub struct MockHttpClient {
    responses: Mutex<HashMap<String, String>>,
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }
}

impl MockHttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mock_response(&self, url_contains: &str, json_body: &str) {
        self.responses
            .lock()
            .unwrap()
            .insert(url_contains.to_string(), json_body.to_string());
    }
}

#[async_trait::async_trait]
impl HttpClient for MockHttpClient {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T> {
        let responses = self.responses.lock().unwrap();
        for (pattern, body) in responses.iter() {
            if url.contains(pattern) {
                return serde_json::from_str(body)
                    .map_err(|e| anyhow!("Mock JSON parse error for {}: {}", url, e));
            }
        }
        Err(anyhow!("No mock response for URL: {}", url))
    }
}
