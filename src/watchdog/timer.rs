use log::debug;

use crate::messages::WorkerMessage;
use std::sync::mpsc::Sender;

const ERROR_MSG: &str = "Could not send timer tick. Is the other side already dropped?";

pub fn spawn_animation_ticker(
    step_duration: std::time::Duration,
    count: u64,
    mut finished: u64,
    tx: Sender<WorkerMessage>,
) {
    debug!(
        "Spawning Ticker {{ step_duration: {}s, count: {}, offset: {} }}",
        step_duration.as_secs_f32(),
        count,
        finished
    );
    tx.send(WorkerMessage::AnimationStep(finished as f32 / count as f32))
        .expect(ERROR_MSG);
    std::thread::spawn(move || loop {
        tx.send(WorkerMessage::AnimationStep(finished as f32 / count as f32))
            .expect(ERROR_MSG);
        if finished >= count {
            break;
        }
        std::thread::sleep(step_duration);
        finished += 1;
    });
}

pub fn spawn_simple_timer(
    duration: std::time::Duration,
    tx: Sender<WorkerMessage>,
    msg: WorkerMessage,
) {
    debug!(
        "Spawning Simple Timer {{ duration: {}s, msg: {:?} }}",
        duration.as_secs_f32(),
        msg
    );
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        tx.send(msg).expect(ERROR_MSG);
    });
}
