use std::fmt::Display;

use crate::pipelines::PipelineResult;
use crate::record::Record;

#[derive(PartialEq)]
pub(crate) enum TaskType {
    Map,
    Reduce,
}

pub struct Task {
    pub template_name: String,
    pub records: Vec<(String, Record)>,
}

impl Task {
    pub fn new(template_name: String, records: Vec<(String, Record)>) -> Self {
        Self { template_name, records }
    }
}

pub(crate) struct WorkerTask {
    pub task_type: TaskType,
    pub template_name: String,
    pub record_name: String,
    pub record: Record,
}

pub(crate) struct WorkerMsg {
    pub task_completed: TaskType,
    pub pipeline_result: PipelineResult,
}

impl Display for WorkerMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pipeline_result.content())
    }
}
