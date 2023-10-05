pub mod html;
pub mod pdf;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use text_splitter::TextSplitter;
/// Content of a record which can be represented as either a string or a vector of strings.
/// To get the string representation of the content, use the `to_string` method.
#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(untagged)]
pub enum Content {
    String(String),
    Vec(Vec<String>),
}

impl ToString for Content {
    /// Get the string representation of the content.
    fn to_string(&self) -> String {
        match self {
            Content::String(string) => string.to_string(),
            Content::Vec(vec) => vec.join("\n******************\n"),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Record {
    /// Header information for the record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// Content of the record.
    pub content: Content,

    /// Metadata for the record (present in PDFs, for example).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

impl Record {
    /// Create a new record with the given content.
    pub fn new(content: Content) -> Record {
        Record {
            header: None,
            content,
            metadata: None,
        }
    }

    /// Modify the header of the record.
    pub fn with_header(mut self, header: String) -> Self {
        self.header = Some(header);
        self
    }

    /// Modify the metadata of the record.
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Splits the content of a `Record` into multiple smaller records based on character count.
    ///
    /// This function divides the content of a `Record` into smaller chunks of approximately equal size.
    /// The chunks are determined by the maximum number of characters allowed per chunk. If the content
    /// is a vector of strings, each string will be split into chunks separately.
    ///
    /// # Arguments
    /// * `chunks` - The desired number of chunks the content should be split into.
    ///
    /// # Returns
    /// A vector of `Record` where each record contains a chunk of the original content.
    ///
    /// # Example
    /// ```
    /// # use orca::record::Record;
    /// # use orca::record::Content;
    /// let record = Record::new(Content::String("Hello World".into()));
    /// let records = record.split(2);
    /// assert_eq!(records.len(), 2);
    /// ```
    pub fn split(&self, chunks: usize) -> Vec<Record> {
        let max_chars = self.content.to_string().len() / chunks;
        let mut records = Vec::new();
        let splitter = TextSplitter::default().with_trim_chunks(true);
        match &self.content {
            Content::String(string) => {
                let chunks = splitter.chunks(string, max_chars);
                for chunk in chunks {
                    records.push(Record::new(Content::String(chunk.to_string())));
                }
            }
            Content::Vec(vec) => {
                for string in vec {
                    let chunks = splitter.chunks(string, max_chars);
                    for chunk in chunks {
                        records.push(Record::new(Content::String(chunk.to_string())));
                    }
                }
            }
        }
        records
    }

    /// Splits the content of a `Record` into multiple smaller records using a tokenizer.
    ///
    /// This function divides the content of a `Record` into smaller chunks using a specified tokenizer.
    /// The chunks are determined by the tokenizer and the number of chunks parameter. If the content is a
    /// vector of strings, each string will be split into chunks separately.
    ///
    /// # Arguments
    /// * `chunks` - The desired number of chunks the content should be split into.
    /// * `tokenizer` - The tokenizer to be used for splitting the content. This can be a Huggingface
    /// tokenizer, a tokenizer from a file, or a tokenizer from bytes.
    ///
    /// # Returns
    /// A `Result` containing a vector of `Record` where each record contains a chunk of the original
    /// content, or an error if there was a problem with tokenization.
    ///
    /// # Example
    /// ```no_run
    /// # use orca::record::Record;
    /// # use orca::record::Content;
    /// # use orca::record::Tokenizer;
    /// # use std::path::Path;
    /// let record = Record::new(Content::String("Hello World".into()));
    /// let records = record.split_with_tokenizer(2, Tokenizer::Huggingface("path_to_tokenizer".into())).unwrap();
    /// assert_eq!(records.len(), 2);
    /// ```
    pub fn split_with_tokenizer(&self, chunks: usize, tokenizer: Tokenizer) -> Result<Vec<Record>> {
        let tokenizer = match tokenizer {
            Tokenizer::Huggingface(tokenizer) => {
                tokenizers::Tokenizer::from_pretrained(tokenizer, None).map_err(anyhow::Error::msg)?
            }
            Tokenizer::File(path) => tokenizers::Tokenizer::from_file(path).map_err(anyhow::Error::msg)?,
            Tokenizer::Bytes(bytes) => tokenizers::Tokenizer::from_bytes(bytes).map_err(anyhow::Error::msg)?,
        };

        let splitter = TextSplitter::new(tokenizer).with_trim_chunks(true);

        let mut records = Vec::new();

        match &self.content {
            Content::String(string) => {
                let chunks = splitter.chunks(string, chunks);
                for chunk in chunks {
                    records.push(Record::new(Content::String(chunk.to_string())));
                }
            }
            Content::Vec(vec) => {
                for string in vec {
                    let chunks = splitter.chunks(string, chunks);
                    for chunk in chunks {
                        records.push(Record::new(Content::String(chunk.to_string())));
                    }
                }
            }
        }

        Ok(records)
    }
}

pub enum Tokenizer<'t> {
    // Huggingface tokenizer
    Huggingface(String),

    // File tokenizer
    File(&'t Path),

    // Bytes tokenizer
    Bytes(&'t [u8]),
}

pub trait Spin {
    /// "Spin the record"
    /// This means that your record should be converted into a generic Record object
    /// that will enable LLM Chains to use it. Think of this as taking a record, or document, and extracting
    /// the text information relevant to the LLML Chain.
    fn spin(&self) -> Result<Record>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_to_string() {
        let content = Content::String("Hello".to_string());
        assert_eq!(content.to_string(), "Hello");

        let content = Content::Vec(vec!["Hello".to_string(), "World".to_string()]);
        assert_eq!(content.to_string(), "Hello\n******************\nWorld");
    }

    #[test]
    fn test_record_new() {
        let content = Content::String("Hello".to_string());
        let record = Record::new(content.clone());
        assert_eq!(record.content, content);
        assert!(record.header.is_none());
        assert!(record.metadata.is_none());
    }

    #[test]
    fn test_with_header_and_metadata() {
        let content = Content::String("Hello".to_string());
        let record =
            Record::new(content.clone()).with_header("Header".to_string()).with_metadata("Metadata".to_string());

        assert_eq!(record.content, content);
        assert_eq!(record.header, Some("Header".to_string()));
        assert_eq!(record.metadata, Some("Metadata".to_string()));
    }

    #[test]
    fn test_split_content() {
        let content = Content::String("Hello World!".to_string());
        let record = Record::new(content);
        let chunks = record.split(2);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].content.to_string(), "Hello");
        assert_eq!(chunks[1].content.to_string(), "World!");
    }

    // This test requires a valid tokenizer and a suitable setup, so it's more of a template
    #[test]
    #[ignore = "This test requires a valid tokenizer and a suitable setup, so it's more of a template"]
    fn test_split_with_tokenizer() {
        // Use an appropriate tokenizer setup for your case
        let tokenizer = Tokenizer::Huggingface("path_to_tokenizer".into());
        let content = Content::String("Hello World!".to_string());
        let record = Record::new(content);
        let chunks = record.split_with_tokenizer(2, tokenizer).unwrap();

        assert_eq!(chunks.len(), 2);
        // Further assertions depending on the expected behavior of your tokenizer
    }

    // Add more tests as necessary
}
