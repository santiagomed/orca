use orca::chains::chain::LLMChain;
use orca::chains::Chain;
use orca::llm::quantized::{Model, Quantized};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let model = Quantized::new()
        .with_model(Model::Mistral7bInstruct)
        .with_sample_len(99)
        .load_model_from_path("../../models/mistral-7b-instruct-v0.1.Q4_K_S.gguf")?
        .build_model()?;

    let pipe =
        LLMChain::new(&model).with_template("greet", "{{#chat}}{{#user}}Hi how are you doing?{{/user}}{{/chat}}");
    let result = pipe.execute("greet").await?;

    println!("{}", result.content());

    Ok(())
}
