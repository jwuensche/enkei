use super::outputs::Output;
use send_wrapper::SendWrapper;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub enum WorkerMessage {
    AddOutput(SendWrapper<Arc<RwLock<Output>>>, u32),
    RemoveOutput(u32),
    AnimationStep(f32),
    AnimationStart(f64),
    Refresh,
}
