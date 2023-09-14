use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Record {
    /// Header information for the record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// Content of the record.
    pub content: String,

    /// Metadata for the record (present in PDFs, for example).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug)]
pub enum RecordError {
    ReqwestError(reqwest::Error),
    IOError(std::io::Error),
}

impl Record {
    pub fn new(content: String) -> Record {
        Record {
            header: None,
            content,
            metadata: None,
        }
    }

    pub fn with_header(mut self, header: String) -> Self {
        self.header = Some(header);
        self
    }

    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl From<reqwest::Error> for RecordError {
    fn from(err: reqwest::Error) -> RecordError {
        RecordError::ReqwestError(err)
    }
}

impl From<std::io::Error> for RecordError {
    fn from(err: std::io::Error) -> RecordError {
        RecordError::IOError(err)
    }
}
