use anyhow::Context;
use tracing_subscriber::{fmt, EnvFilter};
use url::Url;
use webcrawler::{douban, jd, Crawler, CrawlerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统。后续如果爬取过程变复杂，可以直接用 tracing 输出调试信息。
    init_tracing();

    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .context("usage: cargo run -- <url>|douban-top250|douban-book-top250|jd-book-prices [input.json] [output.json] [limit]")?;

    // 优先读取配置文件；如果配置文件不存在或格式错误，就使用内置默认值。
    // 这样项目刚创建时可以直接运行，后续也方便通过配置文件调整爬虫行为。
    let config = CrawlerConfig::from_file("config/default.toml").unwrap_or_default();
    let crawler = Crawler::new(config)?;

    if command == "douban-top250" {
        let output = args
            .next()
            .unwrap_or_else(|| "output/douban_top250.json".to_string());

        // 豆瓣 Top250 是一个分页列表，这里会自动请求 start=0、25、...、225。
        let movies = douban::crawl_top250(&crawler).await?;
        douban::save_movies_json(&output, &movies)?;

        println!("saved {} movies to {}", movies.len(), output);
        for movie in movies.iter().take(10) {
            println!(
                "{:>3}. {} {}",
                movie.rank,
                movie.title,
                movie
                    .rating
                    .map(|rating| format!("({rating})"))
                    .unwrap_or_default()
            );
        }

        return Ok(());
    }

    if command == "douban-book-top250" {
        let output = args
            .next()
            .unwrap_or_else(|| "output/douban_book_top250.json".to_string());

        // 豆瓣图书 Top250 会自动请求 start=0、25、...、225。
        let books = douban::crawl_book_top250(&crawler).await?;
        douban::save_books_json(&output, &books)?;

        println!("saved {} books to {}", books.len(), output);
        for book in books.iter().take(10) {
            println!(
                "{:>3}. {} {}",
                book.rank,
                book.title,
                book.rating
                    .map(|rating| format!("({rating})"))
                    .unwrap_or_default()
            );
        }

        return Ok(());
    }

    if command == "jd-book-prices" {
        let input = args
            .next()
            .unwrap_or_else(|| "output/douban_book_top250.json".to_string());
        let output = args
            .next()
            .unwrap_or_else(|| "output/jd_book_prices_top10.json".to_string());
        let limit = args
            .next()
            .map(|value| value.parse::<usize>())
            .transpose()
            .context("limit must be a positive number")?
            .unwrap_or(10);

        let books = jd::read_douban_books_json(&input)?;
        let selected_books = books.into_iter().take(limit).collect::<Vec<_>>();
        let results = jd::crawl_book_prices(&crawler, &selected_books).await?;
        jd::save_book_prices_json(&output, &results)?;

        println!(
            "saved {} jd book price results to {}",
            results.len(),
            output
        );
        for result in &results {
            if let Some(item) = &result.item {
                println!(
                    "{:>3}. {} => {} {}",
                    result.rank,
                    result.douban_title,
                    item.price
                        .as_ref()
                        .map(|price| format!("¥{price}"))
                        .unwrap_or_else(|| "(no price)".to_string()),
                    item.title
                );
            } else {
                println!(
                    "{:>3}. {} => {}",
                    result.rank,
                    result.douban_title,
                    result.error.as_deref().unwrap_or("no jd item found")
                );
            }
        }

        return Ok(());
    }

    // 从命令行读取起始 URL，例如：
    // cargo run -- https://www.rust-lang.org
    // 先把字符串解析成 Url 类型，避免后面请求阶段才发现地址格式不合法。
    let start_url = Url::parse(&command).context("invalid start url")?;

    // 当前版本先抓取单个页面。之后可以在这里扩展成队列、深度控制和去重逻辑。
    let page = crawler.fetch(start_url).await?;

    // 打印页面的基础信息，方便确认请求和解析流程是否正常。
    println!("url: {}", page.url);
    println!("status: {}", page.status);
    println!("title: {}", page.title().as_deref().unwrap_or("(no title)"));
    println!("links:");

    // links() 会把页面中的相对链接转换成绝对链接，并过滤掉非 http/https 链接。
    for link in page.links() {
        println!("- {link}");
    }

    Ok(())
}

fn init_tracing() {
    // 支持通过环境变量 RUST_LOG 控制日志等级，例如：
    // $env:RUST_LOG="debug"; cargo run -- https://www.rust-lang.org
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).init();
}
