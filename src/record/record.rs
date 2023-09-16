use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Record<C> {
    /// Header information for the record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    /// Content of the record.
    pub content: C,

    /// Metadata for the record (present in PDFs, for example).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

impl<C> Record<C> {
    pub fn new(content: C) -> Record<C> {
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
