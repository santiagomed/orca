use pdf::PdfError;

#[derive(Debug)]
pub enum RecordError {
    /// Reqwest crate error
    ReqwestError(reqwest::Error),

    /// IO error
    IOError(std::io::Error),

    /// PDF crate error
    PDFError(PdfError),
}

impl From<reqwest::Error> for RecordError {
    /// Convert a reqwest error into a record error
    fn from(err: reqwest::Error) -> RecordError {
        RecordError::ReqwestError(err)
    }
}

impl From<std::io::Error> for RecordError {
    /// Convert an IO error into a record error
    fn from(err: std::io::Error) -> RecordError {
        RecordError::IOError(err)
    }
}

impl From<PdfError> for RecordError {
    /// Convert a PDF error into a record error
    fn from(err: PdfError) -> RecordError {
        RecordError::PDFError(err)
    }
}
