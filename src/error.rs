pub type Result<T> = std::result::Result<T, CrawlerError>;

#[derive(Debug, thiserror::Error)]
pub enum CrawlerError {
    #[error("http request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("invalid url: {0}")]
    Url(#[from] url::ParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config parse error: {0}")]
    Config(#[from] toml::de::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Message(String),
}
