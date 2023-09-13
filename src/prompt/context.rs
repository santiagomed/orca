use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Clone)]
pub struct Context<T> {
    variables: BTreeMap<String, T>,
}

impl<T> Context<T> {
    pub fn new() -> Context<T> {
        Context {
            variables: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: T) -> Option<T> {
        self.variables.insert(name.to_string(), value)
    }

    pub fn get(&self, name: &str) -> Option<&T> {
        self.variables.get(name)
    }

    pub fn get_variables(&self) -> BTreeMap<String, String>
    where
        T: Serialize + ToString,
    {
        let mut serialized_variables = BTreeMap::new();
        for (key, value) in &self.variables {
            serialized_variables.insert(key.to_string(), value.to_string());
        }
        serialized_variables
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Serialize, PartialEq, Debug)]
    struct Test {
        name: String,
        age: u8,
    }

    #[test]
    fn test_set_get() {
        let mut context = Context::new();

        context.set(
            "name",
            Test {
                name: "gpt".to_string(),
                age: 1,
            },
        );

        assert_eq!(
            context.get("name"),
            Some(&Test {
                name: "gpt".to_string(),
                age: 1,
            })
        );
    }
}
