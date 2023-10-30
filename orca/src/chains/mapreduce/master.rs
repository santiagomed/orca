use super::task::{Task, TaskType, WorkerMsg, WorkerTask};
use super::worker::Worker;
use crate::chains::chain::LLMChain;
use crate::chains::ChainResult;
use crate::record::{self, Record};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex, RwLock,
};

pub(crate) struct Master {
    worker_channels: Vec<Sender<WorkerTask>>,
    receiver: Arc<Mutex<Receiver<WorkerMsg>>>,
    group: Option<Record>,
}

impl Master {
    pub fn new(num_workers: usize, map_chain: Arc<RwLock<LLMChain>>, reduce_chain: Arc<RwLock<LLMChain>>) -> Self {
        let mut worker_channels = Vec::new();
        let (sender, receiver) = channel::<WorkerMsg>(std::mem::size_of::<WorkerMsg>() * num_workers);
        let sender = Arc::new(Mutex::new(sender));

        for _ in 0..num_workers {
            let (tx, rx) = channel::<WorkerTask>(std::mem::size_of::<Task>() * num_workers);
            worker_channels.push(tx);
            let worker = Worker::new(rx, map_chain.clone(), reduce_chain.clone(), sender.clone());
            worker.spawn().unwrap();
        }

        Master {
            worker_channels,
            receiver: Arc::new(Mutex::new(receiver)),
            group: None,
        }
    }

    pub async fn map(mut self, task: Task) -> Self {
        let receiver_clone = self.receiver.clone();
        let record = tokio::spawn(async move {
            let mut res = Vec::<String>::new();
            while let Some(msg) = receiver_clone.lock().await.recv().await {
                if msg.task_completed == TaskType::Map {
                    res.push(msg.chain_result.content());
                } else {
                    panic!("Reduce task completed before map task.")
                }
            }
            Record::new(record::Content::Vec(res))
        });

        let mut worker_channels = self.worker_channels.clone();
        for (record_name, record) in task.records {
            let channel = worker_channels.pop().unwrap();
            channel
                .send(WorkerTask {
                    task_type: TaskType::Map,
                    template_name: task.template_name.clone(),
                    record_name,
                    record,
                })
                .await
                .unwrap();
        }

        self.group = Some(record.await.unwrap());
        self
    }

    pub async fn reduce(&self, template_name: String) -> ChainResult {
        let receiver_clone = self.receiver.clone();
        let result = tokio::spawn(async move {
            while let Some(msg) = receiver_clone.lock().await.recv().await {
                if msg.task_completed == TaskType::Reduce {
                    return msg.chain_result;
                } else {
                    panic!("Map task completed before reduce task.")
                }
            }
            panic!("No reduce task completed.")
        });

        let channel = self.worker_channels.first().unwrap();
        channel
            .send(WorkerTask {
                task_type: TaskType::Reduce,
                template_name,
                record_name: "".into(),
                record: self.group.as_ref().unwrap().clone(),
            })
            .await
            .unwrap();

        result.await.unwrap()
    }
}
