use reqwest::StatusCode;
use url::Url;

use crate::parser;

#[derive(Debug, Clone)]
pub struct Page {
    // 页面最终 URL。它可能是重定向之后的地址，而不是最初传入的地址。
    pub url: Url,
    // HTTP 状态码，例如 200、404、500。后续可以据此决定是否解析或重试。
    pub status: StatusCode,
    // 页面 HTML 原文。当前先放在内存里，之后可以按需要落盘或流式处理。
    pub body: String,
}

impl Page {
    pub fn new(url: Url, status: StatusCode, body: String) -> Self {
        Self { url, status, body }
    }

    pub fn title(&self) -> Option<String> {
        // 把 HTML 解析细节封装在 parser 模块里，Page 只暴露更高层的语义方法。
        parser::extract_title(&self.body)
    }

    pub fn links(&self) -> Vec<Url> {
        // 提取链接时传入当前页面 URL，用它来把 /path 这类相对链接补全。
        parser::extract_links(&self.url, &self.body)
    }
}
