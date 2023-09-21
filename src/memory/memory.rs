use super::Memory;
use crate::prompt::Message;
use std::fmt::{Display, Formatter};

pub struct Buffer {
    memory: Vec<Message>,
}

impl Buffer {
    /// Initialize a new Memory Buffer.
    pub fn new() -> Self {
        Self { memory: Vec::new() }
    }
}

impl<'m> Memory<'m> for Buffer {
    /// Get the memory of the Memory Buffer.
    fn get_memory(&mut self) -> &mut Vec<Message> {
        &mut self.memory
    }
}

impl Display for Buffer {
    /// Display the memory of the Memory Buffer.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for msg in &self.memory {
            write!(f, "{}", msg)?;
        }
        Ok(())
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
        }
    }
}
