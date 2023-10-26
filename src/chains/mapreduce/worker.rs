use super::task::{TaskType, WorkerMsg, WorkerTask};
use crate::chains::chain::LLMChain;
use crate::chains::Chain;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;

pub(crate) struct Worker {
    receiver: Receiver<WorkerTask>,
    map_chain: Arc<RwLock<LLMChain>>,
    reduce_chain: Arc<RwLock<LLMChain>>,
    sender: Arc<RwLock<Sender<WorkerMsg>>>,
}

impl Worker {
    pub fn new(
        receiver: Receiver<WorkerTask>,
        map_chain: Arc<RwLock<LLMChain>>,
        reduce_chain: Arc<RwLock<LLMChain>>,
        sender: Arc<RwLock<Sender<WorkerMsg>>>,
    ) -> Self {
        Worker {
            receiver,
            map_chain,
            reduce_chain,
            sender,
        }
    }

    pub fn spawn(self) -> Result<()> {
        let map_chain = self.map_chain.clone();
        let reduce_chain = self.reduce_chain.clone();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let mut receiver = self.receiver;
            while let Some(task) = receiver.recv().await {
                {
                    let mut locked_chain = match task.task_type {
                        TaskType::Map => map_chain.blocking_write(),
                        TaskType::Reduce => reduce_chain.blocking_write(),
                    };
                    locked_chain.load_record(&task.record_name, task.record);
                }
                {
                    let locked_chain = match task.task_type {
                        TaskType::Map => map_chain.blocking_read(),
                        TaskType::Reduce => reduce_chain.blocking_read(),
                    };
                    let chain_result = locked_chain.execute("temp").await.unwrap_or_else(|e| {
                        log::error!(
                            "{}",
                            format!("Error while executing chain [{}]: {}", locked_chain.name, e)
                        );
                        panic!();
                    });
                    sender
                        .blocking_read()
                        .send(WorkerMsg {
                            task_completed: task.task_type,
                            chain_result,
                        })
                        .await
                        .unwrap_or_else(|e| {
                            log::error!("{}", format!("Error while sending message: {}", e));
                            panic!();
                        })
                }
            }
        });
        Ok(())
    }
}
