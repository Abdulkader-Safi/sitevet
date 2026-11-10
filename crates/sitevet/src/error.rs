/// Errors that stop an audit. Problems on single pages never land here: they become
/// [`crate::PageResult::error`] or findings on that page.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("invalid URL `{url}`: {reason}")]
    InvalidUrl { url: String, reason: String },
    #[error("invalid option `{field}`: {reason}")]
    InvalidOptions { field: String, reason: String },
    #[error("could not fetch the start page {url}: {source}")]
    StartPageUnreachable {
        url: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("the start page {url} is not HTML (content type `{content_type}`)")]
    NotHtml { url: String, content_type: String },
    #[error(
        "no Chrome or Chromium found (tried {}). Install Google Chrome or Chromium, or set `chrome_path` (or SITEVET_CHROME) to its binary",
        tried.join(", ")
    )]
    BrowserNotFound { tried: Vec<String> },
    #[error("the browser failed: {0}")]
    BrowserFailed(String),
    #[error("history: {0}")]
    History(String),
    #[error("the audit was cancelled")]
    Cancelled,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
