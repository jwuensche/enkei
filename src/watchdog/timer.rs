use crossbeam_channel::{Receiver, Sender};
use log::debug;

use crate::messages::WorkerMessage;

const ERROR_MSG: &str = "Could not send timer tick. Is the other side already dropped?";

pub fn spawn_animation_ticker(
    step_duration: std::time::Duration,
    count: u64,
    mut finished: u64,
    tx: Sender<WorkerMessage>,
    rx: Receiver<()>,
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
        if rx.try_recv().is_ok() {
            break;
        }
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
    rx: Receiver<()>,
    msg: WorkerMessage,
) {
    debug!(
        "Spawning Simple Timer {{ duration: {}s, msg: {:?} }}",
        duration.as_secs_f32(),
        msg
    );
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        if rx.try_recv().is_ok() {
            return;
        }
        tx.send(msg).expect(ERROR_MSG);
    });
}
