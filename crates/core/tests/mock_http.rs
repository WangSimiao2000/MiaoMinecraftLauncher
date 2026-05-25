use std::collections::HashMap;
use std::sync::Mutex;

use miao_core::error::MiaoError;
use miao_core::http::HttpClient;
use serde::de::DeserializeOwned;

type Result<T> = std::result::Result<T, MiaoError>;

pub struct MockHttpClient {
    responses: Mutex<HashMap<String, String>>,
    byte_responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
            byte_responses: Mutex::new(HashMap::new()),
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

    #[allow(dead_code)]
    pub fn mock_bytes(&self, url_contains: &str, data: Vec<u8>) {
        self.byte_responses
            .lock()
            .unwrap()
            .insert(url_contains.to_string(), data);
    }
}

#[async_trait::async_trait]
impl HttpClient for MockHttpClient {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T> {
        let responses = self.responses.lock().unwrap();
        for (pattern, body) in responses.iter() {
            if url.contains(pattern) {
                return Ok(serde_json::from_str(body)?);
            }
        }
        Err(MiaoError::Other(format!(
            "No mock response for URL: {}",
            url
        )))
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let responses = self.byte_responses.lock().unwrap();
        for (pattern, data) in responses.iter() {
            if url.contains(pattern) {
                return Ok(data.clone());
            }
        }
        let json_responses = self.responses.lock().unwrap();
        for (pattern, body) in json_responses.iter() {
            if url.contains(pattern) {
                return Ok(body.as_bytes().to_vec());
            }
        }
        Err(MiaoError::Other(format!(
            "No mock byte response for URL: {}",
            url
        )))
    }
}
