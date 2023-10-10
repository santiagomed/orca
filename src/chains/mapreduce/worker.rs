use super::task::Task;
use crate::chains::Chain;
use crate::record::Record;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Worker {
    receiver: Receiver<Task>,
    chain: Arc<Mutex<Box<dyn Chain>>>,
}

impl Worker {
    pub fn new<'llm>(receiver: Receiver<Task>, chain: Box<dyn Chain>) -> Self {
        Worker {
            receiver,
            chain: Arc::new(Mutex::new(chain)),
        }
    }

    pub fn spawn(self) {
        let chain = self.chain.clone();
        thread::spawn(move || {
            for task in self.receiver.iter() {
                match task {
                    Task::Map {
                        prompt,
                        record_name,
                        record,
                    } => {
                        let mut locked_chain = chain.lock().unwrap();
                        locked_chain.load_record(&record_name, record);
                        locked_chain.execute();
                    }
                    Task::Reduce { prompt, data } => {
                        // TODO: Execute the reduce function using data
                        let mut locked_chain = chain.lock().unwrap();
                        locked_chain.execute();
                    }
                }
            }
        });
    }
}
