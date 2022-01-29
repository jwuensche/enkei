// Add extra decoder function to webp images as `image` does not support most common formats

use std::{path::Path, io::Read};
use image::{DynamicImage, GenericImageView};
use log::debug;

use super::error::ImageError;

pub fn open<P: AsRef<Path>>(path: P) -> Result<DynamicImage, ImageError> {
    debug!("Fallback to separate webp decoder. Format was not supported.");
    let mut file = std::fs::OpenOptions::new().read(true).open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let decoder = webp::Decoder::new(&mut data);
    Ok(decoder.decode().unwrap().to_image())
}
