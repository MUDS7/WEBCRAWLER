use std::{fs, path::Path, time::Duration};

use serde::Deserialize;

use crate::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct CrawlerConfig {
    pub user_agent: String,
    pub request_timeout_secs: u64,
    pub max_depth: usize,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            user_agent: "webcrawler/0.1".to_string(),
            request_timeout_secs: 15,
            max_depth: 1,
        }
    }
}

impl CrawlerConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;

        Ok(config)
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }
}
