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

// Add extra decoder function to webp images as `image` does not support most common formats
use image::DynamicImage;
use log::debug;
use std::{io::Read, path::Path};

use super::error::ImageError;

pub fn open<P: AsRef<Path>>(path: P) -> Result<DynamicImage, ImageError> {
    debug!("Fallback to separate webp decoder. Format was not supported.");
    let mut file = std::fs::OpenOptions::new().read(true).open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let decoder = webp::Decoder::new(&data);
    Ok(decoder.decode().unwrap().to_image())
}
