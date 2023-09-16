use super::error::RecordError;
use super::record::Record;
use super::spin::Spin;
use reqwest;
use scraper;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct HTML {
    body: String,
}

impl HTML {
    pub async fn from_url(url: &str) -> Result<HTML, RecordError> {
        let body = reqwest::get(url).await?.text().await?;
        Ok(HTML { body })
    }

    pub fn from_file(path: &str) -> Result<HTML, RecordError> {
        let body = fs::read_to_string(Path::new(path)).unwrap();
        Ok(HTML { body })
    }
}

impl Spin for HTML {
    type Output = String;

    fn spin(&self) -> Result<Record<String>, RecordError> {
        let html = scraper::Html::parse_document(&self.body);

        // select head
        let header = html.select(&scraper::Selector::parse("head").unwrap()).next().map(|head| head.inner_html()).unwrap_or_default();

        // select main
        let content = html
            .select(&scraper::Selector::parse("main").unwrap())
            .next()
            .map(|main| main.inner_html())
            .unwrap_or_else(|| html.root_element().inner_html());

        // select metadata
        let metadata = html.select(&scraper::Selector::parse("meta").unwrap()).next().map(|meta| meta.inner_html()).unwrap_or_default();

        Ok(Record::new(content).with_header(header).with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_from_url() {
        let record = HTML::from_url("https://google.com").await.unwrap().spin().unwrap();
        assert!(record.header.unwrap().contains("head"));
        assert_eq!(record.metadata.unwrap(), "");
        assert!(record.content.contains("Google"));
    }
}
