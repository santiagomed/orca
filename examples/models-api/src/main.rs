use models::mistral::{Config, Mistral};

fn main() {
    let prompt = "The eiffel tower is";
    let weights = std::path::Path::new("../../weights/mistral_model-q4k.gguf");
    let tokenizer = std::path::Path::new("../../weights/mistral_tokenizer.json");
    let config = Config::default();
    let mistral = Mistral::from_path(weights, tokenizer, config).unwrap();
    let mut output = std::io::stdout();
    mistral.generate(prompt, 100, &mut output).unwrap();
}
