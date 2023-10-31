#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use std::collections::HashMap;

use anyhow::Result;
use clap::Parser;
use orca::chains::chain::LLMChain;
use orca::chains::Chain;
use orca::llm::bert::Bert;
use orca::llm::openai::OpenAI;
use orca::llm::Embedding;
use orca::prompt;
use orca::qdrant::Qdrant;
use orca::qdrant::Value;
use orca::record::pdf::Pdf;
use orca::record::Spin;
use serde_json::json;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    /// The path to the PDF file to index
    file: String,

    #[clap(long)]
    /// The name of the collection to create
    /// (default: the name of the file)
    collection: Option<String>,

    #[clap(long)]
    /// The prompt to use to query the index
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // print pwd
    println!("pwd: {:?}", std::env::current_dir()?);

    let collection = if let Some(col) = args.collection {
        col
    } else {
        args.file.split("/").last().unwrap().split(".").next().unwrap().to_string()
    };

    let pdf_records = Pdf::from_file(&args.file, false).spin()?.split(1000);
    let bert = Bert::new().build_model_and_tokenizer().await?;

    let qdrant = Qdrant::new("localhost", 6334);
    if qdrant.create_collection(&collection, 384).await.is_ok() {
        let mut embeddings = Vec::new();
        for record in &pdf_records {
            let embedding = bert.generate_embedding(prompt!(record)).await?;
            embeddings.push(embedding.get_embedding()?);
        }
        qdrant.insert_many(&collection, embeddings, pdf_records).await?;
    }

    let query_embedding = bert.generate_embedding(prompt!(args.prompt)).await?;
    let result = qdrant.search(&collection, query_embedding.get_embedding()?, 5, None).await?;

    let context = json!({
        "user_prompt": args.prompt,
        "payloads": result
            .iter()
            .filter_map(|found_point| {
                found_point.payload.as_ref().map(|payload| {
                    // Assuming you want to convert the whole payload to a JSON string
                    serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
                })
            })
            .collect::<Vec<String>>()
    });

    println!("Context: {:#?}", context);

    let prompt_for_model = r#"
    {{#chat}}
        {{#system}}
        You are a highly advanced assistant. You receive a prompt from a user and relevant excerpts extracted from a PDF.
        You then answer truthfully to the best of your ability. If you do not know the answer, your response is "I don't know".
        {{/system}}
        
        {{#user}}
        '{{user_prompt}}'.
        {{/user}}
        
        {{#system}}
        Based on the retrieved information from the PDF, here are the relevant excerpts:
        
        {{#each payloads}}
        {{this}}
        {{/each}}
        
        Please provide a comprehensive answer to the user's question, integrating insights from these excerpts and your general knowledge.
        {{/system}}
    {{/chat}}
    "#;

    let openai = OpenAI::new();
    let mut pipe = LLMChain::new(&openai).with_template("query", prompt_for_model);
    pipe.load_context(&context).await;

    let response = pipe.execute("query").await?;

    println!("Response: {}", response.content());

    Ok(())
}
