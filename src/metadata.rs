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

use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::Local;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::{ops::Range, path::Path};
use thiserror::Error;

use crate::schema::gnome_xml::{Background, Image};

pub struct MetadataReader {}

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("Invalid dynamic wallpaper definition. The first item has to be a time defintion in ymdhms.")]
    InvalidTimeFormat,
    #[error("The first item must be a start time definition")]
    InvalidTime,
    #[error("Could not open wallpaper description file: `{0}`")]
    CouldNotOpen(String),
    #[error("Could not parse wallpaper description: `{0}`")]
    CouldNotParse(String),
    #[error("Cannot determine current frame.")]
    CurrentFrame,
}

impl MetadataReader {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Metadata, MetadataError> {
        let config_file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|e| MetadataError::CouldNotOpen(format!("{}", e)))?;
        let config: Background = serde_xml_rs::from_reader(config_file)
            .map_err(|e| MetadataError::CouldNotParse(format!("{}", e)))?;
        // Sanity Checks and Transition
        let mut transitions = vec![];
        let start_time = {
            if let Image::StartTime {
                year,
                month,
                day,
                hour,
                minute,
                second,
            } = config
                .images
                .get(0)
                .ok_or(MetadataError::InvalidTimeFormat)?
            {
                NaiveDate::from_ymd(*year as i32, *month, *day).and_hms(*hour, *minute, *second)
            } else {
                return Err(MetadataError::InvalidTime);
            }
        };
        let mut elapsed = 0f64;

        let mut entry_iter = config.images.iter().skip(1).peekable();

        while let Some(next) = entry_iter.next() {
            let from_file;
            let duration_static;
            let mut duration_transition = 0_f64;

            if let Image::Static { duration, file } = &next {
                duration_static = *duration;
                from_file = file.clone();
            } else {
                return Err(MetadataError::CouldNotParse(format!(
                    "Was expecting <static> block but found instead: {:#?}",
                    next
                )));
            }

            if let Some(Image::Transition {
                duration, to, kind, ..
            }) = entry_iter.peek()
            {
                let kind_trans = kind.clone();
                let to_file = to.clone();
                duration_transition = *duration;

                let duration = elapsed + duration_static + duration_transition;
                transitions.push(Transition::WithAnimation {
                    kind: kind_trans,
                    from: from_file,
                    to: to_file,
                    time_range: (elapsed..duration),
                    duration_static,
                    duration_transition,
                });
                elapsed = duration;
                entry_iter.next();
            } else {
                let duration = elapsed + duration_static + duration_transition;
                transitions.push(Transition::WithoutAnimation {
                    from: from_file,
                    time_range: (elapsed..duration),
                    duration: duration_static,
                });
                elapsed = duration;
            }
        }

        let total_duration_sec = transitions.iter().fold(0f64, |acc, elem| {
            acc + elem.duration_static() + elem.duration_transition()
        });

        let meta_config = Metadata {
            start_time,
            image_transisitons: transitions,
            total_duration_sec,
        };
        Ok(meta_config)
    }

    // This is a workaround to create some basic description if only an image is given as a background
    pub fn static_configuration<P: Into<PathBuf>>(path: P) -> Metadata {
        Metadata {
            start_time: Local::now().naive_local(),
            total_duration_sec: f64::MAX,
            image_transisitons: vec![Transition::WithoutAnimation {
                // Duration is given in nanoseconds in the std, we have to go a bit smaller than that to not panic
                duration: (u64::MAX / 10) as f64,
                time_range: 0f64..f64::MAX,
                from: path.into(),
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Transition {
    WithAnimation {
        kind: String,
        duration_static: f64,
        duration_transition: f64,
        time_range: Range<f64>,
        from: PathBuf,
        to: PathBuf,
    },
    WithoutAnimation {
        duration: f64,
        time_range: Range<f64>,
        from: PathBuf,
    },
}

impl Transition {
    pub fn duration_static(&self) -> f64 {
        match self {
            Transition::WithAnimation {
                duration_static, ..
            } => *duration_static,
            Transition::WithoutAnimation { duration, .. } => *duration,
        }
    }

    pub fn duration_transition(&self) -> f64 {
        match self {
            Transition::WithAnimation {
                duration_transition,
                ..
            } => *duration_transition,
            Transition::WithoutAnimation { .. } => 0f64,
        }
    }

    pub fn is_animated(&self) -> bool {
        match self {
            Transition::WithAnimation { .. } => true,
            Transition::WithoutAnimation { .. } => false,
        }
    }

    pub fn time_range(&self) -> &Range<f64> {
        match self {
            Transition::WithAnimation { time_range, .. } => time_range,
            Transition::WithoutAnimation { time_range, .. } => time_range,
        }
    }

    pub fn from(&self) -> &PathBuf {
        match self {
            Transition::WithAnimation { from, .. } => from,
            Transition::WithoutAnimation { from, .. } => from,
        }
    }

    pub fn to(&self) -> Option<&PathBuf> {
        match self {
            Transition::WithAnimation { to, .. } => Some(to),
            Transition::WithoutAnimation { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    start_time: NaiveDateTime,
    image_transisitons: Vec<Transition>,
    total_duration_sec: f64,
}

pub enum AnimationState {
    Static(f64, Transition),
    Transition(f64, Transition),
}

impl Metadata {
    pub fn current(&self) -> Result<AnimationState, MetadataError> {
        let now = Local::now().naive_local();

        let diff = (now - self.start_time).num_seconds() as f64 % self.total_duration_sec;
        let cur = self
            .image_transisitons
            .iter()
            .find(|elem| elem.time_range().contains(&diff))
            .ok_or(MetadataError::CurrentFrame)?;

        Ok(if diff - cur.time_range().start < cur.duration_static() {
            AnimationState::Static(diff - cur.time_range().start, cur.clone())
        } else {
            AnimationState::Transition(
                diff - cur.time_range().start - cur.duration_static(),
                cur.clone(),
            )
        })
    }
}
