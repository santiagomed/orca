use std::{fmt::Display, sync::Arc, vec};

use super::{Content, Record, Spin};
use anyhow::Result;
use pdf::{
    any::AnySync,
    file::{File, FileOptions, NoLog, SyncCache},
    object::PlainRef,
    PdfError,
};

type PDFFile = File<
    Vec<u8>,
    Arc<SyncCache<PlainRef, Result<AnySync, Arc<PdfError>>>>,
    Arc<SyncCache<PlainRef, Result<Arc<[u8]>, Arc<PdfError>>>>,
    NoLog,
>;

pub struct PDF {
    file: PDFFile,
    split: bool,
}

impl PDF {
    /// Create a new PDF record from a buffer
    /// When calling this function, specify the PDF generic type as a slice of bytes
    /// ```
    /// use orca::record::pdf::PDF;
    /// use base64::{engine::general_purpose, Engine};
    /// use std::io::Read;
    ///
    /// let mut f = std::fs::File::open("./tests/pdf.in").unwrap();
    /// let mut c = String::new();
    /// f.read_to_string(&mut c).unwrap();
    /// let mut bytes: Vec<u8> = Vec::new();
    /// general_purpose::STANDARD.decode_vec(c, &mut bytes).unwrap();
    ///
    /// let record = PDF::from_buffer(bytes, false);
    /// ```
    pub fn from_buffer(buffer: Vec<u8>, split: bool) -> PDF {
        // convert buffer into file object
        PDF {
            file: FileOptions::cached().load(buffer).unwrap(),
            split,
        }
    }

    /// Create a new PDF record from a file
    /// When calling this function, specify the PDF generic type as a vector of bytes
    /// ```
    /// use orca::record::pdf::PDF;
    ///
    /// let record = PDF::from_file("./tests/sample-resume.pdf", false);
    /// ```
    pub fn from_file(path: &str, split: bool) -> PDF {
        // convert buffer into file object
        PDF {
            file: FileOptions::cached().open(path).unwrap(),
            split,
        }
    }
}

pub enum PDFOutput {
    String(String),
    Vec(Vec<String>),
}

impl PDFOutput {
    /// Get the vector representation of the PDF output
    pub fn to_vec(self) -> Vec<String> {
        match self {
            PDFOutput::String(string) => vec![string],
            PDFOutput::Vec(vec) => vec,
        }
    }
}

impl Display for PDFOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PDFOutput::String(string) => write!(f, "{}", string),
            PDFOutput::Vec(vec) => write!(f, "{}", vec.join("\n******************\n")),
        }
    }
}

impl Spin for PDF {
    fn spin(&self) -> Result<Record> {
        let resolver = self.file.resolver();
        return if self.split {
            let mut content = Vec::new();
            for page in self.file.pages() {
                let page = page?;
                let mut page_content = String::new();
                let flow = pdf_text::run(&self.file, &page, &resolver)?;
                for run in flow.runs {
                    for line in run.lines {
                        for word in line.words {
                            page_content.push_str(&word.text);
                            page_content.push(' ');
                        }
                        page_content.push('\n');
                    }
                }
                content.push(page_content);
            }
            Ok(Record::new(Content::Vec(content)))
        } else {
            let resolver = self.file.resolver();
            let mut content = String::new();
            for page in self.file.pages() {
                let page = page?;
                let flow = pdf_text::run(&self.file, &page, &resolver)?;
                for run in flow.runs {
                    for line in run.lines {
                        for word in line.words {
                            content.push_str(&word.text);
                            content.push(' ');
                        }
                        content.push('\n');
                    }
                }
            }
            Ok(Record::new(Content::String(content)))
        };
    }
}

#[cfg(test)]
mod test {

    use std::io::Read;

    use super::*;
    use base64::{engine::general_purpose, Engine};

    #[test]
    fn test_from_buffer() {
        let mut f = std::fs::File::open("./tests/pdf.in").unwrap();
        let mut c = String::new();
        f.read_to_string(&mut c).unwrap();
        let mut bytes: Vec<u8> = Vec::new();
        general_purpose::STANDARD.decode_vec(c, &mut bytes).unwrap();

        let record = PDF::from_buffer(bytes, false);
        assert!(!record.split);
    }

    #[test]
    fn test_from_file() {
        let record = PDF::from_file("./tests/sample-resume.pdf", false);
        assert!(!record.split);
    }

    #[test]
    fn test_spin_from_buffer() {
        std::env::set_var("STANDARD_FONTS", "./assets/pdf_fonts");
        let mut f = std::fs::File::open("./tests/pdf.in").unwrap();
        let mut c = String::new();
        f.read_to_string(&mut c).unwrap();
        let mut bytes: Vec<u8> = Vec::new();
        general_purpose::STANDARD.decode_vec(c, &mut bytes).unwrap();

        let record = PDF::from_buffer(bytes, false).spin().unwrap();
        let correct_content = include_str!("../../tests/out/sample-resume.out");
        assert_eq!(record.content.to_string(), correct_content);
    }

    #[test]
    fn test_spin_from_file() {
        std::env::set_var("STANDARD_FONTS", "./assets/pdf_fonts");
        let record = PDF::from_file("./tests/sample-resume.pdf", false).spin().unwrap();
        let correct_content = include_str!("../../tests/out/sample-resume.out");
        assert_eq!(record.content.to_string(), correct_content);
    }
}
