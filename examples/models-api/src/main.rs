use models::quantized::{Config, Quantized};

fn main() {
    let prompt = "The eiffel tower is";
    let weights = std::path::Path::new("../../weights/llama-2-7b-chat.ggmlv3.q2_K.bin");
    let tokenizer = std::path::Path::new("../../weights/llama_tokenizer.json");
    let weights = std::fs::read(weights).unwrap();
    let tokenizer = std::fs::read(tokenizer).unwrap();
    let config = Config::default();
    let mistral = Quantized::from_ggml_stream(weights, tokenizer, config).unwrap();
    let mut output = std::io::stdout();
    mistral.generate(prompt, 100, &mut output).unwrap();
}
