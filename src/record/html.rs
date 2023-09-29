use super::error::RecordError;
use super::Spin;
use super::{Content, Record};
use anyhow::Result;
use reqwest;
use scraper::Selector;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct HTML {
    body: String,
    selectors: String,
}

impl HTML {
    const DEFAULT_SELECTORS: &'static str = "main, article, div.content";

    /// Create a new HTML record from a URL
    pub async fn from_url(url: &str) -> Result<HTML, RecordError> {
        let client = reqwest::ClientBuilder::new().timeout(std::time::Duration::from_secs(5)).build()?;
        let body = client.get(url).send().await?.text().await?;

        Ok(HTML {
            body,
            selectors: Self::DEFAULT_SELECTORS.to_string(),
        })
    }

    /// Create a new HTML record from a file
    pub fn from_file(path: &str) -> Result<HTML, RecordError> {
        let body = fs::read_to_string(Path::new(path))?;
        Ok(HTML {
            body,
            selectors: Self::DEFAULT_SELECTORS.to_string(),
        })
    }

    /// Set the selectors for the HTML record
    pub fn with_selectors(mut self, selectors: &str) -> HTML {
        self.selectors = selectors.to_string();
        self
    }
}

impl Spin for HTML {
    fn spin(&self) -> Result<Record> {
        let html = scraper::Html::parse_document(&self.body);

        let header_selector = Selector::parse("header, nav").unwrap();
        let metadata_selector = Selector::parse("meta").unwrap();

        let header = html.select(&header_selector).map(|element| element.inner_html()).collect::<Vec<_>>().join("\n");

        let mut metadata = String::new();
        html.select(&metadata_selector).for_each(|element| {
            if element.value().attr("name").is_some() {
                metadata.push_str(format!("{}: ", element.value().attr("name").unwrap()).as_str());
                metadata.push_str(element.value().attr("content").unwrap());
                metadata.push('\n');
            }
        });

        let content_selector = Selector::parse(self.selectors.as_str()).unwrap();
        let content = html.select(&content_selector).map(|element| element.inner_html()).collect::<Vec<_>>().join("\n");

        Ok(Record::new(Content::String(content)).with_header(header).with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_from_url() {
        let record = HTML::from_url("https://careers.roblox.com/jobs/5221252").await.unwrap().spin().unwrap();
        assert!(record.header.unwrap().contains("head"));
        assert!(record.metadata.unwrap().contains("Roblox"));
        assert!(record.content.to_string().contains("Roblox"));
    }
}
