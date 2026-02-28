pub mod pipeline;
pub mod stages;
pub mod traits;

pub use pipeline::Pipeline;
pub use traits::{PipelineContext, PipelineInput, PipelineOutput, Stage};
