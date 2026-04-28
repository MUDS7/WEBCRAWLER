use scraper::{Html, Selector};
use url::Url;

pub fn extract_title(html: &str) -> Option<String> {
    // scraper 会把 HTML 字符串解析成可查询的 DOM 树。
    let document = Html::parse_document(html);

    // CSS 选择器解析理论上可能失败，这里失败时直接返回 None。
    let selector = Selector::parse("title").ok()?;

    document
        .select(&selector)
        .next()
        // title 标签里可能包含多个文本节点，合并后再去掉首尾空白。
        .map(|title| title.text().collect::<String>().trim().to_string())
        .filter(|title| !title.is_empty())
}

pub fn extract_links(base_url: &Url, html: &str) -> Vec<Url> {
    let document = Html::parse_document(html);

    // 只选择带 href 属性的 a 标签，这是网页爬虫最常见的下一跳入口。
    let selector = match Selector::parse("a[href]") {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };

    document
        .select(&selector)
        // 取出 href 原始值，可能是绝对链接、相对链接、锚点或 javascript:。
        .filter_map(|node| node.value().attr("href"))
        // 用当前页面地址作为基准，把相对链接转换成绝对链接。
        .filter_map(|href| base_url.join(href).ok())
        // 只保留真正可请求的网页链接，过滤 mailto:、javascript:、tel: 等协议。
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .collect()
}
