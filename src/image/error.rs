use thiserror::Error;

use cairo::Error as CairoError;
use cairo::BorrowError as CairoBorrowError;

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
}
