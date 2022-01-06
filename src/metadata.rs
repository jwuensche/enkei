use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::Local;
use std::fs::OpenOptions;
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
}

impl MetadataReader {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Metadata, MetadataError> {
        let config_file = OpenOptions::new().read(true).open(path).map_err(|e| MetadataError::CouldNotOpen(format!("{}", e)))?;
        let config: Background = serde_xml_rs::from_reader(config_file).map_err(|e| MetadataError::CouldNotParse(format!("{}", e)))?;
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
            } = config.images.get(0).ok_or_else(|| MetadataError::InvalidTimeFormat)?
            {
                NaiveDate::from_ymd(*year as i32, *month, *day).and_hms(*hour, *minute, *second)
            } else {
                return Err(MetadataError::InvalidTime);
            }
        };
        let mut elapsed = 0f64;

        let mut entry_iter = config.images.iter().peekable();

        while let Some(next) = entry_iter.next() {
            let mut from_file = "".to_owned();
            let mut duration_static = 0f64;
            let mut duration_transition = 0f64;

            if let Image::Static { duration, file } = &next {
                duration_static = *duration;
                from_file = file.clone();
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
    pub fn static_configuration(path: &str) -> Metadata {
        Metadata {
            start_time: Local::now().naive_local(),
            total_duration_sec: f64::MAX,
            image_transisitons: vec![Transition::WithoutAnimation {
                duration: f64::MAX / 1_000_000_000f64,
                time_range: 0f64..f64::MAX,
                from: path.to_string(),
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
        from: String,
        to: String,
    },
    WithoutAnimation {
        duration: f64,
        time_range: Range<f64>,
        from: String,
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

    pub fn from(&self) -> &String {
        match self {
            Transition::WithAnimation { from, .. } => from,
            Transition::WithoutAnimation { from, .. } => from,
        }
    }

    pub fn to(&self) -> Option<&String> {
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

pub enum State {
    Static(f64, Transition),
    Transition(f64, Transition),
}

impl Metadata {
    pub fn current(&self) -> Result<State, String> {
        let now = Local::now().naive_local();

        let diff = (now - self.start_time).num_seconds() as f64 % self.total_duration_sec;
        let cur = self
            .image_transisitons
            .iter()
            .find(|elem| elem.time_range().contains(&diff))
            .ok_or("Error in search")?;

        Ok(if diff - cur.time_range().start < cur.duration_static() {
            State::Static(diff - cur.time_range().start, cur.clone())
        } else {
            State::Transition(
                diff - cur.time_range().start - cur.duration_static(),
                cur.clone(),
            )
        })
    }
}
