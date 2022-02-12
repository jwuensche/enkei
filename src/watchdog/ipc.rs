use std::{io::Read, os::unix::net::UnixListener, path::PathBuf};

use crossbeam_channel::Sender;
use log::{debug, error, warn};
use serde::Deserialize;

use crate::{
    image::scaling::{Filter, Scaling},
    messages::WorkerMessage,
    Mode,
};
use thiserror::Error;

#[derive(Error, Debug)]
enum InnerError {
    #[error("XDG_RUNTIME_DIR could not be found in environment.")]
    XdgRuntimeDirNotDefined,
    #[error("Could not create socket. `{0}`")]
    SocketCreation(std::io::Error),
}

#[derive(Deserialize, Debug)]
pub struct Message {
    pub filter: Option<Filter>,
    pub scaling: Option<Scaling>,
    pub path: PathBuf,
    pub mode: Option<Mode>,
}

// Spawn an additional thread solely for receiving messages from `enkeictl`
// Paths are checked for validity and then passed as a file handle to
// the worker loop.
// Errors related to socket creation are not fatal and will be logged for the user
pub fn spawn(tx: Sender<WorkerMessage>) {
    if let Err(e) = spawn_inner(tx) {
        warn!("Could not spawn IPC socket. Reason: {e}");
    }
}

fn spawn_inner(tx: Sender<WorkerMessage>) -> Result<(), InnerError> {
    let runtime_dir: PathBuf = std::env::var("XDG_RUNTIME_DIR")
        .map_err(|_| InnerError::XdgRuntimeDirNotDefined)?
        .into();
    let socket_path = runtime_dir.join("enkei-ipc.sock");
    // TODO: Clean up when leaving this may create situations where we can't
    // find the receiver side anymore and the sending is refused.
    std::fs::remove_file(&socket_path).ok();
    let socket = UnixListener::bind(&socket_path).map_err(|e| InnerError::SocketCreation(e))?;
    std::thread::spawn(move || loop {
        match socket.accept() {
            Ok((mut socket, _)) => {
                let mut res = Vec::new();
                socket.read_to_end(&mut res).ok();
                if let Ok(msg) = bincode::deserialize::<Message>(&res) {
                    if msg.path.exists() && msg.path.is_file() {
                        debug!("Received path {{ {:?} }}", msg.path);
                        tx.send(WorkerMessage::IPCConfigUpdate(msg))
                            .expect("Cannot fail");
                    } else {
                        debug!(
                            "Received a message {{ {:?} }} but it was no valid path. Dropping...",
                            msg.path
                        )
                    }
                }
            }
            Err(_) => {}
        }
    });
    Ok(())
}
