use orca::chains::chain::LLMChain;
use orca::chains::Chain;
use orca::llm::openai::OpenAI;
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
    let mut chain = LLMChain::new(&client).with_template("capitals", prompt);
    chain
        .load_context(&Data {
            country1: "France".to_string(),
            country2: "Germany".to_string(),
        })
        .await;
    let res = chain.execute("capitals").await?.content();

    assert!(res.contains("Berlin") || res.contains("berlin"));
    Ok(())
}
