use pdf::PdfError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecordError {
    /// Reqwest crate error
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    /// IO error
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// PDF crate error
    #[error(transparent)]
    PDFError(#[from] PdfError),
}
