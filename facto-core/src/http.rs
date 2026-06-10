use std::time::Duration;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

/// Maximum body size accepted from any upstream HTTP response. Anything
/// larger is rejected with [`HttpClientError::BodyTooLarge`] to prevent
/// allocator DoS from a compromised upstream or malicious CDN edge.
pub const MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024; // 50 MB

/// Errors that can occur when building or using an HTTP client.
#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    #[error("failed to build HTTP client: {0}")]
    Build(#[from] reqwest::Error),
    #[error("response body exceeded {limit} bytes")]
    BodyTooLarge { limit: usize },
    #[error("failed to read response body: {0}")]
    BodyRead(reqwest::Error),
}

/// Default HTTP client with timeouts, user-agent, strict redirect policy,
/// and HTTPS-only enforcement.
pub fn default_client() -> Result<reqwest::Client, HttpClientError> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("facto/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .build()?)
}

/// HTTP client with a Bearer token for authenticated APIs.
pub fn bearer_client(token: &str) -> Result<reqwest::Client, HttpClientError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    Ok(reqwest::Client::builder()
        .user_agent(format!("facto/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .build()?)
}

/// HTTP client with a Private-Token header (GitLab).
pub fn private_token_client(token: &str) -> Result<reqwest::Client, HttpClientError> {
    let mut headers = HeaderMap::new();
    headers.insert("PRIVATE-TOKEN", HeaderValue::from_str(token)?);
    Ok(reqwest::Client::builder()
        .user_agent(format!("facto/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .build()?)
}

/// Read a response body into a `Vec<u8>`, aborting as soon as the byte
/// count exceeds `limit`. Prevents a hostile or buggy upstream from
/// OOM-killing the process by streaming an unbounded body.
///
/// The caller is responsible for JSON/UTF-8 interpretation. Use together
/// with [`MAX_RESPONSE_BYTES`] for the default cap.
pub async fn bytes_with_limit(
    mut resp: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, HttpClientError> {
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(HttpClientError::BodyRead)? {
        if buf.len() + chunk.len() > limit {
            return Err(HttpClientError::BodyTooLarge { limit });
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_client_builds() {
        default_client().unwrap();
    }

    #[tokio::test]
    async fn bearer_client_builds() {
        bearer_client("ghp_test").unwrap();
    }

    #[tokio::test]
    async fn private_token_client_builds() {
        private_token_client("glpat_test").unwrap();
    }
}
