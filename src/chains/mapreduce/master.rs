use super::task::Task;
use super::worker::Worker;
use crate::chains::{chain::LLMChain, Chain};
use crate::record::Record;
use std::sync::{
    mpsc::{channel, Sender},
    Arc, Mutex,
};

pub struct Master {
    worker_channels: Vec<Sender<Task>>,
}

impl Master {
    pub fn new<'llm, C>(worker_count: usize, record: Record, chain: Arc<Mutex<dyn Chain>>, prompt: &str) -> Self {
        let mut worker_channels = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let (tx, rx) = channel();
            worker_channels.push(tx);
            Worker::new(rx, chain.clone()).spawn();
        }

        Master { worker_channels }
    }

    pub fn assign_task(&self, task: Task, record: Record) {
        let worker_id = task.get_id() % self.worker_channels.len();
        self.worker_channels[worker_id].send(task).unwrap();
    }
}
