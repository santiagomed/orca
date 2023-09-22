pub mod error;
pub mod html;
pub mod pdf;
use serde::Serialize;
use error::RecordError;

/// Content of a record which can be represented as either a string or a vector of strings.
/// To get the string representation of the content, use the `to_string` method.
#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
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
}

pub trait Spin {
    /// "Spin the record"
    /// This means that your record should be converted into a generic Record object
    /// that will enable LLM Chains to use it. Think of this as taking a record, or document, and extracting
    /// the text information relevant to the LLML Chain.
    fn spin(&self) -> Result<Record, RecordError>;
}