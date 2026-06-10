use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unknown runtime: {0}")]
    Unknown(String),
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub trait Runtime: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn endoflife_id(&self) -> &str;
    fn changelog_url(&self, cycle: &str) -> Option<String>;
}
