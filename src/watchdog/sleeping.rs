// Haha Sleeping Watchdog! Get it?

use crate::messages::WorkerMessage;
use chrono::offset::Local;
use log::{debug, info};

use std::sync::mpsc::Sender;

const POLL_INTERVAL_SEC: u64 = 10;

pub fn initialize(sender: Sender<WorkerMessage>) {
    debug!("Initializing Sleep Watchdog");
    std::thread::spawn(move || {
        let start = Local::now();
        std::thread::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SEC));
        let end = Local::now();
        if (end - start).num_seconds() > 10 {
            info!("Detected Sleeping Cycle. Send Refresh to worker thread.");
            sender
                .send(WorkerMessage::Refresh)
                .expect("Sleeping Watchdog could not bark!");
        }
    });
}
