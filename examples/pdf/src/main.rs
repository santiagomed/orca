use anyhow::Result;
use orca::llm::bert::Bert;
use orca::llm::Embedding;
use orca::qdrant::Qdrant;
use orca::record::pdf::Pdf;
use orca::record::Spin;

fn main() -> Result<()> {
    let pdf_record = Pdf::from_file("../memgpt.pdf", false).spin()?;
    let qdrant = Qdrant::new("localhost", 6333);
    qdrant.create_collection("memgpt", 384);
    let bert = Bert::new();

    Ok(())
}
