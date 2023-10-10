use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::record::{self, Record};

pub enum Task {
    Map {
        prompt: String,
        record_name: String,
        record: Record,
    },
    Reduce {
        prompt: String,
        data: Vec<(String, Record)>,
    },
}

impl Task {
    pub fn get_id(&self) -> usize {
        let mut hasher = DefaultHasher::new();
        // TODO: Fix this. Hashing does not result in a unique value for each task
        // since prompts can be the same for different tasks.
        match self {
            Task::Map {
                prompt, record_name, ..
            } => {
                prompt.hash(&mut hasher);
                record_name.hash(&mut hasher);
            }
            Task::Reduce { data, .. } => {
                for (key, ..) in data {
                    key.hash(&mut hasher);
                    // value.hash(&mut hasher);
                }
            }
        }
        hasher.finish() as usize
    }
}
