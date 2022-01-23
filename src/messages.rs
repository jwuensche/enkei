use super::outputs::Output;
use send_wrapper::SendWrapper;
use std::sync::RwLock;
use std::rc::Rc;

#[derive(Debug)]
pub enum WorkerMessage {
    // This is currently a hack, we never construct this message from another thread than the main one. But other
    // varians might just be, so the SendWrapper is here to allow us to keep this as one message and keep the main events as one channel receiver.
    AddOutput(SendWrapper<Rc<RwLock<Output>>>, u32),
    RemoveOutput(u32),
    AnimationStep(f32),
    AnimationStart(f64),
    Refresh,
}
