use std::sync::mpsc::{Receiver, channel, Sender};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use getset::Getters;
use serde::__private::de::FlatInternallyTaggedAccess;
use thiserror::Error;

use wayland_client::protocol::wl_output::{Subpixel, Transform, Mode as ModeFlag, WlOutput};
use wayland_client::protocol::{
    wl_output,
};
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
}

#[derive(Getters, Debug)]
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

#[derive(Getters, Debug)]
pub struct Geometry {
    #[get = "pub"]
    x: i32,
    #[get = "pub"]
    y: i32,
    #[get = "pub"]
    physical_width: i32,
    #[get = "pub"]
    physical_height: i32,
    #[get = "pub"]
    subpixel: Subpixel,
    #[get = "pub"]
    make: String,
    #[get = "pub"]
    model: String,
    #[get = "pub"]
    transform: Transform,
}

impl Output {
    pub fn new(inner: Main<wl_output::WlOutput>) -> Self {
        Self {
            geometry: None,
            mode: None,
            scale: 1,
            inner,
        }
    }

    pub fn geometry(&self) -> Option<&Geometry> {
        self.geometry.as_ref()
    }

    pub fn mode(&self) -> Option<&Mode> {
        self.mode.as_ref()
    }

    pub fn scale(&self) -> i32 {
        self.scale
    }

    pub fn inner(&self) -> &WlOutput {
        &self.inner
    }
}

pub struct OutputManager {
    inner: Arc<RwLock<Vec<Arc<RwLock<Output>>>>>,
}

pub fn handle_output_events(pass: &Arc<RwLock<Output>>, event: wl_output::Event, added: &Sender<WorkerMessage>) {
    match event {
        wl_output::Event::Geometry { x, y, physical_width, physical_height, subpixel, make, model, transform } => {
            let mut lock = pass.write().expect("Could not lock output object");
            lock.geometry = Some(Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            });
        },
        wl_output::Event::Mode { flags, width, height, refresh } => {
            let mut lock = pass.write().expect("Could not lock output object");
            lock.mode = Some(Mode{
                flags,
                width,
                height,
                refresh
            });
        },
        wl_output::Event::Scale { factor } => {
            let mut lock = pass.write().expect("Could not lock output object");
            lock.scale = factor;
        },
        wl_output::Event::Done => added.send(
            WorkerMessage::AddOutput(SendWrapper::new(Arc::clone(&pass)))
        ).unwrap(),
        _ => unreachable!(),
    }
}

impl OutputManager {
    pub fn new(output_handles: Arc<RwLock<Vec<Arc<RwLock<Output>>>>>) -> Self {
        let obj = Self {
            inner: output_handles,
        };
        obj
    }

    pub fn outputs(&self) -> Option<RwLockReadGuard<Vec<Arc<RwLock<Output>>>>> {
        self.inner.read().ok()
    }
}
