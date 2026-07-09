use std::collections::HashMap;

use scraper::{Html, Selector};
use url::Url;

use crate::error::IngestError;

pub fn version() -> &'static str {
    "scraper module 0.1.0"
}

const ALLOWED_CONTENT_TYPES: &[&str] = &["text/html", "text/plain", "application/xhtml+xml"];

struct RobotsRules {
    disallows: Vec<String>,
    allows: Vec<String>,
}

impl RobotsRules {
    fn is_allowed(&self, path: &str) -> bool {
        let mut best_disallow: Option<usize> = None;
        let mut best_allow: Option<usize> = None;

        for rule in &self.allows {
            if path.starts_with(rule) {
                let prev = best_allow.unwrap_or(0);
                if rule.len() > prev {
                    best_allow = Some(rule.len());
                }
            }
        }
        for rule in &self.disallows {
            if path.starts_with(rule) {
                let prev = best_disallow.unwrap_or(0);
                if rule.len() > prev {
                    best_disallow = Some(rule.len());
                }
            }
        }

        match (best_allow, best_disallow) {
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(a_len), Some(d_len)) => a_len >= d_len,
            (None, None) => true,
        }
    }
}

pub struct ScraperAdapter {
    client: reqwest::Client,
    user_agent: String,
    robots_cache: HashMap<String, RobotsRules>,
}

impl ScraperAdapter {
    pub fn new(user_agent: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            user_agent,
            robots_cache: HashMap::new(),
        }
    }

    pub async fn fetch_text(&mut self, url: &str) -> Result<String, IngestError> {
        let parsed = Url::parse(url).map_err(IngestError::UrlParse)?;

        if !self.check_robots(url).await? {
            return Err(IngestError::RobotsTxt(format!(
                "path {} is disallowed by robots.txt",
                parsed.path()
            )));
        }

        let resp = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(IngestError::HttpStatus { status, body });
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        let is_allowed = ALLOWED_CONTENT_TYPES
            .iter()
            .any(|&allowed| content_type.starts_with(allowed));

        if !is_allowed {
            return Err(IngestError::ContentType(content_type));
        }

        let body = resp.text().await?;

        if content_type.starts_with("text/plain") {
            Ok(body)
        } else {
            Ok(extract_visible_text(&body))
        }
    }

    async fn check_robots(&mut self, url: &str) -> Result<bool, IngestError> {
        let parsed = Url::parse(url).map_err(IngestError::UrlParse)?;
        let host = parsed.host_str().unwrap_or("localhost");
        let origin = match parsed.port() {
            Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
            None => format!("{}://{}", parsed.scheme(), host),
        };

        if !self.robots_cache.contains_key(&origin) {
            let rules = self.fetch_robots(&origin).await?;
            self.robots_cache.insert(origin.clone(), rules);
        }

        let rules = self.robots_cache.get(&origin).unwrap();
        Ok(rules.is_allowed(parsed.path()))
    }

    async fn fetch_robots(&mut self, origin: &str) -> Result<RobotsRules, IngestError> {
        let robots_url = format!("{origin}/robots.txt");
        let resp = self.client.get(&robots_url).send().await;

        let body = match resp {
            Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
            _ => {
                return Ok(RobotsRules {
                    disallows: vec![],
                    allows: vec![],
                });
            }
        };

        Ok(parse_robots_txt(&body))
    }
}

fn parse_robots_txt(body: &str) -> RobotsRules {
    let mut disallows: Vec<String> = Vec::new();
    let mut allows: Vec<String> = Vec::new();
    let mut current_agent: Option<String> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if lower.starts_with("user-agent:") {
            let agent = trimmed["user-agent:".len()..].trim().to_string();
            current_agent = Some(agent);
        } else if lower.starts_with("disallow:") {
            if current_agent.as_deref() == Some("*") {
                let path = trimmed["disallow:".len()..].trim().to_string();
                if !path.is_empty() {
                    disallows.push(path);
                }
            }
        } else if lower.starts_with("allow:") && current_agent.as_deref() == Some("*") {
            let path = trimmed["allow:".len()..].trim().to_string();
            if !path.is_empty() {
                allows.push(path);
            }
        }
    }

    RobotsRules { disallows, allows }
}

pub fn extract_visible_text(html: &str) -> String {
    let document = Html::parse_document(html);
    let script_selector = Selector::parse("script, style").unwrap();
    let body_selector = Selector::parse("body").unwrap();

    let body = match document.select(&body_selector).next() {
        Some(b) => b,
        None => return String::new(),
    };

    collect_text_skipping_script_style(&body, &script_selector)
}

