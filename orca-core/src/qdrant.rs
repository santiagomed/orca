use std::collections::HashMap;

use anyhow::{Context, Result};
pub use qdrant_client::qdrant::Value as QdrantValue;
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

impl<T> ToPayload for T
where
    T: Serialize,
{
    fn to_payload(self) -> Result<Payload> {
        let value = serde_json::to_value(self)?;

        if let serde_json::Value::Object(map) = value {
            let converted_map: HashMap<String, QdrantValue> =
                map.into_iter().map(|(k, v)| (k, QdrantValue::from(v))).collect();
            Ok(Payload::from(converted_map))
        } else {
            // If the value is not an object, wrap it in a map with a generic key.
            let mut map = HashMap::new();
            map.insert("value".to_string(), QdrantValue::from(value));
            Ok(Payload::from(map))
        }
    }
}

/// Represents a found point in the vector database.
pub struct FoundPoint {
    pub id: u64,
    pub score: f32,
    pub payload: Option<HashMap<String, QdrantValue>>, // assuming Value is from serde_json
}

pub type Value = QdrantValue;

/// Represents search conditions for the Qdrant wrapper.
pub enum Condition {
    Matches(String, Value), // Assuming Value is from serde_json or your own type
                            // Add more conditions as per qdrant's capabilities
}

/// Converts a `Value` to a `MatchValue` for use in a `Condition`.
fn convert_to_match_value(value: qdrant_client::qdrant::Value) -> qdrant_client::qdrant::r#match::MatchValue {
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
    /// use orca_core::qdrant::Qdrant;
    ///
    /// let client = Qdrant::new("http://localhost:6334").unwrap();
    /// ```
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let config = QdrantClientConfig::from_url(url);
        let client = QdrantClient::new(Some(config))?;
        Ok(Qdrant { client })
    }

    /// Creates a new `Qdrant` instance given an existing `QdrantClient`.
    /// This is useful if you want to use a custom `QdrantClient` instance.
    ///
    /// # Arguments
    /// * `client` - An existing `QdrantClient` instance.
    ///
    /// # Example
    /// ```
    /// use orca_core::qdrant::Qdrant;
    /// use qdrant_client::prelude::QdrantClient;
    ///
    /// let client = QdrantClient::new(None).unwrap();
    /// let qdrant = Qdrant::from_client(client);
    /// ```
    pub fn from_client(client: QdrantClient) -> Self {
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
    /// # use orca_core::qdrant::Qdrant;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Qdrant::new("http://localhost:6334").unwrap();
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
    /// # use orca_core::qdrant::Qdrant;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Qdrant::new("http://localhost:6334").unwrap();
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
    /// # use orca_core::qdrant::Qdrant;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct MyPayload {
    /// #     name: String,
    /// #     age: u8,
    /// # }
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Qdrant::new("http://localhost:6334").unwrap();
    /// let collection_name = "my_collection";
    /// let vector = vec![0.1, 0.2, 0.3];
    /// let payload = MyPayload { name: "John".to_string(), age: 30 };
    /// client.insert(collection_name, vector, payload).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert<T>(&self, collection_name: &str, vector: Vec<f32>, payload: T) -> Result<()>
    where
        T: ToPayload,
    {
        let payload: Payload = payload.to_payload()?;
        let points = vec![PointStruct::new(0, vector, payload)];
        self.client.upsert_points_blocking(collection_name, None, points, None).await?;
        Ok(())
    }

    /// Inserts multiple vectors and their corresponding payloads into the specified collection.
    ///
    /// # Arguments
    /// * `collection_name` - The name of the collection to insert the vectors and payloads into.
    /// * `vectors` - A vector of vectors, where each inner vector represents a vector to be inserted.
    /// * `payloads` - A vector of payloads, where each payload corresponds to a vector to be inserted.
    ///
    /// # Returns
    /// Returns a `Result` indicating whether the operation was successful or not.
    ///
    /// # Type Parameters
    /// * `T` - The type of the payload.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use orca_core::qdrant::Qdrant;
    /// # use std::error::Error;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn Error>> {
    /// # let client = Qdrant::new("http://localhost:6334").unwrap();
    /// let vectors = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    /// let payloads = vec!["payload1".to_string(), "payload2".to_string()];
    /// client.insert_many("collection_name", vectors, payloads).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert_many<T>(
        &self,
        collection_name: &str,
        vectors: Vec<Vec<f32>>,
        payloads: Vec<T>,
    ) -> anyhow::Result<()>
    where
        T: ToPayload,
    {
        let points_result: anyhow::Result<Vec<PointStruct>> = vectors
            .into_iter()
            .zip(payloads.into_iter())
            .enumerate()
            .map(|(id, (vector, payload))| {
                let payload =
                    payload.to_payload().with_context(|| format!("Failed to convert payload at index {}", id))?;
                Ok(PointStruct::new(id as u64, vector, payload))
            })
            .collect();

        let points = points_result?;

        self.client.upsert_points_blocking(collection_name, None, points, None).await?;
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
    /// # use orca_core::qdrant::{Qdrant, Condition};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Qdrant::new("http://localhost:6334").unwrap();
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

    const URL: &str = "http://localhost:6334";

    fn generate_unique_collection_name() -> String {
        let rng = rand::thread_rng();
        let suffix: String = rng.sample_iter(&Alphanumeric).take(8).map(char::from).collect();
        format!("test_collection_{}", suffix)
    }

    async fn teardown(collection_name: &str) {
        let qdrant = Qdrant::new(URL).unwrap();
        let _ = qdrant.delete_collection(collection_name).await;
    }

    #[tokio::test]
    async fn test_create_collection() {
        let qdrant = Qdrant::new(URL).unwrap();
        let unique_collection_name = generate_unique_collection_name();

        let result = qdrant.create_collection(&unique_collection_name, 128).await;
        assert!(result.is_ok());

        teardown(&unique_collection_name).await;
    }

    #[tokio::test]
    async fn test_insert_point() {
        let qdrant = Qdrant::new(URL).unwrap();
        let unique_collection_name = generate_unique_collection_name();

        qdrant.create_collection(&unique_collection_name, 3).await.unwrap();

        let vector = vec![0.1, 0.2, 0.3];
        let payload = "some_payload";

        let result = qdrant.insert(&unique_collection_name, vector, payload).await;
        assert!(result.is_ok());

        teardown(&unique_collection_name).await;
    }

    #[tokio::test]
    async fn test_search_points() {
        let qdrant = Qdrant::new(URL).unwrap();
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
