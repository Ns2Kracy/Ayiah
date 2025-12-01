use crate::scraper::{Result, ScraperError};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;

/// HTTP client wrapper for providers
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    base_url: String,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .user_agent("Ayiah/0.1.0")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.into(),
        }
    }

    /// Get the underlying reqwest client
    #[must_use] 
    pub const fn inner(&self) -> &Client {
        &self.client
    }

    /// Build full URL from endpoint
    #[must_use] 
    pub fn url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint)
    }

    /// Execute GET request and parse JSON response
    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = self.url(endpoint);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(ScraperError::Network)?;

        Self::handle_response(response).await
    }

    /// Execute GET request with query parameters
    pub async fn get_with_params<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let url = self.url(endpoint);
        let response = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await
            .map_err(ScraperError::Network)?;

        Self::handle_response(response).await
    }

    /// Execute POST request with JSON body
    pub async fn post_json<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.url(endpoint);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .map_err(ScraperError::Network)?;

        Self::handle_response(response).await
    }

    /// Handle response and parse JSON
    async fn handle_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            let message = response.text().await.unwrap_or_default();

            return Err(ScraperError::Api {
                status: status_code,
                message,
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ScraperError::Parse(format!("JSON parse error: {e}")))
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new("")
    }
}
