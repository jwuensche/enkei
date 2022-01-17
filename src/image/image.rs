use super::{error::ImageError, scaling::Scaling, scaling::Filter};
use cairo::ImageSurface;
use image::{Pixel, GenericImageView};
use crate::outputs::Mode;

pub struct Image {
    inner: ImageSurface,
    scaling: Scaling,
    filter: Filter,
}

impl Image {
    pub fn new(path: &str, scaling: Scaling, filter: Filter) -> Result<Self, ImageError> {
        let image = image::open(path)?;
        let width = image.width();
        let height = image.height();
        let image_data: Vec<u8> = image.to_rgb8().as_raw().clone().chunks_exact(3).flat_map(|arr| {
            [arr[2], arr[1], arr[0], 0]
        }).collect();
        let stride = cairo::Format::Rgb24
            .stride_for_width(width)
            .map_err(|_| ImageError::Generic(format!("The stride could not be determined for width {}", width)))?;
        let surface = ImageSurface::create_for_data(image_data, cairo::Format::Rgb24, width as i32, height as i32, stride)
            .map_err(|e| ImageError::CouldNotCreateSurface(e))?;
        Ok(Self {
            inner: surface,
            scaling,
            filter,
        })
    }

    pub fn process(&self, mode: &Mode) -> Result<Vec<u8>, ImageError> {
        self.scaling.scale(&self.inner, mode, self.filter)
    }
}
