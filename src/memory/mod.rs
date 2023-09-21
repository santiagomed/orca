use crate::prompt::Message;

pub mod memory;

pub trait Memory<'m>: MemoryClone<'m> {
    /// Get the memory of the Memory Buffer.
    fn get_memory(&mut self) -> &mut Vec<Message>;

    /// Load a message into the Memory Buffer.
    fn save_memory(&mut self, msgs: Vec<Message>) {
        *self.get_memory() = msgs;
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
