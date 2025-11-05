use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// HTTP client wrapper with timeout and authentication support
pub struct HttpClient {
    client: reqwest::Client,
    timeout: Duration,
}

impl HttpClient {
    /// Create a new HTTP client with specified timeout
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    /// Fetch JSON data from URL with optional authentication token
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str, token: Option<&str>) -> Result<T> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("trotd/0.1.0"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(token) = token {
            let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
                .context("Invalid authentication token")?;
            headers.insert(AUTHORIZATION, auth_value);
        }

        let response = self
            .client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .with_context(|| format!("Failed to fetch URL: {url}"))?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("HTTP request failed with status {status}: {url}");
        }

        response
            .json::<T>()
            .await
            .with_context(|| format!("Failed to parse JSON response from {url}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HttpClient::new(6);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_get_json_with_mock() {
        // Integration tests with mockito will be added in provider tests
        // This is a placeholder for unit test structure
        let client = HttpClient::new(6).unwrap();
        assert_eq!(client.timeout.as_secs(), 6);
    }
}
