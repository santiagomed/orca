use std::collections::HashMap;

use anyhow::Result;
use qdrant_client::prelude::*;
use qdrant_client::qdrant::point_id::PointIdOptions;
use qdrant_client::qdrant::value::Kind;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{CreateCollection, Filter, SearchPoints, VectorParams, VectorsConfig};
use serde::Serialize;

/// Trait to convert a type to a Qdrant payload.
pub trait ToPayload {
    fn to_payload(self) -> Result<Payload>;
}

pub struct StringPayload(String);

impl ToPayload for StringPayload {
    fn to_payload(self) -> Result<Payload> {
        let mut map = HashMap::new();
        map.insert("data".to_string(), Value::from(self.0));
        Ok(Payload::new_from_hashmap(map))
    }
}

impl<T> ToPayload for T
where
    T: Serialize,
{
    fn to_payload(self) -> Result<Payload> {
        let payload_map: HashMap<String, Value> = serde_json::from_value(serde_json::to_value(self)?)?;
        Ok(Payload::new_from_hashmap(payload_map))
    }
}

/// Represents a found point in the vector database.
pub struct FoundPoint {
    pub id: u64,
    pub score: f32,
    pub payload: Option<HashMap<String, Value>>, // assuming Value is from serde_json
}

/// Represents search conditions for the Qdrant wrapper.
pub enum Condition {
    Matches(String, Value), // Assuming Value is from serde_json or your own type
                            // Add more conditions as per qdrant's capabilities
}

/// Converts a `Value` to a `MatchValue` for use in a `Condition`.
fn convert_to_match_value(value: qdrant_client::prelude::Value) -> qdrant_client::qdrant::r#match::MatchValue {
    match value.kind {
        Some(Kind::BoolValue(b)) => b.into(),
        Some(Kind::IntegerValue(i)) => i.into(),
        Some(Kind::StringValue(s)) => s.into(),
        Some(Kind::DoubleValue(d)) => {
            // You might decide to handle this differently since MatchValue doesn't seem to support f64 directly.
            panic!("Unsupported double value: {}", d)
        }
        Some(Kind::StructValue(_)) => {
            // This represents a complex structure and might need specialized handling.
            panic!("Unsupported structured value")
        }
        Some(Kind::ListValue(_)) => {
            // This represents a list and might need specialized handling.
            panic!("Unsupported list value")
        }
        Some(Kind::NullValue(_)) | None => {
            panic!("Null or unsupported value type")
        }
    }
}

impl Condition {
    /// Converts a `Condition` to a `qdrant_client::qdrant::Condition`.
    fn to_qdrant_condition(&self) -> qdrant_client::qdrant::Condition {
        match self {
            Condition::Matches(key, value) => {
                let match_value = convert_to_match_value(value.clone());
                qdrant_client::qdrant::Condition::matches(key, match_value)
            } // Handle other conditions similarly
        }
    }
}

pub struct Qdrant {
    client: QdrantClient,
}

impl Qdrant {
    /// Creates a new `Qdrant` instance with the given `host` and `port`.
    ///
    /// # Arguments
    /// * `host` - A string slice that holds the IP address or hostname of the Qdrant server.
    /// * `port` - An unsigned 16-bit integer that holds the port number of the Qdrant server.
    ///
    /// # Example
    /// ```
    /// use orca::qdrant::Qdrant;
    ///
    /// let qdrant = Qdrant::new("127.0.0.1", 6333);
    /// ```
    pub fn new(host: &str, port: u16) -> Self {
        let config = QdrantClientConfig::from_url(&format!("http://{}:{}", host, port));
        let client = QdrantClient::new(Some(config)).unwrap();
        Qdrant { client }
    }

