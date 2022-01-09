use std::sync::mpsc::Sender;
use crate::messages::WorkerMessage;

const ERROR_MSG: &str = "Could not send timer tick. Is the other side already dropped?";

pub fn spawn_animation_ticker(step_duration: std::time::Duration, count: u64, mut finished: u64, tx: Sender<WorkerMessage>) {
    tx.send(WorkerMessage::AnimationStep(finished as f32 / count as f32)).expect(ERROR_MSG);
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

pub fn spawn_simple_timer(duration: std::time::Duration, tx: Sender<WorkerMessage>, msg: WorkerMessage) {
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        tx.send(msg).expect(ERROR_MSG);
    });
}
