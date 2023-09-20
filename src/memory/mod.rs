use std::sync::{Mutex, Arc};

use crate::prompt::prompt::Message;

pub mod memory;

pub trait Memory {
    /// Get the memory of the Memory Buffer.
    fn get_memory(&mut self) -> &mut Vec<Message>;

    /// Load a message into the Memory Buffer.
    fn load_memory(&mut self, msgs: Vec<Message>) {
        self.get_memory().extend(msgs);
    }
}