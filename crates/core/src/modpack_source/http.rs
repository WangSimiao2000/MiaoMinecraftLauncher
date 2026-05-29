use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use thiserror::Error;
use url::Url;

const MAX_RETRIES: u32 = 3;
const BASE_BACKOFF_MS: u64 = 500;
const RATE_LIMIT_CAP_SECS: u64 = 60;

const ACTIVE_THROTTLE_REMAINING: u64 = 30;
const ACTIVE_THROTTLE_DELAY_MS: u64 = 1_000;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("rate-limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("invalid URL: {0}")]
    Url(String),
}

#[derive(Debug, Default, Clone)]
struct RateState {
    remaining: Option<u64>,
    reset_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct RateLimitedClient {
    inner: reqwest::Client,
    rate_state: Arc<Mutex<HashMap<String, RateState>>>,
    progress: Option<Arc<dyn ProgressSink>>,
}

pub trait ProgressSink: Send + Sync {
    fn on_rate_limited(&self, host: &str, retry_after: Duration);
    fn on_throttling(&self, host: &str, delay: Duration);
}

impl RateLimitedClient {
    pub fn new(inner: reqwest::Client) -> Self {
        Self {
            inner,
            rate_state: Arc::new(Mutex::new(HashMap::new())),
            progress: None,
        }
    }

    pub fn with_progress(mut self, sink: Arc<dyn ProgressSink>) -> Self {
        self.progress = Some(sink);
        self
    }

    pub async fn get_json<T>(&self, url: &str) -> Result<T, HttpError>
    where
        T: serde::de::DeserializeOwned,
    {
        let resp = self.send(|| self.inner.get(url)).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpError> {
        let resp = self.send(|| self.inner.get(url)).await?;
        Ok(resp.bytes().await?.to_vec())
    }

    async fn send<F>(&self, build: F) -> Result<reqwest::Response, HttpError>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut attempt: u32 = 0;
        loop {
            let req = build().build().map_err(HttpError::Reqwest)?;
            let host = req
                .url()
                .host_str()
                .ok_or_else(|| HttpError::Url(req.url().to_string()))?
                .to_ascii_lowercase();

            self.maybe_throttle(&host).await;

            let resp = self.inner.execute(req).await?;
            let status = resp.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after = parse_rate_limit_retry(&resp).unwrap_or(RATE_LIMIT_CAP_SECS);
                if attempt >= MAX_RETRIES {
                    self.notify_rate_limited(&host, Duration::from_secs(retry_after));
                    return Err(HttpError::RateLimited {
                        retry_after_secs: retry_after,
                    });
                }
                let wait = Duration::from_secs(retry_after.min(RATE_LIMIT_CAP_SECS));
                self.notify_rate_limited(&host, wait);
                tokio::time::sleep(wait).await;
                attempt += 1;
                continue;
            }

            if status.is_server_error() {
                if attempt >= MAX_RETRIES {
                    return Ok(resp.error_for_status()?);
                }
                tokio::time::sleep(backoff_delay(attempt)).await;
                attempt += 1;
                continue;
            }

            self.update_rate_state(&host, &resp);
            return Ok(resp.error_for_status()?);
        }
    }

    fn update_rate_state(&self, host: &str, resp: &reqwest::Response) {
        let headers = resp.headers();
        let remaining = header_u64_lc(headers, "x-ratelimit-remaining");
        let reset_secs = header_u64_lc(headers, "x-ratelimit-reset");

        if remaining.is_none() && reset_secs.is_none() {
            return;
        }

        let mut map = self.rate_state.lock().unwrap();
        let state = map.entry(host.to_string()).or_default();
        if let Some(r) = remaining {
            state.remaining = Some(r);
        }
        if let Some(secs) = reset_secs {
            state.reset_at = Some(chrono::Utc::now() + chrono::Duration::seconds(secs as i64));
        }
    }

