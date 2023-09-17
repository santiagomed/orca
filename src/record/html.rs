use super::error::RecordError;
use super::record::{Content, Record};
use super::spin::Spin;
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
    pub async fn from_url(url: &str, selectors: &str) -> Result<HTML, RecordError> {
        // check for timeout
        let client = reqwest::ClientBuilder::new().timeout(std::time::Duration::from_secs(5)).build()?;
        let body = client.get(url).send().await?.text().await?;

        Ok(HTML {
            body,
            selectors: selectors.to_string(),
        })
    }

    pub fn from_file(path: &str, selectors: &str) -> Result<HTML, RecordError> {
        let body = fs::read_to_string(Path::new(path)).unwrap();
        Ok(HTML {
            body,
            selectors: selectors.to_string(),
        })
    }
}

impl Spin for HTML {
    fn spin(&self) -> Result<Record, RecordError> {
        let html = scraper::Html::parse_document(&self.body);

        let mut header = String::new();
        html.select(&Selector::parse("head").unwrap()).for_each(|element| {
            header.push_str(element.inner_html().as_str());
        });

        let mut metadata = String::new();
        html.select(&Selector::parse("meta").unwrap()).for_each(|element| {
            if element.value().attr("name").is_some() {
                metadata.push_str(format!("{}: ", element.value().attr("name").unwrap()).as_str());
                metadata.push_str(element.value().attr("content").unwrap());
                metadata.push('\n');
            }
        });

        // select content
        let content = html
            .select(&Selector::parse(self.selectors.as_str()).unwrap())
            .map(|element| element.inner_html())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Record::new(Content::String(content)).with_header(header).with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_from_url() {
        let record = HTML::from_url("https://careers.roblox.com/jobs/5221252", "p, li").await.unwrap().spin().unwrap();
        assert!(record.header.unwrap().contains("head"));
        assert!(record.metadata.unwrap().contains("Roblox"));
        assert!(record.content.to_string().contains("Roblox"));
    }
}
