use super::outputs::Output;
use super::metadata::Transition;
use std::sync::{
    Arc,
    RwLock,
};
use send_wrapper::SendWrapper;

#[derive(Debug)]
pub enum WorkerMessage {
    AddOutput(SendWrapper<Arc<RwLock<Output>>>),
    RemoveOutput(SendWrapper<Arc<RwLock<Output>>>),
    AnimationStep(f32),
    AnimationStart(f64),
    Refresh,
}
