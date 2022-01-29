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

use cairo::ImageSurface;

use super::error::ImageError;

use crate::outputs::Mode;
use clap::ArgEnum;

#[derive(PartialEq, Debug, Clone, Copy, ArgEnum)]
pub enum Filter {
    Fast,
    Good,
    Best,
    Nearest,
    Bilinear,
    Gaussian,
}

#[derive(PartialEq, Debug, Clone, Copy, ArgEnum)]
pub enum Scaling {
    Fill,
    Fit,
    None,
}

impl Scaling {
    pub fn scale(
        &self,
        sur: &ImageSurface,
        geometry: &Mode,
        filter: Filter,
    ) -> Result<Vec<u8>, ImageError> {
        match self {
            Scaling::Fill => Scaling::fill(sur, geometry, filter),
            Scaling::Fit => Scaling::fit(sur, geometry, filter),
            Scaling::None => Scaling::none(sur, geometry),
        }
    }

    fn none(buf: &ImageSurface, geometry: &Mode) -> Result<Vec<u8>, ImageError> {
        let pad_width = (*geometry.width() as f64 - buf.width() as f64) / 2.0;
        let pad_height = (*geometry.height() as f64 - buf.height() as f64) / 2.0;

        {
            let target = cairo::ImageSurface::create(
                cairo::Format::Rgb24,
                *geometry.width(),
                *geometry.height(),
            )
            .map_err(ImageError::CouldNotCreateSurface)?;
            let ctx = cairo::Context::new(&target).map_err(ImageError::CouldNotCreateContext)?;
            ctx.set_source_surface(buf, pad_width, pad_height)
                .map_err(ImageError::CouldNotSetSource)?;
            ctx.paint().map_err(ImageError::CouldNotWriteResult)?;
            drop(ctx);

            Ok(target
                .take_data()
                .map_err(ImageError::CouldNotGetData)?
                .to_vec())
        }
    }

    fn fit(buf: &ImageSurface, geometry: &Mode, filter: Filter) -> Result<Vec<u8>, ImageError> {
        Scaling::fill_or_fit(buf, geometry, filter, f64::min)
    }

    fn fill(buf: &ImageSurface, geometry: &Mode, filter: Filter) -> Result<Vec<u8>, ImageError> {
        Scaling::fill_or_fit(buf, geometry, filter, f64::max)
    }

    fn fill_or_fit<F: Fn(f64, f64) -> f64>(
        buf: &ImageSurface,
        geometry: &Mode,
        filter: Filter,
        comp: F,
    ) -> Result<Vec<u8>, ImageError> {
        // 1. Crop the image if necessary
        // 2. Scale the image to the proper size

        let height_ratio = *geometry.height() as f64 / buf.height() as f64;
        let width_ratio = *geometry.width() as f64 / buf.width() as f64;
        let max_ratio = comp(height_ratio, width_ratio);

        // Get cropping edges (aspect)
        let crop_height = ((buf.height() as f64 * max_ratio) as i32)
            .checked_sub(*geometry.height())
            .map(|elem| (elem / 2) as f64 / max_ratio)
            .unwrap_or(0.0)
            .clamp(-(*geometry.height() as f64), *geometry.height() as f64);
        let crop_width = ((buf.width() as f64 * max_ratio) as i32)
            .checked_sub(*geometry.width())
            .map(|elem| (elem / 2) as f64 / max_ratio)
            .unwrap_or(0.0)
            .clamp(-(*geometry.width() as f64), *geometry.width() as f64);
        // Create context and scale and crop to fit
        {
            let target = cairo::ImageSurface::create(
                cairo::Format::Rgb24,
                *geometry.width(),
                *geometry.height(),
            )
            .map_err(ImageError::CouldNotCreateSurface)?;
            let ctx = cairo::Context::new(&target).map_err(ImageError::CouldNotCreateContext)?;
            ctx.scale(max_ratio, max_ratio);
            ctx.set_source_surface(buf, -crop_width, -crop_height)
                .map_err(ImageError::CouldNotSetSource)?;
            ctx.source().set_filter(filter.into());
            ctx.paint().map_err(ImageError::CouldNotWriteResult)?;
            drop(ctx);

            let data = target
                .take_data()
                .map_err(ImageError::CouldNotGetData)?
                .to_vec();
            Ok(data
                .chunks_exact(4)
                .flat_map(|arr| [arr[2], arr[1], arr[0]])
                .collect())
        }
    }
}

impl From<Filter> for cairo::Filter {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Fast => cairo::Filter::Fast,
            Filter::Good => cairo::Filter::Good,
            Filter::Best => cairo::Filter::Best,
            Filter::Nearest => cairo::Filter::Nearest,
            Filter::Bilinear => cairo::Filter::Bilinear,
            Filter::Gaussian => cairo::Filter::Gaussian,
        }
    }
}
