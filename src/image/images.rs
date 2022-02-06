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

use std::path::PathBuf;

use super::{error::ImageError, scaling::Filter, scaling::Scaling, webp};
use crate::outputs::ScaledMode;
use image::{DynamicImage, ImageFormat};
use log::debug;

pub struct Image {
    inner: DynamicImage,
    scaling: Scaling,
    filter: Filter,
}

impl Image {
    pub fn new(path: PathBuf, scaling: Scaling, filter: Filter) -> Result<Self, ImageError> {
        let image = {
            let image = image::open(&path);
            if let Err(image::ImageError::Unsupported(e)) = &image {
                match e.format_hint() {
                    image::error::ImageFormatHint::Exact(ImageFormat::WebP) => webp::open(&path)?,
                    _ => image?,
                }
            } else {
                image?
            }
        };
        Ok(Self {
            inner: image,
            scaling,
            filter,
        })
    }

    pub fn process(&self, mode: &ScaledMode) -> Result<Vec<u8>, ImageError> {
        let start = std::time::Instant::now();
        let res = self.scaling.scale(&self.inner, mode, self.filter);
        debug!(
            "Scaling of image to size {{ x: {}, y: {} }} took {}ms",
            mode.width,
            mode.height,
            start.elapsed().as_millis()
        );
        res
    }
}
