use orca::llm::openai::OpenAI;
use orca::pipeline::simple::LLMPipeline;
use orca::pipeline::Pipeline;
use orca::prompt::context::Context;
use serde::Serialize;

#[derive(Serialize)]
pub struct Data {
    country1: String,
    country2: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = OpenAI::new();
    let prompt = r#"
            {{#chat}}
            {{#user}}
            What is the capital of {{country1}}?
            {{/user}}
            {{#assistant}}
            Paris
            {{/assistant}}
            {{#user}}
            What is the capital of {{country2}}?
            {{/user}}
            {{/chat}}
            "#;
    let pipeline = LLMPipeline::new(&client).load_template("capitals", prompt)?.load_context(&Context::new(Data {
        country1: "France".to_string(),
        country2: "Germany".to_string(),
    })?)?;
    let res = pipeline.execute("capitals").await?.content();

    assert!(res.contains("Berlin") || res.contains("berlin"));
    Ok(())
}
