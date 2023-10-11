use super::task::Task;
use crate::chains::Chain;
use crate::record::Record;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Worker {
    receiver: Receiver<Task>,
    chain: Arc<Mutex<dyn Chain>>,
}

impl Worker {
    pub fn new<'llm>(receiver: Receiver<Task>, chain: Arc<Mutex<dyn Chain>>) -> Self {
        Worker { receiver, chain }
    }

    pub fn spawn(self) {
        let chain = self.chain.clone();
        thread::spawn(async move {
            for task in self.receiver.iter() {
                match task {
                    Task::Map {
                        prompt,
                        record_name,
                        record,
                    } => {
                        let mut locked_chain = chain.lock().unwrap();
                        locked_chain.load_record(&record_name, record);
                        let chain_result = locked_chain.execute();
                    }
                    Task::Reduce { prompt, data } => {
                        // TODO: Execute the reduce function using data
                        let mut locked_chain = chain.lock().unwrap();
                        let chain_result = locked_chain.execute();
                    }
                }
            }
        });
    }
}
