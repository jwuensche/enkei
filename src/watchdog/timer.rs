use std::sync::mpsc::Sender;
use crate::messages::WorkerMessage;

const ERROR_MSG: &str = "Could not send timer tick. Is the other side already dropped?";

pub fn initialize(step_duration: std::time::Duration, count: u64, tx: Sender<WorkerMessage>) {
    let mut finished = 0;
    std::thread::spawn(move || {
        loop {
            tx.send(WorkerMessage::AnimationStep(finished as f32 / count as f32)).expect(ERROR_MSG);
            if finished >= count {
                break;
            }
            std::thread::sleep(step_duration);
            finished+=1;
        }
    });
}
