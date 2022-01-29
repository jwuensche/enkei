use std::path::PathBuf;

use super::{error::ImageError, scaling::Filter, scaling::Scaling, webp};
use crate::outputs::Mode;
use cairo::ImageSurface;
use image::{GenericImageView, ImageFormat};
use log::debug;

pub struct Image {
    inner: ImageSurface,
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
        let width = image.width();
        let height = image.height();
        let image_data: Vec<u8> = image
            .to_rgb8()
            .as_raw()
            .clone()
            .chunks_exact(3)
            .flat_map(|arr| [arr[2], arr[1], arr[0], 0])
            .collect();
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
        Ok(Self {
            inner: surface,
            scaling,
            filter,
        })
    }

    pub fn process(&self, mode: &Mode) -> Result<Vec<u8>, ImageError> {
        let start = std::time::Instant::now();
        let res = self.scaling.scale(&self.inner, mode, self.filter);
        debug!("Scaling of image took {}ms", start.elapsed().as_millis());
        res
    }
}
