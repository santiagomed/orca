use super::error::RecordError;
use super::record::Record;

pub trait Spin {
    /// "Spin the record"
    /// This means that your record should be converted into a generic Record object
    /// that will enable LLM Chains to use it. Think of this as taking a record, or document, and extracting
    /// the text information relevant to the LLML Chain.
    fn spin(&self) -> Result<Record, RecordError>;
}
