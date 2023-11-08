use anyhow::Result;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct Context(HashMap<String, JsonValue>);

impl Context {
    /// Create a new context from a serializable object
    pub fn new<C>(context: C) -> Result<Context>
    where
        C: Serialize + Sync,
    {
        let json_str = serde_json::to_string(&context)?;
        let hashmap: HashMap<String, JsonValue> = serde_json::from_str(&json_str)?;
        Ok(Context(hashmap))
    }

    /// Create a new context from a JSON string
    pub fn from_string(context: &str) -> Result<Context> {
        let hashmap: HashMap<String, JsonValue> = serde_json::from_str(context)?;
        Ok(Context(hashmap))
    }

    /// Get a reference to a value by key
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0.get(key)
    }

    /// Set a value for a key, where value is any serializable type
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)?;
        self.0.insert(key.to_string(), json_value);
        Ok(())
    }

    /// Get a reference to the underlying hashmap
    pub fn as_object(&self) -> &HashMap<String, JsonValue> {
        &self.0
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
        let mut context = Context::new(Test {
            name: "gpt".to_string(),
            age: 1,
        })
        .unwrap();

        context
            .set(
                "name",
                Test {
                    name: "gpt".to_string(),
                    age: 1,
                },
            )
            .unwrap();

        assert_eq!(
            context.get("name").unwrap(),
            &serde_json::to_value(&Test {
                name: "gpt".to_string(),
                age: 1,
            })
            .unwrap()
        );
    }
}
