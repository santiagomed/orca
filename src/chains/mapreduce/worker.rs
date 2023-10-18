use super::task::Task;
use crate::chains::Chain;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

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
        tokio::spawn(async move {
            let mut receiver = self.receiver;
            while let Some(task) = receiver.recv().await {
                match task {
                    Task::Map {
                        prompt,
                        record_name,
                        record,
                    } => {
                        let mut locked_chain = chain.lock().await;
                        locked_chain.load_record(&record_name, record);
                        let chain_result = locked_chain.execute().await;
                    }
                    Task::Reduce { prompt, data } => {
                        // TODO: Execute the reduce function using data
                        let mut locked_chain = chain.lock().await;
                        let chain_result = locked_chain.execute().await;
                    }
                }
            }
        });
    }
}
