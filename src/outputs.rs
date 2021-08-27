use std::sync::mpsc::{Receiver, channel, Sender};
use std::sync::{Arc, RwLock};
use getset::Getters;
use serde::__private::de::FlatInternallyTaggedAccess;
use thiserror::Error;

use wayland_client::protocol::wl_output::{Subpixel, Transform, Mode as ModeFlag};
use wayland_client::protocol::{
    wl_output,
};
use wayland_client::Main;

#[derive(Error, Debug)]
pub enum OutputError<'a> {
    #[error("Output does not have member `{0}` defined")]
    KeyNotDefined(&'a str),
}

pub struct Output {
    geometry: Option<Geometry>,
    mode: Option<Mode>,
    scale: i32,
}

#[derive(Getters)]
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

#[derive(Getters)]
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
    fn new() -> Self {
        Self {
            geometry: None,
            mode: None,
            scale: 1,
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
}

pub struct OutputManager {
    outputs: Vec<Arc<RwLock<Output>>>,
    inner: Arc<RwLock<Vec<Main<wl_output::WlOutput>>>>,
    deleted: Receiver<Arc<RwLock<Output>>>,
}

fn handle_output_events(pass: &Arc<RwLock<Output>>, event: wl_output::Event, deleted: &Sender<Arc<RwLock<Output>>>) {
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
        wl_output::Event::Done => deleted.send(Arc::clone(&pass)).unwrap(),
        _ => unreachable!(),
    }
}

impl OutputManager {
    /// It is advised to `sync_roundtrip` the attached event queue to push all events through and update all given outputs as soon as possible
    pub fn new(output_handles: Arc<RwLock<Vec<Main<wl_output::WlOutput>>>>) -> Self {
        let mut outputs = Vec::new();
        let lock = output_handles.read().unwrap();
        let (tx, rx) = channel();
        for output in lock.iter() {
            let new_output = Arc::new(RwLock::new(Output::new()));
            let pass = Arc::clone(&new_output);
            let deleted = tx.clone();
            output.quick_assign(move |_, event, _| {
                handle_output_events(&pass, event, &deleted);
            });
            outputs.push(new_output);
        }
        drop(lock);
        let obj = Self {
            outputs,
            inner: output_handles,
            deleted: rx,
        };
        obj
    }

    pub fn outputs(&self) -> &Vec<Arc<RwLock<Output>>> {
        &self.outputs
    }
}
