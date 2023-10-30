use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Clone)]
pub struct Context<T> {
    variables: BTreeMap<String, T>,
}

impl<T> Default for Context<T> {
    /// Create a new context
    fn default() -> Self {
        Self {
            variables: BTreeMap::new(),
        }
    }
}

impl<T> Context<T> {
    /// Create a new context
    pub fn new() -> Context<T> {
        Context::default()
    }

    /// Set a variable in the context
    pub fn set(&mut self, name: &str, value: T) -> Option<T> {
        self.variables.insert(name.to_string(), value)
    }

    /// Get a variable from the context
    pub fn get(&self, name: &str) -> Option<&T> {
        self.variables.get(name)
    }

    /// Get the variables from the context and clean them up for serialization
    pub fn get_variables(&self) -> BTreeMap<String, String>
    where
        T: Serialize,
    {
        let mut serialized_variables = BTreeMap::new();
        for (key, value) in &self.variables {
            let s = serde_json::to_string(value).unwrap();
            let s = s.replace('\"', "");
            serialized_variables.insert(key.to_string(), s.to_string());
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
