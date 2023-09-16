use pdf::PdfError;

#[derive(Debug)]
pub enum RecordError {
    ReqwestError(reqwest::Error),
    IOError(std::io::Error),
    PDFError(PdfError),
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

impl From<PdfError> for RecordError {
    fn from(err: PdfError) -> RecordError {
        RecordError::PDFError(err)
    }
}
