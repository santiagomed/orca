use super::task::{TaskType, WorkerMsg, WorkerTask};
use crate::chains::chain::LLMChain;
use crate::chains::Chain;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, RwLock};

pub(crate) struct Worker {
    receiver: Receiver<WorkerTask>,
    map_chain: Arc<RwLock<LLMChain>>,
    reduce_chain: Arc<RwLock<LLMChain>>,
    sender: Arc<Mutex<Sender<WorkerMsg>>>,
}

impl Worker {
    pub fn new(
        receiver: Receiver<WorkerTask>,
        map_chain: Arc<RwLock<LLMChain>>,
        reduce_chain: Arc<RwLock<LLMChain>>,
        sender: Arc<Mutex<Sender<WorkerMsg>>>,
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
                let mut template_name = task.template_name;
                {
                    let mut locked_chain = match task.task_type {
                        TaskType::Map => map_chain.write().await,
                        TaskType::Reduce => reduce_chain.write().await,
                    };
                    template_name = locked_chain.duplicate_template(&template_name).unwrap_or_else(|| {
                        log::error!("{}", format!("Template [{}] does not exist", template_name));
                        panic!("{}", format!("Template [{}] does not exist", template_name));
                    });
                    locked_chain.load_record(&task.record_name, task.record);
                }
                let locked_chain = match task.task_type {
                    TaskType::Map => map_chain.read().await,
                    TaskType::Reduce => reduce_chain.read().await,
                };
                let chain_result = locked_chain.execute(&template_name).await.unwrap_or_else(|e| {
                    log::error!(
                        "{}",
                        format!("Error while executing chain [{}]: {}", locked_chain.name, e)
                    );
                    panic!(
                        "{}",
                        format!("Error while executing chain [{}]: {}", locked_chain.name, e)
                    );
                });
                println!("{}", chain_result.content());
                sender
                    .lock()
                    .await
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
        });
        Ok(())
    }
}
