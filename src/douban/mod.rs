use std::{fs, path::Path};

use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use url::Url;

use crate::{Crawler, CrawlerError, Result};

const TOP250_URL: &str = "https://movie.douban.com/top250";
const BOOK_TOP250_URL: &str = "https://book.douban.com/top250";
const PAGE_SIZE: usize = 25;
const TOTAL_COUNT: usize = 250;

#[derive(Debug, Clone, Serialize)]
pub struct DoubanMovie {
    pub rank: usize,
    pub title: String,
    pub original_title: Option<String>,
    pub rating: Option<f32>,
    pub rating_people: Option<u32>,
    pub quote: Option<String>,
    pub info: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubanBook {
    pub rank: usize,
    pub title: String,
    pub rating: Option<f32>,
    pub rating_people: Option<u32>,
    pub quote: Option<String>,
    pub publish_info: String,
    pub url: String,
}

pub async fn crawl_top250(crawler: &Crawler) -> Result<Vec<DoubanMovie>> {
    let mut movies = Vec::with_capacity(TOTAL_COUNT);

    // 豆瓣 Top250 一共 10 页，每页 25 条，start 参数按 25 递增。
    for start in (0..TOTAL_COUNT).step_by(PAGE_SIZE) {
        let mut url = Url::parse(TOP250_URL)?;
        url.query_pairs_mut()
            .append_pair("start", &start.to_string())
            .append_pair("filter", "");

        let page = crawler.fetch(url).await?;
        if !page.status.is_success() {
            return Err(CrawlerError::Message(format!(
                "douban request failed: {} {}",
                page.status, page.url
            )));
        }

        let mut page_movies = parse_top250_page(&page.body);
        if page_movies.is_empty() {
            return Err(CrawlerError::Message(format!(
                "no movies parsed from douban page: {}",
                page.url
            )));
        }

        movies.append(&mut page_movies);

        // 给目标站点留一点喘息时间，也降低被反爬策略命中的概率。
        sleep(Duration::from_millis(1200)).await;
    }

    movies.sort_by_key(|movie| movie.rank);
    movies.dedup_by_key(|movie| movie.rank);

    Ok(movies)
}

pub async fn crawl_book_top250(crawler: &Crawler) -> Result<Vec<DoubanBook>> {
    let mut books = Vec::with_capacity(TOTAL_COUNT);

    // 豆瓣图书 Top250 同样是 10 页，每页 25 条，start 参数按 25 递增。
    for start in (0..TOTAL_COUNT).step_by(PAGE_SIZE) {
        let mut url = Url::parse(BOOK_TOP250_URL)?;
        url.query_pairs_mut()
            .append_pair("start", &start.to_string());

        let page = crawler.fetch(url).await?;
        if !page.status.is_success() {
            return Err(CrawlerError::Message(format!(
                "douban book request failed: {} {}",
                page.status, page.url
            )));
        }

        let mut page_books = parse_book_top250_page(&page.body, start);
        if page_books.is_empty() {
            return Err(CrawlerError::Message(format!(
                "no books parsed from douban page: {}",
                page.url
            )));
        }

        books.append(&mut page_books);

        // 图书页也加一点延时，避免连续请求太快。
        sleep(Duration::from_millis(1200)).await;
    }

    books.sort_by_key(|book| book.rank);
    books.dedup_by_key(|book| book.rank);

    Ok(books)
}

pub fn save_movies_json(path: impl AsRef<Path>, movies: &[DoubanMovie]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(movies)?;
    fs::write(path, content)?;

    Ok(())
}

pub fn save_books_json(path: impl AsRef<Path>, books: &[DoubanBook]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(books)?;
    fs::write(path, content)?;

    Ok(())
}

fn parse_top250_page(html: &str) -> Vec<DoubanMovie> {
    let document = Html::parse_document(html);
    let item_selector = selector(".grid_view .item");

    document
        .select(&item_selector)
        .filter_map(parse_movie_item)
        .collect()
}

fn parse_book_top250_page(html: &str, start: usize) -> Vec<DoubanBook> {
    let document = Html::parse_document(html);
    let item_selector = selector("tr.item");

    document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, item)| parse_book_item(item, start + index + 1))
        .collect()
}

fn parse_movie_item(item: ElementRef<'_>) -> Option<DoubanMovie> {
    let rank = text_of(item, ".pic em")?.parse().ok()?;
    let title = text_of(item, ".hd a .title")?;
    let original_title = text_of(item, ".hd a .other");
    let rating = text_of(item, ".rating_num").and_then(|value| value.parse().ok());
    let rating_people = text_of(item, ".star span:last-child").and_then(parse_people_count);
    let quote = text_of(item, ".inq");
    let info = text_of(item, ".bd p").unwrap_or_default();
    let url = item
        .select(&selector(".hd a"))
        .next()
        .and_then(|link| link.value().attr("href"))
        .unwrap_or_default()
        .to_string();

    Some(DoubanMovie {
        rank,
        title,
        original_title,
        rating,
        rating_people,
        quote,
        info,
        url,
    })
}

fn parse_book_item(item: ElementRef<'_>, rank: usize) -> Option<DoubanBook> {
    let title_link = item.select(&selector(".pl2 a")).next()?;
    let title = title_link
        .value()
        .attr("title")
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| Some(normalize_text(title_link.text())))?;

    let rating = text_of(item, ".rating_nums").and_then(|value| value.parse().ok());
    let rating_people = text_of(item, ".star .pl").and_then(parse_people_count);
    let quote = text_of(item, ".inq");
    let publish_info = text_of(item, "p.pl").unwrap_or_default();
    let url = title_link
        .value()
        .attr("href")
        .unwrap_or_default()
        .to_string();

    Some(DoubanBook {
        rank,
        title,
        rating,
        rating_people,
        quote,
        publish_info,
        url,
    })
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

fn parse_people_count(text: String) -> Option<u32> {
    let digits: String = text.chars().filter(|ch| ch.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("valid static css selector")
}
