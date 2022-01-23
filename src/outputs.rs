use getset::Getters;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use thiserror::Error;

use wayland_client::protocol::wl_output;
use wayland_client::protocol::wl_output::{Mode as ModeFlag, WlOutput};
use wayland_client::Main;

#[derive(Error, Debug)]
pub enum OutputError<'a> {
    #[error("Output does not have member `{0}` defined")]
    KeyNotDefined(&'a str),
}

use crate::messages::WorkerMessage;
use send_wrapper::SendWrapper;

#[derive(Debug)]
pub struct Output {
    geometry: Option<Geometry>,
    mode: Option<Mode>,
    scale: i32,
    inner: Main<wl_output::WlOutput>,
    id: u32,
}

#[derive(Getters, Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub struct Mode {
    #[get = "pub"]
    flags: ModeFlag,
    #[get = "pub"]
    width: i32,
    #[get = "pub"]
    height: i32,
    // This is scaled to an absolute number of milliseconds
    #[get = "pub"]
    refresh: i32,
}

#[derive(Getters, Debug, Clone)]
pub struct Geometry {
    #[get = "pub"]
    x: i32,
    #[get = "pub"]
    y: i32,
    #[get = "pub"]
    make: String,
    #[get = "pub"]
    model: String,
}

impl Output {
    pub fn new(inner: Main<wl_output::WlOutput>, id: u32) -> Self {
        Self {
            geometry: None,
            mode: None,
            scale: 1,
            inner,
            id,
        }
    }

    pub fn geometry(&self) -> Option<&Geometry> {
        self.geometry.as_ref()
    }

    pub fn mode(&self) -> Option<&Mode> {
        self.mode.as_ref()
    }

    pub fn inner(&self) -> &WlOutput {
        &self.inner
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

pub fn handle_output_events(
    pass: &Arc<RwLock<Output>>,
    event: wl_output::Event,
    added: &Sender<WorkerMessage>,
    id: u32,
) {
    match event {
        wl_output::Event::Geometry {
            x,
            y,
            physical_width: _,
            physical_height: _,
            subpixel: _,
            make,
            model,
            transform: _,
        } => {
            let mut lock = pass.write().expect("Could not lock output object");
            lock.geometry = Some(Geometry { x, y, make, model });
        }
        wl_output::Event::Mode {
            flags,
            width,
            height,
            refresh,
        } => {
            let mut lock = pass.write().expect("Could not lock output object");
            lock.mode = Some(Mode {
                flags,
                width,
                height,
                refresh,
            });
        }
        wl_output::Event::Scale { factor } => {
            let mut lock = pass.write().expect("Could not lock output object");
            lock.scale = factor;
        }
        wl_output::Event::Done => added
            .send(WorkerMessage::AddOutput(
                SendWrapper::new(Arc::clone(pass)),
                id,
            ))
            .unwrap(),
        _ => unreachable!(),
    }
}
