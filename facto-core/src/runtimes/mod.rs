pub mod catalog;
pub mod endoflife;
pub mod manager;
mod trait_def;

pub use trait_def::*;

use crate::http::{MAX_RESPONSE_BYTES, bytes_with_limit};

/// Read a response body with the default size cap and deserialize as JSON.
pub(crate) async fn bounded_json<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> RuntimeResult<T> {
    let bytes = bytes_with_limit(resp, MAX_RESPONSE_BYTES)
        .await
        .map_err(|e| RuntimeError::Parse(e.to_string()))?;
    serde_json::from_slice::<T>(&bytes)
        .map_err(|e| RuntimeError::Parse(format!("json decode: {e}")))
}
