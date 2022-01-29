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
    #[error("Generic: `{0}`")]
    Generic(String),
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
