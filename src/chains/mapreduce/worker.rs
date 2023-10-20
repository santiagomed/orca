use super::task::{TaskType, WorkerMsg, WorkerTask};
use crate::chains::Chain;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

pub(crate) struct Worker {
    receiver: Receiver<WorkerTask>,
    map_chain: Arc<Mutex<dyn Chain>>,
    reduce_chain: Arc<Mutex<dyn Chain>>,
    sender: Arc<Mutex<Sender<WorkerMsg>>>,
}

impl Worker {
    pub fn new(
        receiver: Receiver<WorkerTask>,
        map_chain: Arc<Mutex<dyn Chain>>,
        reduce_chain: Arc<Mutex<dyn Chain>>,
        sender: Arc<Mutex<Sender<WorkerMsg>>>,
    ) -> Self {
        Worker {
            receiver,
            map_chain,
            reduce_chain,
            sender,
        }
    }

    pub fn spawn(self) {
        let map_chain = self.map_chain.clone();
        let reduce_chain = self.reduce_chain.clone();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let mut receiver = self.receiver;
            while let Some(task) = receiver.recv().await {
                let mut locked_chain = match task.task_type {
                    TaskType::Map => map_chain.lock().await,
                    TaskType::Reduce => reduce_chain.lock().await,
                };
                locked_chain.load_record(&task.record_name, task.record);
                let chain_result = locked_chain.execute().await.unwrap();
                sender
                    .lock()
                    .await
                    .send(WorkerMsg {
                        task_completed: task.task_type,
                        chain_result,
                    })
                    .await
                    .unwrap();
            }
        });
    }
}
