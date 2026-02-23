use crate::pipeline::traits::{Stage, PipelineContext};

/// 核心 Pipeline 结构体，用于编排串联整个链条
pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: Box<dyn Stage>) {
        self.stages.push(stage);
    }

    pub async fn execute(&self, context: &mut PipelineContext<'_>) -> anyhow::Result<()> {
        for (i, stage) in self.stages.iter().enumerate() {
            context.current_stage = i;
            stage.process(context).await?;
        }
        Ok(())
    }
}