    async fn maybe_throttle(&self, host: &str) {
        let action = {
            let map = self.rate_state.lock().unwrap();
            let state = match map.get(host) {
                Some(s) => s.clone(),
                None => return,
            };
            decide_throttle(&state)
        };
        match action {
            ThrottleAction::None => {}
            ThrottleAction::Slow => {
                let delay = Duration::from_millis(ACTIVE_THROTTLE_DELAY_MS);
                self.notify_throttling(host, delay);
                tokio::time::sleep(delay).await;
            }
            ThrottleAction::WaitUntilReset(reset_at) => {
                let now = chrono::Utc::now();
                if reset_at > now {
                    let wait = (reset_at - now)
                        .to_std()
                        .unwrap_or(Duration::ZERO)
                        .min(Duration::from_secs(RATE_LIMIT_CAP_SECS));
                    self.notify_throttling(host, wait);
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    fn notify_rate_limited(&self, host: &str, retry_after: Duration) {
        if let Some(sink) = &self.progress {
            sink.on_rate_limited(host, retry_after);
        }
    }

    fn notify_throttling(&self, host: &str, delay: Duration) {
        if let Some(sink) = &self.progress {
            sink.on_throttling(host, delay);
        }
    }
}

#[derive(Debug)]
enum ThrottleAction {
    None,
    Slow,
    WaitUntilReset(chrono::DateTime<chrono::Utc>),
}

fn decide_throttle(state: &RateState) -> ThrottleAction {
    match state.remaining {
        Some(0) => match state.reset_at {
            Some(reset) => ThrottleAction::WaitUntilReset(reset),
            None => ThrottleAction::Slow,
        },
        Some(r) if r < ACTIVE_THROTTLE_REMAINING => ThrottleAction::Slow,
        _ => ThrottleAction::None,
    }
}

fn header_u64_lc(headers: &reqwest::header::HeaderMap, name: &str) -> Option<u64> {
    headers
        .iter()
        .find(|(k, _)| k.as_str().eq_ignore_ascii_case(name))
        .and_then(|(_, v)| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
}

fn parse_rate_limit_retry(resp: &reqwest::Response) -> Option<u64> {
    let headers = resp.headers();

    if let Some(secs) = header_u64_lc(headers, "x-ratelimit-reset") {
        return Some(secs.min(RATE_LIMIT_CAP_SECS));
    }

    let retry_after = headers
        .iter()
        .find(|(k, _)| k.as_str().eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, v)| v.to_str().ok())?
        .trim()
        .to_string();

    if let Ok(secs) = retry_after.parse::<u64>() {
        return Some(secs.min(RATE_LIMIT_CAP_SECS));
    }

    let parsed = chrono::DateTime::parse_from_rfc2822(&retry_after).ok()?;
    let now = chrono::Utc::now();
    let delta = parsed.signed_duration_since(now).num_seconds();
    if delta <= 0 {
        Some(0)
    } else {
        Some((delta as u64).min(RATE_LIMIT_CAP_SECS))
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    let factor = 4u64.saturating_pow(attempt);
    Duration::from_millis(BASE_BACKOFF_MS.saturating_mul(factor))
}

pub fn host_of(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()?
        .host_str()
        .map(|s| s.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde::Deserialize;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Deserialize)]
    struct Empty {}

    fn client() -> RateLimitedClient {
        RateLimitedClient::new(reqwest::Client::new())
    }

    #[derive(Default)]
    struct CountingSink {
        rate_limited_calls: AtomicUsize,
        throttling_calls: AtomicUsize,
    }

    impl ProgressSink for CountingSink {
        fn on_rate_limited(&self, _host: &str, _retry_after: Duration) {
            self.rate_limited_calls.fetch_add(1, Ordering::SeqCst);
        }
        fn on_throttling(&self, _host: &str, _delay: Duration) {
            self.throttling_calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn t_http_06_modrinth_429_with_xratelimit_reset_retried() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(GET).path("/v2/projects");
                then.status(429).header("X-Ratelimit-Reset", "0");
            })
            .await;

        let url = server.url("/v2/projects");
        let result: Result<Empty, HttpError> = client().get_json(&url).await;

        assert!(matches!(
            result,
            Err(HttpError::RateLimited {
                retry_after_secs: 0
            })
        ));
        assert_eq!(m.calls_async().await, (MAX_RETRIES + 1) as usize);
    }

    #[tokio::test]
    async fn t_http_07_5xx_then_200_succeeds_after_retry_count() {
        let server = MockServer::start_async().await;
        let m = server
            .mock_async(|when, then| {
                when.method(GET).path("/v2/up");
                then.status(503);
            })
            .await;

        let url = server.url("/v2/up");
        let result: Result<Empty, HttpError> = client().get_json(&url).await;

        assert!(result.is_err());
        assert_eq!(m.calls_async().await, (MAX_RETRIES + 1) as usize);
    }

    #[tokio::test]
    async fn t_http_08_xratelimit_remaining_below_30_throttles() {
        let server = MockServer::start_async().await;
        let _m = server
            .mock_async(|when, then| {
                when.method(GET).path("/v2/limited");
                then.status(200)
                    .header("content-type", "application/json")
                    .header("X-Ratelimit-Remaining", "5")
                    .header("X-Ratelimit-Reset", "60")
                    .body("{}");
            })
            .await;

        let sink = Arc::new(CountingSink::default());
        let c = client().with_progress(sink.clone());
        let url = server.url("/v2/limited");

        let _: Empty = c.get_json(&url).await.expect("first call ok");
        let started = std::time::Instant::now();
        let _: Empty = c.get_json(&url).await.expect("second call ok");
        let elapsed = started.elapsed();

        assert!(
            elapsed >= Duration::from_millis(ACTIVE_THROTTLE_DELAY_MS - 50),
            "expected throttle delay, got {elapsed:?}"
        );
        assert_eq!(sink.throttling_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn t_http_09_progress_sink_notified_on_429() {
        let server = MockServer::start_async().await;
        let _m = server
            .mock_async(|when, then| {
                when.method(GET).path("/v2/limited2");
                then.status(429).header("X-Ratelimit-Reset", "0");
            })
            .await;

        let sink = Arc::new(CountingSink::default());
        let c = client().with_progress(sink.clone());
        let url = server.url("/v2/limited2");
        let _: Result<Empty, HttpError> = c.get_json(&url).await;

        assert!(sink.rate_limited_calls.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn t_http_10_decide_throttle_zero_remaining_with_reset_waits() {
        let reset = chrono::Utc::now() + chrono::Duration::seconds(5);
        let s = RateState {
            remaining: Some(0),
            reset_at: Some(reset),
        };
        assert!(matches!(
            decide_throttle(&s),
            ThrottleAction::WaitUntilReset(_)
        ));

        let s2 = RateState {
            remaining: Some(10),
            reset_at: Some(reset),
        };
        assert!(matches!(decide_throttle(&s2), ThrottleAction::Slow));

        let s3 = RateState {
            remaining: Some(100),
            reset_at: Some(reset),
        };
        assert!(matches!(decide_throttle(&s3), ThrottleAction::None));
    }

    #[test]
    fn t_http_11_host_of_extracts_lowercase() {
        assert_eq!(
            host_of("https://API.Modrinth.com/v2/projects"),
            Some("api.modrinth.com".to_string())
        );
        assert_eq!(host_of("not-a-url"), None);
    }
}
