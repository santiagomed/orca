use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct Context {
    variables: BTreeMap<String, String>,
}

impl Context {
    pub fn new() -> Context {
        Context {
            variables: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    pub fn variables(&self) -> &BTreeMap<String, String> {
        &self.variables
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_set_get() {
        let mut context = Context::new();
        context.set("name", "gpt");
        assert_eq!(context.get("name"), Some(&"gpt".to_string()));
    }
}
