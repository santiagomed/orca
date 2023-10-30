use crate::prompt::chat::Message;
use crate::prompt::Prompt;

use anyhow::Result;
use std::fmt::{Display, Formatter};

pub trait Memory: MemoryClone + Send + Sync {
    /// Get the memory of the Memory Buffer.
    fn memory(&mut self) -> &mut dyn Prompt;

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: &dyn Prompt) -> Result<()>;
}

/// We do this to allow for cloning of Box<dyn Memory>.
pub trait MemoryClone {
    fn clone_box(&self) -> Box<dyn Memory>;
}

impl<T> MemoryClone for T
where
    T: Memory + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn Memory> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Memory> {
    fn clone(&self) -> Box<dyn Memory> {
        self.clone_box()
    }
}

#[derive(Default, Debug)]
pub struct Buffer {
    memory: String,
}

impl Buffer {
    /// Initialize a new Memory Buffer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Memory for Buffer {
    /// Get the memory of the Memory Buffer.
    fn memory(&mut self) -> &mut dyn Prompt {
        &mut self.memory
    }

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: &dyn Prompt) -> Result<()> {
        self.memory = msgs.to_string();
        Ok(())
    }
}

impl Display for Buffer {
    /// Display the memory of the Memory Buffer.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.memory)
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
        }
    }
}

#[derive(Default, Debug)]
pub struct ChatBuffer {
    memory: Vec<Message>,
}

impl ChatBuffer {
    /// Initialize a new Memory Buffer.
    pub fn new() -> Self {
        Self { memory: Vec::new() }
    }
}

impl Memory for ChatBuffer {
    /// Get the memory of the Memory Buffer.
    fn memory(&mut self) -> &mut dyn Prompt {
        &mut self.memory
    }

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: &dyn Prompt) -> Result<()> {
        self.memory = msgs.to_chat()?;
        Ok(())
    }
}

impl Clone for ChatBuffer {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
        }
    }
}
