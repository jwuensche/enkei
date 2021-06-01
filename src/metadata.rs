use anyhow::Result;
use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::Local;
use std::fs::OpenOptions;
use std::{ops::Range, path::Path};

use crate::schema::gnome_xml::{Background, Image};

pub struct MetadataReader {}

impl MetadataReader {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Metadata> {
        let config_file = OpenOptions::new()
            .read(true)
            .open(path)
            .expect("File could not be openned");
        let config: Background =
            serde_xml_rs::from_reader(config_file).expect("Content could not be read");
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
            } = config.images.get(0).ok_or(anyhow::Error::msg("Invalid dynamic wallpaper definition. The first item has to be a time defintion in ymdhms."))?
            {
                NaiveDate::from_ymd(*year as i32, *month, *day).and_hms(*hour, *minute, *second)
            } else {
                return Err(anyhow::Error::msg("First item is not starting time"));
            }
        };
        let mut elapsed = 0u64;

        let mut entry_iter = config.images.iter().peekable();

        while let Some(next) = entry_iter.next() {
            let mut kind_trans = "".to_owned();
            let mut from_file = "".to_owned();
            let mut duration_static = 0;
            let mut duration_transition = 0;

            if let Image::Static{ duration, file } = &next {
                duration_static = *duration as u32;
                from_file = file.clone();
            }

            if let Some(Image::Transition { duration, to, kind, .. }) = entry_iter.peek() {
                kind_trans = kind.clone();
                let to_file = to.clone();
                duration_transition = *duration as u32;

                let duration = elapsed + duration_static as u64 + duration_transition as u64;
                transitions.push(Transition {
                    kind: kind_trans,
                    from: from_file,
                    to: Some(to_file),
                    time_range: (elapsed..duration),
                    duration_static,
                    duration_transition,
                });
                elapsed = duration;
                entry_iter.next();
            } else {
                let duration = elapsed + duration_static as u64 + duration_transition as u64;
                transitions.push(Transition {
                    kind: kind_trans,
                    from: from_file,
                    to: None,
                    time_range: (elapsed..duration),
                    duration_static,
                    duration_transition,
                });
                elapsed = duration;

            }

        }

        let total_duration_sec = transitions.iter().fold(0, |acc, elem| {
            acc + elem.duration_static as u64 + elem.duration_transition as u64
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
            image_transisitons: vec![Transition {
                kind: "overlay".to_owned(),
                duration_static: u32::max_value(),
                duration_transition: 0,
                time_range: 0..u64::max_value(),
                from: path.to_string(),
                to: None,
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub kind: String,
    pub duration_static: u32,
    pub duration_transition: u32,
    time_range: Range<u64>,
    pub from: String,
    pub to: Option<String>,
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
            .find(|elem| elem.time_range.contains(&diff))
            .ok_or("Error in search")?;

        Ok(
            if diff - cur.time_range.start < cur.duration_static as u64 {
                State::Static((diff - cur.time_range.start) as u32, cur.clone())
            } else {
                State::Transition(
                    (diff - cur.time_range.start - cur.duration_static as u64) as u32,
                    cur.clone(),
                )
            },
        )
    }
}