fn collect_text_skipping_script_style(
    node: &scraper::element_ref::ElementRef,
    skip_selector: &Selector,
) -> String {
    use scraper::Node;

    let mut result = String::new();

    if skip_selector.matches(node) {
        return result;
    }

    for child in node.children() {
        match child.value() {
            Node::Text(text) => {
                let t = &text.text;
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(trimmed);
                }
            }
            _ => {
                if let Some(child_ref) = scraper::element_ref::ElementRef::wrap(child) {
                    if skip_selector.matches(&child_ref) {
                        continue;
                    }
                    let child_text = collect_text_skipping_script_style(&child_ref, skip_selector);
                    if !child_text.is_empty() {
                        if !result.is_empty() {
                            result.push(' ');
                        }
                        result.push_str(&child_text);
                    }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn should_extract_visible_text_from_html() {
        let mock_server = MockServer::start().await;

        let html = r#"<!DOCTYPE html><html><body><h1>Hello</h1><p>World</p></body></html>"#;

        Mock::given(method("GET"))
            .and(path("/page"))
            .and(header("User-Agent", "test-agent"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(html.to_string().into_bytes(), "text/html"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let mut adapter = ScraperAdapter::new("test-agent".into());
        let result = adapter
            .fetch_text(&format!("{}/page", mock_server.uri()))
            .await;

        assert!(result.is_ok(), "expected ok, got {result:?}");
        let text = result.unwrap();
        assert!(text.contains("Hello"), "text should contain Hello");
        assert!(text.contains("World"), "text should contain World");
    }

    #[tokio::test]
    async fn should_reject_disallowed_content_type() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/file.pdf"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw("%PDF-content".to_string().into_bytes(), "application/pdf"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let mut adapter = ScraperAdapter::new("test-agent".into());
        let result = adapter
            .fetch_text(&format!("{}/file.pdf", mock_server.uri()))
            .await;

        assert!(matches!(result, Err(IngestError::ContentType(_))));
    }

    #[tokio::test]
    async fn should_return_error_on_http_500() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let mut adapter = ScraperAdapter::new("test-agent".into());
        let result = adapter
            .fetch_text(&format!("{}/error", mock_server.uri()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_honor_robots_txt_disallow() {
        let mock_server = MockServer::start().await;

        let robots = "User-agent: *\nDisallow: /private/\n";
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(robots)
                    .insert_header("content-type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let mut adapter = ScraperAdapter::new("test-agent".into());
        let result = adapter
            .fetch_text(&format!("{}/private/data", mock_server.uri()))
            .await;

        assert!(matches!(result, Err(IngestError::RobotsTxt(_))));
    }

    #[tokio::test]
    async fn should_allow_when_robots_txt_allows() {
        let mock_server = MockServer::start().await;

        let robots = "User-agent: *\nDisallow: /private/\n";
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(robots)
                    .insert_header("content-type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/public/hello"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                "<html><body>public</body></html>".to_string().into_bytes(),
                "text/html",
            ))
            .mount(&mock_server)
            .await;

        let mut adapter = ScraperAdapter::new("test-agent".into());
        let result = adapter
            .fetch_text(&format!("{}/public/hello", mock_server.uri()))
            .await;

        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[tokio::test]
    async fn should_allow_when_robots_txt_missing() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/any/page"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                "<html><body>ok</body></html>".to_string().into_bytes(),
                "text/html",
            ))
            .mount(&mock_server)
            .await;

        let mut adapter = ScraperAdapter::new("test-agent".into());
        let result = adapter
            .fetch_text(&format!("{}/any/page", mock_server.uri()))
            .await;

        assert!(result.is_ok(), "expected ok, got {result:?}");
    }

    #[tokio::test]
    async fn should_strip_script_and_style_from_html() {
        let html = r#"<!DOCTYPE html><html><head><style>.red{color:red}</style></head><body><h1>Title</h1><script>alert("x")</script><p>Content</p></body></html>"#;

        let text = extract_visible_text(html);
        assert!(text.contains("Title"), "should contain Title");
        assert!(text.contains("Content"), "should contain Content");
        assert!(!text.contains("alert"), "should not contain script content");
        assert!(!text.contains(".red"), "should not contain style content");
    }

    #[test]
    fn should_parse_robots_txt_with_disallow_all() {
        let body = "User-agent: *\nDisallow: /\n";
        let rules = parse_robots_txt(body);
        assert!(!rules.is_allowed("/any/path"));
    }

    #[test]
    fn should_parse_robots_txt_with_allow_all_when_empty() {
        let body = "";
        let rules = parse_robots_txt(body);
        assert!(rules.is_allowed("/any/path"));
    }

    #[test]
    fn should_parse_robots_txt_allowing_specific_path() {
        let body = "User-agent: *\nDisallow: /private/\nAllow: /public/\n";
        let rules = parse_robots_txt(body);
        assert!(rules.is_allowed("/public/hello"));
    }

    #[test]
    fn should_extract_visible_text_with_body_selector() {
        let html = "<html><body>Hello <b>World</b></body></html>";
        let text = extract_visible_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn should_return_empty_string_when_no_body() {
        let html = "<html></html>";
        let text = extract_visible_text(html);
        assert_eq!(text, "");
    }
}
