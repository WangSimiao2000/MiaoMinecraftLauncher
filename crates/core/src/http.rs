use anyhow::Result;
use serde::de::DeserializeOwned;

#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T>;
    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

pub struct ReqwestClient {
    inner: reqwest::Client,
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HttpClient for ReqwestClient {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T> {
        let resp = self.inner.get(url).send().await?.error_for_status()?;
        let data = resp.json().await?;
        Ok(data)
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.inner.get(url).send().await?.error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }
}

#[async_trait::async_trait]
impl HttpClient for reqwest::Client {
    async fn get_json<T: DeserializeOwned + Send>(&self, url: &str) -> Result<T> {
        let resp = self.get(url).send().await?.error_for_status()?;
        let data = resp.json().await?;
        Ok(data)
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.get(url).send().await?.error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }
}
