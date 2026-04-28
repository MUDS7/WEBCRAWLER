use reqwest::{
    header::{HeaderMap, ACCEPT, ACCEPT_LANGUAGE},
    Client,
};
use url::Url;

use crate::{config::CrawlerConfig, crawler::Page, Result};

#[derive(Clone)]
pub struct Crawler {
    client: Client,
}

impl Crawler {
    pub fn new(config: CrawlerConfig) -> Result<Self> {
        let request_timeout = config.request_timeout();
        let client = Client::builder()
            .user_agent(config.user_agent)
            .default_headers(
                [
                    (
                        ACCEPT,
                        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                    ),
                    (ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8"),
                ]
                .into_iter()
                .map(|(name, value)| (name, value.parse().expect("valid static header value")))
                .collect(),
            )
            .timeout(request_timeout)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        Ok(Self { client })
    }

    pub async fn fetch(&self, url: Url) -> Result<Page> {
        let response = self.client.get(url).send().await?;
        Self::read_page(response).await
    }

    pub async fn fetch_with_headers(&self, url: Url, headers: HeaderMap) -> Result<Page> {
        let response = self.client.get(url).headers(headers).send().await?;
        Self::read_page(response).await
    }

    async fn read_page(response: reqwest::Response) -> Result<Page> {
        let status = response.status();
        let final_url = response.url().clone();
        let body = response.text().await?;

        Ok(Page::new(final_url, status, body))
    }
}
