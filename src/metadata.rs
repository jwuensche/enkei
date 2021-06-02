use anyhow::Result;
use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::Local;
use std::fs::OpenOptions;
use std::{ops::Range, path::Path};

use crate::schema::gnome_xml::{Background, Image};

pub struct MetadataReader {}

impl MetadataReader {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Metadata> {
        let config_file = OpenOptions::new().read(true).open(path)?;
        let config: Background =
            serde_xml_rs::from_reader(config_file)?;
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
            } = config.images.get(0).ok_or_else(|| anyhow::Error::msg("Invalid dynamic wallpaper definition. The first item has to be a time defintion in ymdhms."))?
            {
                NaiveDate::from_ymd(*year as i32, *month, *day).and_hms(*hour, *minute, *second)
            } else {
                return Err(anyhow::Error::msg("First item is not starting time"));
            }
        };
        let mut elapsed = 0u64;

        let mut entry_iter = config.images.iter().peekable();

        while let Some(next) = entry_iter.next() {
            let mut from_file = "".to_owned();
            let mut duration_static = 0;
            let mut duration_transition = 0;

            if let Image::Static { duration, file } = &next {
                duration_static = *duration as u32;
                from_file = file.clone();
            }

            if let Some(Image::Transition {
                duration, to, kind, ..
            }) = entry_iter.peek()
            {
                let kind_trans = kind.clone();
                let to_file = to.clone();
                duration_transition = *duration as u32;

                let duration = elapsed + duration_static as u64 + duration_transition as u64;
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
                let duration = elapsed + duration_static as u64 + duration_transition as u64;
                transitions.push(Transition::WithoutAnimation {
                    from: from_file,
                    time_range: (elapsed..duration),
                    duration: duration_static,
                });
                elapsed = duration;
            }
        }

        let total_duration_sec = transitions.iter().fold(0, |acc, elem| {
            acc + elem.duration_static() as u64 + elem.duration_transition() as u64
        });

        let meta_config = Metadata {
            start_time,
            image_transisitons: transitions,
            total_duration_sec,
        };
        Ok(meta_config)
    }

    pub fn stat(path: &str) -> Metadata {
        Metadata {
            start_time: Local::now().naive_local(),
            total_duration_sec: u64::max_value(),
            image_transisitons: vec![Transition::WithoutAnimation {
                duration: u32::max_value(),
                time_range: 0..u64::max_value(),
                from: path.to_string(),
            }],
        }
    }
}

//pub struct Transition {
//    pub kind: String,
//    pub duration_static: u32,
//    pub duration_transition: u32,
//    time_range: Range<u64>,
//    pub from: String,
//    pub to: Option<String>,
//}

#[derive(Debug, Clone)]
pub enum Transition {
    WithAnimation {
        kind: String,
        duration_static: u32,
        duration_transition: u32,
        time_range: Range<u64>,
        from: String,
        to: String,
    },
    WithoutAnimation {
        duration: u32,
        time_range: Range<u64>,
        from: String,
    },
}

impl Transition {
    pub fn duration_static(&self) -> u32 {
        match self {
            Transition::WithAnimation {
                duration_static, ..
            } => *duration_static,
            Transition::WithoutAnimation { duration, .. } => *duration,
        }
    }

    pub fn duration_transition(&self) -> u32 {
        match self {
            Transition::WithAnimation {
                duration_transition,
                ..
            } => *duration_transition,
            Transition::WithoutAnimation { .. } => 0,
        }
    }

    pub fn is_animated(&self) -> bool {
        match self {
            Transition::WithAnimation { .. } => true,
            Transition::WithoutAnimation { .. } => false,
        }
    }

    pub fn time_range(&self) -> &Range<u64> {
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
    total_duration_sec: u64,
}

pub enum State {
    Static(u32, Transition),
    Transition(u32, Transition),
}

impl Metadata {
    pub fn current(&self) -> Result<State, String> {
        let now = Local::now().naive_local();

        let diff = (now - self.start_time).num_seconds() as u64 % self.total_duration_sec;
        let cur = self
            .image_transisitons
            .iter()
            .find(|elem| elem.time_range().contains(&diff))
            .ok_or("Error in search")?;

        Ok(
            if diff - cur.time_range().start < cur.duration_static() as u64 {
                State::Static((diff - cur.time_range().start) as u32, cur.clone())
            } else {
                State::Transition(
                    (diff - cur.time_range().start - cur.duration_static() as u64) as u32,
                    cur.clone(),
                )
            },
        )
    }
}
