// enkei: An OpenGL accelerated wallpaper tool for wayland
// Copyright (C) 2022 Johannes Wünsche
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crossbeam_channel::Sender;
use getset::Getters;
use std::rc::Rc;
use std::sync::RwLock;

use wayland_client::protocol::wl_output;
use wayland_client::protocol::wl_output::{Mode as ModeFlag, WlOutput};
use wayland_client::Main;

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
    pass: &Rc<RwLock<Output>>,
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
                SendWrapper::new(Rc::clone(pass)),
                id,
            ))
            .expect("Handler failed and had to be aborted."),
        _ => {},
    }
}
