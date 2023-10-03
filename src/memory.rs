use std::fmt::{Display, Formatter};

use anyhow::Result;

use crate::prompt::chat::{clean_json_string, Message};

pub trait MemoryData {
    fn save(&mut self, data: &str) -> Result<()>;
    fn to_string(&self) -> Result<String>;
    fn to_vec(&self) -> Result<Vec<Message>>;
}

impl MemoryData for Vec<Message> {
    fn save(&mut self, data: &str) -> Result<()> {
        println!("Saving data: {}", data);
        let msgs = serde_json::from_str::<Vec<Message>>(&format!("[{}]", &clean_json_string(data)))?;
        self.extend(msgs);
        Ok(())
    }

    fn to_string(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    fn to_vec(&self) -> Result<Vec<Message>> {
        Ok(self.clone())
    }
}
impl MemoryData for String {
    fn save(&mut self, data: &str) -> Result<()> {
        self.push_str(data);
        Ok(())
    }

    fn to_string(&self) -> Result<String> {
        Ok(self.clone())
    }

    fn to_vec(&self) -> Result<Vec<Message>> {
        Err(anyhow::anyhow!("Unable to convert String to Vec<Message>"))
    }
}

pub trait Memory: MemoryClone {
    /// Get the memory of the Memory Buffer.
    fn memory(&mut self) -> &mut dyn MemoryData;

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: &mut dyn MemoryData);
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
    fn memory(&mut self) -> &mut dyn MemoryData {
        &mut self.memory
    }

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: &mut dyn MemoryData) {
        self.memory = msgs.to_string().unwrap();
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
    fn memory(&mut self) -> &mut dyn MemoryData {
        &mut self.memory
    }

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: &mut dyn MemoryData) {
        self.memory = msgs.to_vec().unwrap();
    }
}

impl Clone for ChatBuffer {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
        }
    }
}

pub mod template {
    pub static CHAT_TEMPLATE: &str = r#"
    {{#system}}
    The following is a conversation between a human and an AI.
    The AI is talkative and provides lots of specific details from its context.
    If the AI does not know the answer to a question, it truthfully says it does not know.
    Current conversation:
    {{memory}}
    {{/system}} 
    "#;
}
