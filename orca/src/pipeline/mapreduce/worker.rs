use super::task::{TaskType, WorkerMsg, WorkerTask};
use crate::pipelines::pipeline::LLMPipeline;
use crate::pipelines::Pipeline;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, RwLock};

pub(crate) struct Worker {
    receiver: Receiver<WorkerTask>,
    map_pipeline: Arc<RwLock<LLMPipeline>>,
    reduce_pipeline: Arc<RwLock<LLMPipeline>>,
    sender: Arc<Mutex<Sender<WorkerMsg>>>,
}

impl Worker {
    pub fn new(
        receiver: Receiver<WorkerTask>,
        map_pipeline: Arc<RwLock<LLMPipeline>>,
        reduce_pipeline: Arc<RwLock<LLMPipeline>>,
        sender: Arc<Mutex<Sender<WorkerMsg>>>,
    ) -> Self {
        Worker {
            receiver,
            map_pipeline,
            reduce_pipeline,
            sender,
        }
    }

    pub fn spawn(self) -> Result<()> {
        let map_pipeline = self.map_pipeline.clone();
        let reduce_pipeline = self.reduce_pipeline.clone();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let mut receiver = self.receiver;
            while let Some(task) = receiver.recv().await {
                let mut template_name = task.template_name;
                {
                    let mut locked_pipeline = match task.task_type {
                        TaskType::Map => map_pipeline.write().await,
                        TaskType::Reduce => reduce_pipeline.write().await,
                    };
                    template_name = locked_pipeline.duplicate_template(&template_name).unwrap_or_else(|| {
                        log::error!("{}", format!("Template [{}] does not exist", template_name));
                        panic!("{}", format!("Template [{}] does not exist", template_name));
                    });
                    locked_pipeline.load_record(&task.record_name, task.record);
                }
                let locked_pipeline = match task.task_type {
                    TaskType::Map => map_pipeline.read().await,
                    TaskType::Reduce => reduce_pipeline.read().await,
                };
                let pipeline_result = locked_pipeline.execute(&template_name).await.unwrap_or_else(|e| {
                    log::error!(
                        "{}",
                        format!("Error while executing pipeline [{}]: {}", locked_pipeline.name, e)
                    );
                    panic!(
                        "{}",
                        format!("Error while executing pipeline [{}]: {}", locked_pipeline.name, e)
                    );
                });
                println!("{}", pipeline_result.content());
                sender
                    .lock()
                    .await
                    .send(WorkerMsg {
                        task_completed: task.task_type,
                        pipeline_result,
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
