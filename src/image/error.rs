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

use image::error::DecodingError;
use image::error::LimitError;
use image::error::UnsupportedError;
use thiserror::Error;

use cairo::BorrowError as CairoBorrowError;
use cairo::Error as CairoError;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("Could not create ImageSurface with Cairo: `{0}`")]
    CouldNotCreateSurface(CairoError),
    #[error("Could not create Context with Cairo: `{0}`")]
    CouldNotCreateContext(CairoError),
    #[error("Could not set source Surface to Context with Cairo: `{0}`")]
    CouldNotSetSource(CairoError),
    #[error("Could not write Result of Context with Cairo: `{0}`")]
    CouldNotWriteResult(CairoError),
    #[error("Could not get image data with Cairo: `{0}`")]
    CouldNotGetData(CairoBorrowError),
    #[error("Could not decode image in: `{0}`")]
    CouldNotDecode(DecodingError),
    #[error("Loading Image took more resources than allowed: `{0}`")]
    ResourceLimit(LimitError),
    #[error("Unsupported: `{0}`")]
    Unsupported(UnsupportedError),
    #[error("Reading of file failed: `{0}`")]
    Io(std::io::Error),
    #[error("Image Buffer could not be interpreted: `{0}`")]
    BufferInvalid(fast_image_resize::ImageBufferError),
    #[error("Generic: `{0}`")]
    Generic(String),
}

impl From<fast_image_resize::ImageBufferError> for ImageError {
    fn from(e: fast_image_resize::ImageBufferError) -> Self {
        Self::BufferInvalid(e)
    }
}

impl From<image::error::ImageError> for ImageError {
    fn from(org: image::error::ImageError) -> Self {
        match org {
            image::ImageError::Decoding(e) => ImageError::CouldNotDecode(e),
            image::ImageError::Limits(e) => ImageError::ResourceLimit(e),
            image::ImageError::Unsupported(e) => ImageError::Unsupported(e),
            image::ImageError::IoError(e) => ImageError::Io(e),
            image::ImageError::Parameter(e) => {
                ImageError::Generic(format!("Loading of image failed: {}", e))
            }
            image::ImageError::Encoding(_) => unimplemented!(),
        }
    }
}

impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
