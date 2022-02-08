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

use std::num::NonZeroU32;

use cairo::ImageSurface;
use image::DynamicImage;
use log::debug;

use super::error::ImageError;

use crate::outputs::ScaledMode;
use clap::ArgEnum;

#[derive(PartialEq, Debug, Clone, Copy, ArgEnum)]
pub enum Filter {
    Fast,
    Good,
    Best,
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
        sur: &DynamicImage,
        geometry: &ScaledMode,
        filter: Filter,
    ) -> Result<Vec<u8>, ImageError> {
        match self {
            Scaling::Fill => Scaling::fill(sur, geometry, filter),
            Scaling::Fit => Scaling::fit(sur, geometry, filter),
            Scaling::None => Scaling::none(sur, geometry),
        }
    }

    fn none(buf: &DynamicImage, geometry: &ScaledMode) -> Result<Vec<u8>, ImageError> {
        let pad_width = (geometry.width as f64 - buf.width() as f64) / 2.0;
        let pad_height = (geometry.height as f64 - buf.height() as f64) / 2.0;

        {
            let image_data: Vec<u8> = buf
                .to_rgba8()
                .chunks_exact(3)
                .flat_map(|arr| [arr[2], arr[1], arr[0], 0])
                .collect();
            let width = buf.width();
            let height = buf.height();
            let stride = cairo::Format::Rgb24.stride_for_width(width).map_err(|_| {
                ImageError::Generic(format!(
                    "The stride could not be determined for width {}",
                    width
                ))
            })?;
            let surface = ImageSurface::create_for_data(
                image_data,
                cairo::Format::Rgb24,
                width as i32,
                height as i32,
                stride,
            )
            .map_err(ImageError::CouldNotCreateSurface)?;

            let target =
                cairo::ImageSurface::create(cairo::Format::Rgb24, geometry.width, geometry.height)
                    .map_err(ImageError::CouldNotCreateSurface)?;
            let ctx = cairo::Context::new(&target).map_err(ImageError::CouldNotCreateContext)?;
            ctx.set_source_surface(&surface, pad_width, pad_height)
                .map_err(ImageError::CouldNotSetSource)?;
            ctx.paint().map_err(ImageError::CouldNotWriteResult)?;
            drop(ctx);

            Ok(target
                .take_data()
                .map_err(ImageError::CouldNotGetData)?
                .to_vec())
        }
    }

    fn fit(
        buf: &DynamicImage,
        geometry: &ScaledMode,
        filter: Filter,
    ) -> Result<Vec<u8>, ImageError> {
        Scaling::fill_or_fit(buf, geometry, filter, f64::min)
    }

    fn fill(
        buf: &DynamicImage,
        geometry: &ScaledMode,
        filter: Filter,
    ) -> Result<Vec<u8>, ImageError> {
        Scaling::fill_or_fit(buf, geometry, filter, f64::max)
    }

    fn fill_or_fit<F: Fn(f64, f64) -> f64>(
        buf: &DynamicImage,
        geometry: &ScaledMode,
        filter: Filter,
        comp: F,
    ) -> Result<Vec<u8>, ImageError> {
        // 1. Crop the image if necessary
        // 2. Scale the image to the proper size

        let height_ratio = geometry.height as f64 / buf.height() as f64;
        let width_ratio = geometry.width as f64 / buf.width() as f64;
        let max_ratio = comp(height_ratio, width_ratio);

        // Get cropping edges (aspect)
        // This needs to take into accordance the now larger scaled
        let crop_height = ((buf.height() as f64 * max_ratio) as i32)
            .checked_sub(geometry.height)
            .map(|elem| (elem / 2) as f64)
            .unwrap_or(0.0)
            .clamp(-(geometry.height as f64), geometry.height as f64);
        let crop_width = ((buf.width() as f64 * max_ratio) as i32)
            .checked_sub(geometry.width)
            .map(|elem| (elem / 2) as f64)
            .unwrap_or(0.0)
            .clamp(-(geometry.width as f64), geometry.width as f64);

        /*
         * SIMD Resize experiment
         */
        let width = (buf.width() as f64 * max_ratio) as u32;
        let height = (buf.height() as f64 * max_ratio) as u32;

        let image_data: Vec<u8>;
        if width != buf.width() && buf.height() != height {
            debug!("Using SIMD image resizing");
            let fallback = NonZeroU32::new(1).expect("Cannot fail");
            let orig_image = fast_image_resize::Image::from_vec_u8(
                NonZeroU32::new(buf.width() as u32).unwrap_or(fallback),
                NonZeroU32::new(buf.height() as u32).unwrap_or(fallback),
                buf.to_rgba8().into_raw(),
                fast_image_resize::PixelType::U8x4,
            )?;

            let mut scaled_image = fast_image_resize::Image::new(
                NonZeroU32::new(width).unwrap_or(fallback),
                NonZeroU32::new(height).unwrap_or(fallback),
                fast_image_resize::PixelType::U8x4,
            );
            let mut scaled_view = scaled_image.view_mut();
            let mut resizer = fast_image_resize::Resizer::new(
                fast_image_resize::ResizeAlg::Convolution(filter.into()),
            );
            // This function only fails if we use different kinds of PixelTypes
            resizer
                .resize(&orig_image.view(), &mut scaled_view)
                .expect("Cannot fail");

            image_data = scaled_image
                // Turn around because of endianness of cairo...
                .buffer()
                .chunks_exact(4)
                .flat_map(|arr| [arr[2], arr[1], arr[0], arr[3]])
                .collect();
        } else {
            debug!("No scaling required for image");
            // Flippity Flop, it's time to stop
            image_data = buf
                .to_rgba8()
                .into_raw()
                .chunks_exact(4)
                .flat_map(|arr| [arr[2], arr[1], arr[0], arr[3]])
                .collect();
        }

        let stride = cairo::Format::ARgb32.stride_for_width(width).map_err(|_| {
            ImageError::Generic(format!(
                "The stride could not be determined for width {}",
                width
            ))
        })?;
        let surface = ImageSurface::create_for_data(
            image_data,
            cairo::Format::ARgb32,
            width as i32,
            height as i32,
            stride,
        )
        .map_err(ImageError::CouldNotCreateSurface)?;

        // Create context and scale and crop to fit
        {
            let target: ImageSurface =
                cairo::ImageSurface::create(cairo::Format::Rgb24, geometry.width, geometry.height)
                    .map_err(ImageError::CouldNotCreateSurface)?;
            let ctx = cairo::Context::new(&target).map_err(ImageError::CouldNotCreateContext)?;
            ctx.set_source_surface(&surface, -crop_width, -crop_height)
                .map_err(ImageError::CouldNotSetSource)?;
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

impl From<Filter> for fast_image_resize::FilterType {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Fast => fast_image_resize::FilterType::Bilinear,
            Filter::Good => fast_image_resize::FilterType::CatmullRom,
            Filter::Best => fast_image_resize::FilterType::Lanczos3,
        }
    }
}
