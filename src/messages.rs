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

use super::outputs::Output;
use send_wrapper::SendWrapper;
use std::rc::Rc;
use std::sync::RwLock;

#[derive(Debug)]
pub enum WorkerMessage {
    // This is currently a hack, we never construct this message from another thread than the main one. But other
    // varians might just be, so the SendWrapper is here to allow us to keep this as one message and keep the main events as one channel receiver.
    AddOutput(SendWrapper<Rc<RwLock<Output>>>, u32),
    RemoveOutput(u32),
    AnimationStep(f32),
    AnimationStart(f64),
    Refresh,
}
