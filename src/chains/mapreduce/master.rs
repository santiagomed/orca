use super::task::Task;
use super::worker::Worker;
use crate::chains::Chain;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{channel, Sender},
    Mutex,
};

pub struct Master {
    worker_channels: Vec<Sender<Task>>,
}

impl Master {
    pub fn new<'llm, C>() -> Self {
        let worker_channels = Vec::new();
        Master { worker_channels }
    }

    pub fn init_worker(mut self, chain: Arc<Mutex<dyn Chain>>) -> Self {
        let (tx, rx) = channel::<Task>(100);
        self.worker_channels.push(tx);
        let worker = Worker::new(rx, chain);
        worker.spawn();
        self
    }

    pub async fn assign_task(&self, task: Task) {
        let worker_id = task.get_id() % self.worker_channels.len();
        self.worker_channels[worker_id].send(task).await.unwrap();
    }
}