    /// Creates a new collection with the given name and vector size.
    ///
    /// # Arguments
    /// * `collection_name` - A string slice that holds the name of the collection to be created.
    /// * `vector_size` - An unsigned 64-bit integer that represents the size of the vectors in the collection.
    ///
    /// # Example
    /// ```no_run
    /// # use orca::qdrant::Qdrant;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Qdrant::new("127.0.0.1", 6333);
    /// let collection_name = "test_collection";
    /// let vector_size = 128;
    /// client.create_collection(collection_name, vector_size).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_collection(&self, collection_name: &str, vector_size: u64) -> Result<()> {
        let config = Some(Config::Params(VectorParams {
            size: vector_size,
            distance: Distance::Cosine.into(),
            ..Default::default()
        }));
        let vectors_config = VectorsConfig { config };
        let create_collection = CreateCollection {
            collection_name: collection_name.to_string(),
            vectors_config: Some(vectors_config),
            ..Default::default()
        };
        self.client.create_collection(&create_collection).await?;
        Ok(())
    }

    /// Deletes a collection with the given name.
    ///
    /// # Arguments
    ///
    /// * `collection_name` - A string slice that holds the name of the collection to be deleted.
    ///
    /// # Example
    /// ```no_run
    /// # use orca::qdrant::Qdrant;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Qdrant::new("localhost", 6333);
    /// let collection_name = "test_collection";
    /// client.delete_collection(collection_name).await?;
    /// # Ok(())
    /// # }
    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        self.client.delete_collection(collection_name).await?;
        Ok(())
    }

    /// Inserts a new point into the specified collection with the given vector and payload.
    ///
    /// # Arguments
    /// * `collection_name` - A string slice that holds the name of the collection.
    /// * `vector` - A vector of 32-bit floating point numbers that represents the point's vector.
    /// * `payload` - A generic type that holds the payload to be associated with the point.
    ///
    /// # Examples
    /// ```no_run
    /// # use orca::qdrant::Qdrant;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyPayload {
    /// #     name: String,
    /// #     age: u8,
    /// # }
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let qdrant = Qdrant::new("localhost", 6333);
    /// let collection_name = "my_collection";
    /// let vector = vec![0.1, 0.2, 0.3];
    /// let payload = MyPayload { name: "John".to_string(), age: 30 };
    /// qdrant.insert(collection_name, vector, payload).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert<T>(&self, collection_name: &str, vector: Vec<f32>, payload: T) -> Result<()>
    where
        T: ToPayload,
    {
        let payload: Payload = payload.to_payload()?;
        let points = vec![PointStruct::new(0, vector, payload)];
        self.client.upsert_points_blocking(collection_name, points, None).await?;
        Ok(())
    }

    /// Searches for points in a given collection that match the specified conditions.
    ///
    /// # Arguments
    /// * `collection_name` - The name of the collection to search in.
    /// * `vector` - The vector to search for.
    /// * `limit` - The maximum number of results to return.
    /// * `conditions` - Optional conditions to filter the search results.
    ///
    /// # Returns
    /// A `Result` containing a `Vec` of `FoundPoint`s that match the search criteria.
    ///
    /// # Example
    /// ```no_run
    /// # use orca::qdrant::{Qdrant, Condition};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Qdrant::new("localhost", 6333);
    /// let conditions = vec![Condition::Matches(
    ///    "age".into(),
    ///     30.into(),
    /// )];
    /// let results = client
    ///     .search("my_collection", vec![1.0, 2.0, 3.0], 10, Some(conditions))
    ///     .await?;
    /// for result in results {
    ///     println!("Found point with ID {} and score {}", result.id, result.score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        limit: usize,
        conditions: Option<Vec<Condition>>,
    ) -> Result<Vec<FoundPoint>> {
        let filter = conditions.map(|cond| Filter::all(cond.into_iter().map(|c| c.to_qdrant_condition())));
        let search_request = SearchPoints {
            collection_name: collection_name.into(),
            vector,
            filter,
            limit: limit as u64,
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let response = self.client.search_points(&search_request).await?;

        let results: Vec<FoundPoint> = response
            .result
            .into_iter()
            .filter_map(|scored_point| {
                let id = match scored_point.id {
                    Some(point_id) => {
                        match point_id.point_id_options {
                            Some(PointIdOptions::Num(id)) => id,
                            _ => return None, // Ignore other variants or if it's None
                        }
                    }
                    None => return None, // Skip this point if it doesn't have an ID
                };
                let score = scored_point.score;
                let payload = scored_point.payload;
                Some(FoundPoint {
                    id,
                    score,
                    payload: Some(payload),
                })
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use serde_json::json;

    const TEST_HOST: &str = "localhost";
    const TEST_PORT: u16 = 6334;

    fn generate_unique_collection_name() -> String {
        let rng = rand::thread_rng();
        let suffix: String = rng.sample_iter(&Alphanumeric).take(8).map(char::from).collect();
        format!("test_collection_{}", suffix)
    }

    async fn teardown(collection_name: &str) {
        let qdrant = Qdrant::new(TEST_HOST, TEST_PORT);
        let _ = qdrant.delete_collection(collection_name).await;
    }

    #[tokio::test]
    async fn test_create_collection() {
        let qdrant = Qdrant::new(TEST_HOST, TEST_PORT);
        let unique_collection_name = generate_unique_collection_name();

        let result = qdrant.create_collection(&unique_collection_name, 128).await;
        assert!(result.is_ok());

        teardown(&unique_collection_name).await;
    }

    #[tokio::test]
    async fn test_insert_point() {
        let qdrant = Qdrant::new(TEST_HOST, TEST_PORT);
        let unique_collection_name = generate_unique_collection_name();

        qdrant.create_collection(&unique_collection_name, 3).await.unwrap();

        let vector = vec![0.1, 0.2, 0.3];
        let payload = "some_payload".to_string();

        let result = qdrant.insert(&unique_collection_name, vector, StringPayload(payload)).await;
        assert!(result.is_ok());

        teardown(&unique_collection_name).await;
    }

    #[tokio::test]
    async fn test_search_points() {
        let qdrant = Qdrant::new(TEST_HOST, TEST_PORT);
        let unique_collection_name = generate_unique_collection_name();

        qdrant.create_collection(&unique_collection_name, 3).await.unwrap();
        let vector = vec![0.1, 0.2, 0.3];
        let payload = json!(
            {
                "name": "John",
                "age": 30
            }
        );
        qdrant.insert(&unique_collection_name, vector.clone(), payload).await.unwrap();

        let conditions = vec![Condition::Matches("name".to_string(), "John".into())];

        let results = qdrant.search(&unique_collection_name, vector, 10, Some(conditions)).await;
        assert!(results.is_ok());

        let points = results.unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].payload.as_ref().unwrap()["name"], "John".into());

        teardown(&unique_collection_name).await;
    }

    #[test]
    #[should_panic(expected = "Unsupported double value")]
    fn test_unsupported_match_value() {
        let _ = convert_to_match_value(Value {
            kind: Some(Kind::DoubleValue(1.23)),
        });
    }
}
