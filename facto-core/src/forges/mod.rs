pub mod codeberg;
pub mod github;
pub mod gitlab;
pub mod manager;
mod trait_def;

pub use trait_def::*;

use crate::http::{MAX_RESPONSE_BYTES, bytes_with_limit};

/// Read a response body with the default size cap and deserialize as JSON.
/// Use this instead of `resp.json()` to prevent allocator DoS.
pub(crate) async fn bounded_json<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> ForgeResult<T> {
    let bytes = bytes_with_limit(resp, MAX_RESPONSE_BYTES)
        .await
        .map_err(|e| ForgeError::Parse(e.to_string()))?;
    serde_json::from_slice::<T>(&bytes).map_err(|e| ForgeError::Parse(format!("json decode: {e}")))
}
