<div align="center">
  <h1>Orca</h1>
  <img src="https://github.com/scrippt-tech/orca/assets/30184543/1dc482c2-48cd-4982-ab23-b2fed6c492d5" width="640"/>

  <p>
    <strong>Orca is a LLM Orchestration Framework written in Rust. It is designed to be a simple, easy-to-use, and easy-to-extend framework for creating LLM Orchestration. It is currently in development so it may contain bugs and its functionality is limited.</strong>
  </p>
  <p>

<!-- prettier-ignore-start -->

[![CI](https://github.com/scrippt-tech/orca/actions/workflows/ci.yml/badge.svg)](https://github.com/scrippt-tech/orca/actions/workflows/ci.yml)

<!-- prettier-ignore-end -->

  </p>
</div>

# About Orca
Orca is currently in development. It is hard to say what the future of Orca looks like, as I am currently learning about LLM orchestrations and its extensive applications. These are some ideas I want to explore. Suggestions are welcome!
 * [WebAssembly]("https://webassembly.org") to create simple, portable, yet powerful LLM applications that can run serverless across platforms.
 * Taking advantage of Rust for fast memory-safe distributed LLM applications.
 * Deploying LLMs to the edge (think IOT devices, mobile devices, etc.)

# Set up
To set up Orca, you will need to install Rust. You can do this by following the instructions [here](https://www.rust-lang.org/tools/install). Once you have Rust installed, you can add Orca to your Cargo.toml file as a dependency:
```toml
[dependencies]
orca = { git = "https://github.com/scrippt-tech/orca" }
```

# Features
* Prompt templating using handlebars-like syntax (see example below)
* Loading records (documents)
  * HTML from URLs or local files
  * PDF from bytes or local files
* Vector store support with [Qdrant]("https://qdrant.tech")
* Current LLM support:
  * [OpenAI Chat]("https://openai.com")
  * Limited [Bert]("https://huggingface.co/docs/transformers/model_doc/bert) support using the [Candle]("https://github.com/huggingface/candle") ML framework
* Chains:
  * Simple chains
  * Sequential chains

# Examples
Orca supports simple LLM chains and sequential chains. It also supports reading PDF and HTML records (documents).

## OpenAI Chat
```rust
use orca::chains::chain::LLMChain;
use orca::chains::Chain;
use orca::llm::openai::OpenAI;
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
    let mut chain = LLMChain::new(&client).with_template("capitals", prompt);
    chain
        .load_context(&Context::new(Data {
            country1: "France".to_string(),
            country2: "Germany".to_string(),
        })?)
        .await;
    let res = chain.execute("capitals").await?.content();

    assert!(res.contains("Berlin") || res.contains("berlin"));
    Ok(())
}
```

# Contributing
Contributors are welcome! If you would like to contribute, please open an issue or a pull request. If you would like to add a new feature, please open an issue first so we can discuss it. 

## Running locally
We use `[cargo-make](https://github.com/sagiegurari/cargo-make)` to run Orca locally. To install it run:
```bash
cargo install cargo-make
```
Once you have cargo-make installed, you can build or test Orca by running:
```bash
$ makers build # Build Orca
$ makers test # Test Orca
```
