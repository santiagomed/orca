#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use anyhow::Result;
use orca::llm::bert::Bert;
use orca::llm::Embedding;
use orca::qdrant::Qdrant;
use orca::record::pdf::{self, Pdf};
use orca::record::Spin;

#[tokio::main]
async fn main() -> Result<()> {
    let pdf_record = Pdf::from_file("../memgpt.pdf", false).spin()?.split(1000);
    let qdrant = Qdrant::new("localhost", 6333);
    qdrant.create_collection("memgpt", 384).await?;
    let bert = Bert::new();

    Ok(())
}
