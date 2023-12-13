use anyhow::{Context, Result};
use clap::Parser;
use orca::{
    llm::{bert::Bert, quantized::Quantized, Embedding},
    memory::Buffer,
    pipeline::simple::LLMPipeline,
    pipeline::Pipeline,
    prompt,
    prompt::context::Context as OrcaContext,
    prompts,
    qdrant::Qdrant,
    record::{pdf::Pdf, Spin},
};
use rand::Rng;
use serde_json::json;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    /// The path to the PDF file to index
    file: String,

    #[clap(long)]
    /// The prompt to use to query the index
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // init logger
    env_logger::init();

    let pdf_records = Pdf::from_file(&args.file, false)
        .context("Failed to read PDF file")?
        .spin()
        .context("Failed to process PDF spin")?
        .split(399);

    let bert = Bert::new().build_model_and_tokenizer().await?;

    let collection = std::path::Path::new(&args.file)
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract file stem"))?
        .to_string();

    // Initialize Qdrant
    let qdrant = Qdrant::new("http://localhost:6334")?;
    qdrant.create_collection(&collection, 384).await?;

    // Generate embeddings and insert into Qdrant
    let embeddings = bert.generate_embeddings(prompts!(&pdf_records)).await?;
    qdrant.insert_many(&collection, embeddings.to_vec2()?, pdf_records).await?;

    // Use prompt to query Qdrant
    let query_embedding = bert.generate_embedding(prompt!(args.prompt)).await?;
    let result = qdrant.search(&collection, query_embedding.to_vec()?.clone(), 1, None).await?;

    let prompt_for_model = r#"
    {{#chat}}

        {{#system}}
        You are a highly advanced assistant. You receive a prompt from a user and relevant excerpts extracted from a PDF. You then answer truthfully to the best of your ability. If you do not know the answer, your response is I don't know.
        {{/system}}

        {{#user}}
        {{user_prompt}}
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

    let context = json!({
        "user_prompt": args.prompt,
        "payloads": result
            .iter()
            .filter_map(|found_point| {
                found_point.payload.as_ref().map(|payload| {
                    serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
                })
            })
            .collect::<Vec<String>>()
    });

    let mistral = Quantized::new()
        .with_model(orca::llm::quantized::Model::Mistral7bInstruct)
        .with_sample_len(4000)
        .with_seed(rand::thread_rng().gen_range(0..100))
        .load_model_from_path("../../weights/mistral-7b-instruct-v0.1.Q4_K_M.gguf")?
        .build_model()?;

    let pipe = LLMPipeline::new(&mistral)
        .load_template("query", prompt_for_model)?
        .load_context(&OrcaContext::new(context)?)?
        .load_memory(Buffer::new());

    let res = pipe.execute("query").await?;

    println!("\nResponse: {}", res.content());

    let stdin = std::io::stdin();
    let mut input = String::new();

    loop {
        println!("Enter your prompt (type 'exit' to quit): ");
        input.clear();
        stdin.read_line(&mut input)?;
        let trimmed_input = input.trim();

        // Exit condition
        if trimmed_input.eq_ignore_ascii_case("exit") {
            break;
        }

        let pipe = pipe.clone().load_template("query", trimmed_input)?;

        let res = pipe.execute("query").await?;

        println!("\nResponse: {}", res.content());
    }

    Ok(())
}
