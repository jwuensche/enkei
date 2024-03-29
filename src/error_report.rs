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

use std::{fmt::Display, rc::Rc, sync::RwLock};

use crate::{
    metadata::{Metadata, MetadataError},
    outputs::Output,
    ApplicationError,
};

type SharedOutputs = Rc<RwLock<Vec<Rc<RwLock<Output>>>>>;

// This module serves to display a reportable error if any error occur during execution.
// It is inspired by `human_panic` which is quite nice to deal with panics.
// I will try to reproduce some more usable for the user to interpret.
pub struct ErrorReport {
    error: ApplicationError,
    metadata: Option<Metadata>,
    outputs: Option<SharedOutputs>,
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

    pub fn with_outputs(mut self, outputs: SharedOutputs) -> Self {
        self.outputs = Some(outputs);
        self
    }

    /// This method is for the cases when the error is of minor scale and probably due to some user error (typos etc.)
    fn marginal_error(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.error))
    }

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

impl Display for ErrorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}

use std::fmt::Debug;

impl Debug for ErrorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error {
            ApplicationError::InvalidDataType | ApplicationError::NotAFile(_) => {
                self.marginal_error(f)
            }
            _ => self.fmt(f),
        }
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
