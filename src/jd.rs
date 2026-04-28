use std::{env, fs, path::Path};

use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use url::Url;

use crate::{douban::DoubanBook, Crawler, CrawlerError, Result};

const JD_SEARCH_URL: &str = "https://search.jd.com/Search";
const JD_PRICE_URL: &str = "https://p.3.cn/prices/mgets";

#[derive(Debug, Clone, Serialize)]
pub struct JdBookPriceResult {
    pub rank: usize,
    pub douban_title: String,
    pub douban_publish_info: String,
    pub search_url: String,
    pub item: Option<JdSearchItem>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JdSearchItem {
    pub sku: String,
    pub title: String,
    pub shop: Option<String>,
    pub item_url: String,
    pub price: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JdPrice {
    id: String,
    p: String,
}

pub fn read_douban_books_json(path: impl AsRef<Path>) -> Result<Vec<DoubanBook>> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_book_prices_json(path: impl AsRef<Path>, results: &[JdBookPriceResult]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(results)?;
    fs::write(path, content)?;

    Ok(())
}

pub async fn crawl_book_prices(
    crawler: &Crawler,
    books: &[DoubanBook],
) -> Result<Vec<JdBookPriceResult>> {
    let mut results = Vec::with_capacity(books.len());

    for book in books {
        let search_url = build_search_url(&book.title)?;
        let result = match crawl_one_book_price(crawler, book, search_url.clone()).await {
            Ok(item) => JdBookPriceResult {
                rank: book.rank,
                douban_title: book.title.clone(),
                douban_publish_info: book.publish_info.clone(),
                search_url: search_url.to_string(),
                item,
                error: None,
            },
            Err(err) => JdBookPriceResult {
                rank: book.rank,
                douban_title: book.title.clone(),
                douban_publish_info: book.publish_info.clone(),
                search_url: search_url.to_string(),
                item: None,
                error: Some(err.to_string()),
            },
        };

        results.push(result);
        sleep(Duration::from_millis(1500)).await;
    }

    Ok(results)
}

async fn crawl_one_book_price(
    crawler: &Crawler,
    book: &DoubanBook,
    search_url: Url,
) -> Result<Option<JdSearchItem>> {
    let page = fetch_jd_page(crawler, search_url).await?;
    if !page.status.is_success() {
        return Err(CrawlerError::Message(format!(
            "jd search request failed: {} {}",
            page.status, page.url
        )));
    }
    if page.url.as_str().contains("/risk_handler/") {
        return Err(CrawlerError::Message(format!(
            "jd search blocked by verification page: {}",
            page.url
        )));
    }

    let Some(mut item) = parse_first_search_item(&page.body) else {
        return Ok(None);
    };

    item.price = fetch_price(crawler, &item.sku).await?;

    if item.title.is_empty() {
        item.title = book.title.clone();
    }

    Ok(Some(item))
}

async fn fetch_price(crawler: &Crawler, sku: &str) -> Result<Option<String>> {
    let mut url = Url::parse(JD_PRICE_URL)?;
    url.query_pairs_mut()
        .append_pair("skuIds", &format!("J_{sku}"))
        .append_pair("type", "1");

    let page = fetch_jd_page(crawler, url).await?;
    if !page.status.is_success() {
        return Err(CrawlerError::Message(format!(
            "jd price request failed: {} {}",
            page.status, page.url
        )));
    }

    let prices: Vec<JdPrice> = serde_json::from_str(&page.body)?;
    Ok(prices
        .into_iter()
        .find(|price| price.id == format!("J_{sku}"))
        .map(|price| price.p)
        .filter(|price| price != "-1"))
}

async fn fetch_jd_page(crawler: &Crawler, url: Url) -> Result<crate::Page> {
    let Some(cookie) = jd_cookie() else {
        return crawler.fetch(url).await;
    };

    let mut headers = HeaderMap::new();
    let cookie = HeaderValue::from_str(&cookie)
        .map_err(|err| CrawlerError::Message(format!("invalid JD_COOKIE header value: {err}")))?;
    headers.insert(COOKIE, cookie);
    headers.insert(REFERER, HeaderValue::from_static("https://www.jd.com/"));

    crawler.fetch_with_headers(url, headers).await
}

fn jd_cookie() -> Option<String> {
    env::var("JD_COOKIE")
        .ok()
        .map(|cookie| cookie.trim().to_string())
        .filter(|cookie| !cookie.is_empty())
}

fn build_search_url(keyword: &str) -> Result<Url> {
    let mut url = Url::parse(JD_SEARCH_URL)?;
    url.query_pairs_mut()
        .append_pair("keyword", keyword)
        .append_pair("enc", "utf-8")
        .append_pair("wq", keyword);
    Ok(url)
}

fn parse_first_search_item(html: &str) -> Option<JdSearchItem> {
    let document = Html::parse_document(html);
    let item_selector = selector("li.gl-item");

    document
        .select(&item_selector)
        .filter_map(parse_search_item)
        .next()
}

fn parse_search_item(item: ElementRef<'_>) -> Option<JdSearchItem> {
    let sku = item.value().attr("data-sku")?.trim().to_string();
    if sku.is_empty() {
        return None;
    }

    let title = text_of(item, ".p-name em")
        .or_else(|| text_of(item, ".p-name a"))
        .unwrap_or_default();
    let shop = text_of(item, ".p-shop a").or_else(|| text_of(item, ".p-shop"));
    let item_url = item
        .select(&selector(".p-name a"))
        .next()
        .and_then(|link| link.value().attr("href"))
        .map(normalize_jd_url)
        .unwrap_or_else(|| format!("https://item.jd.com/{sku}.html"));

    Some(JdSearchItem {
        sku,
        title,
        shop,
        item_url,
        price: None,
    })
}

fn normalize_jd_url(url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("https://item.jd.com{url}")
    } else {
        url.to_string()
    }
}

fn text_of(root: ElementRef<'_>, css: &str) -> Option<String> {
    root.select(&selector(css))
        .next()
        .map(|node| normalize_text(node.text()))
        .filter(|text| !text.is_empty())
}

fn normalize_text<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    parts
        .flat_map(|part| part.split_whitespace())
        .collect::<Vec<_>>()
        .join(" ")
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("valid static css selector")
}
