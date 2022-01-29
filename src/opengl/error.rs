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

use log::error;

pub unsafe fn check_error(pos: &str) {
    let err = gl::GetError();
    if err != 0 {
        error!("OpenGL has encountered an error ({}) ({})", err, pos);
        error!("GL_INVALID_VALUE: {}", gl::INVALID_VALUE);
        error!("GL_INVALID_ENUM: {}", gl::INVALID_ENUM);
        error!("GL_INVALID_OPERATION: {}", gl::INVALID_OPERATION);
        std::process::exit(1);
    }
}
