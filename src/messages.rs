use super::outputs::Output;
use send_wrapper::SendWrapper;
use std::sync::RwLock;
use std::rc::Rc;

#[derive(Debug)]
pub enum WorkerMessage {
    AddOutput(SendWrapper<Rc<RwLock<Output>>>, u32),
    RemoveOutput(u32),
    AnimationStep(f32),
    AnimationStart(f64),
    Refresh,
}
