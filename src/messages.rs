use super::outputs::Output;
use super::metadata::Transition;
use std::sync::{
    Arc,
    RwLock,
};
use send_wrapper::SendWrapper;

#[derive(Debug)]
pub enum WorkerMessage {
    AddOutput(SendWrapper<Arc<RwLock<Output>>>, u32),
    RemoveOutput(u32),
    AnimationStep(f32),
    AnimationStart(f64),
    Refresh,
}
