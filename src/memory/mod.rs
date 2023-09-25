use crate::prompt::Message;
use std::fmt::{Display, Formatter};

pub trait Memory<'m>: MemoryClone<'m> {
    /// Get the memory of the Memory Buffer.
    fn memory(&mut self) -> &mut Vec<Message>;

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: Vec<Message>) {
        *self.memory() = msgs;
    }
}

/// We do this to allow for cloning of Box<dyn Memory>.
pub trait MemoryClone<'m> {
    fn clone_box(&self) -> Box<dyn Memory<'m> + 'm>;
}

impl<'m, T> MemoryClone<'m> for T
where
    T: 'm + Memory<'m> + Clone,
{
    fn clone_box(&self) -> Box<dyn Memory<'m> + 'm> {
        Box::new(self.clone())
    }
}

impl<'m> Clone for Box<dyn Memory<'m> + 'm> {
    fn clone(&self) -> Box<dyn Memory<'m> + 'm> {
        self.clone_box()
    }
}

#[derive(Default, Debug)]
pub struct Buffer {
    memory: Vec<Message>,
}

impl Buffer {
    /// Initialize a new Memory Buffer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'m> Memory<'m> for Buffer {
    /// Get the memory of the Memory Buffer.
    fn memory(&mut self) -> &mut Vec<Message> {
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
