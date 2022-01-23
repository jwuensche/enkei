use std::{
    error::Error,
    fmt::{Display, Write},
    sync::{Arc, RwLock},
};

use crate::{
    metadata::{Metadata, MetadataError},
    outputs::Output,
    ApplicationError,
};

// This module serves to display a reportable error if any error occur during execution.
// It is inspired by `human_panic` which is quite nice to deal with panics.
// I will try to reproduce some more usable for the user to interpret.
pub struct ErrorReport {
    error: ApplicationError,
    metadata: Option<Metadata>,
    outputs: Option<Arc<RwLock<Vec<Arc<RwLock<Output>>>>>>,
}

const ERROR_HEADER: &str = "###### ERROR ######

This is unfortunate. enkei seems to have encountered an error and had to exit. We try to avoid this but sometimes it happens.
Below you can find a report on the current configuration and the cause for the crash.\n\n";

impl ErrorReport {
    pub fn new(error: ApplicationError) -> Self {
        Self {
            error,
            metadata: None,
            outputs: None,
        }
    }

    pub fn with_metadata(mut self, meta: Metadata) -> Self {
        self.metadata = Some(meta);
        self
    }

    pub fn with_outputs(mut self, outputs: Arc<RwLock<Vec<Arc<RwLock<Output>>>>>) -> Self {
        self.outputs = Some(outputs);
        self
    }

    pub fn report(self) {
        match &self.error {
            ApplicationError::InvalidDataType => self.marginal_error(),
            _ => eprintln!("{}", self),
        }
    }

    /// This method is for the cases when the error is of minor scale and probably due to some user error (typos etc.)
    fn marginal_error(&self) {
        eprintln!("Error: {}", self.error);
    }
}

impl Display for ErrorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(ERROR_HEADER)?;
        f.write_fmt(format_args!("Cause: {}\n\n", self.error))?;
        if let Some(outputs) = &self.outputs {
            f.write_fmt(format_args!("Known Outputs: {:#?}\n", outputs))?;
        } else {
            f.write_str("Known Outputs: Unavailable\n")?;
        }
        if let Some(meta) = &self.metadata {
            f.write_fmt(format_args!("Image Metadata: {:#?}\n", meta))?;
        } else {
            f.write_str("Image Metadata: Unavailable\n")?;
        }
        Ok(())
    }
}

impl From<ApplicationError> for ErrorReport {
    fn from(e: ApplicationError) -> Self {
        Self::new(e)
    }
}

impl From<MetadataError> for ErrorReport {
    fn from(e: MetadataError) -> Self {
        Self::new(ApplicationError::MetadataError(e))
    }
}
