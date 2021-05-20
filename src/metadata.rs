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
            } = config.images.get(0).unwrap()
            {
                NaiveDate::from_ymd(*year as i32, *month, *day).and_hms(*hour, *minute, *second)
            } else {
                return Err(anyhow::Error::msg("First item is not starting time"));
            }
        };
        let mut elapsed = 0u64;
        for (_, pt) in config
            .images
            .windows(2)
            .enumerate()
            .filter(|e| e.0 % 2 == 1)
        {
            let mut kind_trans = "".to_owned();
            let mut from_file = "".to_owned();
            let mut to_file = "".to_owned();
            let mut duration_static = 0;
            let mut duration_transition = 0;

            // if matches!(pt[0], Image::Static {..}) || matches!(pt[1], Image::Transition {..}) {
            //     return Err(anyhow::Error::msg("Invalid image configuration"));
            // }
            pt.iter().for_each(|elem| match elem {
                Image::Static { duration, file } => {
                    duration_static = duration.clone() as u32;
                    from_file = file.clone();
                }
                Image::Transition {
                    duration, to, kind, ..
                } => {
                    kind_trans = kind.clone();
                    to_file = to.clone();
                    duration_transition = duration.clone() as u32;
                }
                _ => {
                    unreachable!()
                }
            });
            let duration = elapsed + duration_static as u64 + duration_transition as u64;
            transitions.push(Transition {
                kind: kind_trans,
                from: from_file,
                to: to_file,
                time_range: (elapsed..duration),
                duration_static,
                duration_transition,
            });
            elapsed = duration;
        }

        let total_duration_sec = transitions.iter().fold(0, |acc, elem| {
            acc + elem.duration_static as u64 + elem.duration_transition as u64
        });

        let meta_config = Metadata {
            start_time,
            image_transisitons: transitions,
            total_duration_sec,
        };
        // dbg!(&meta_config);
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
                to: path.to_string(),
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
    pub to: String,
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
        //dbg!(diff);
        let cur = self
            .image_transisitons
            .iter()
            .filter(|elem| elem.time_range.contains(&diff))
            .next()
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

    pub fn current_transition(&self) -> Result<Transition, String> {
        Ok(match self.current() {
            Ok(State::Static(_, tr)) => tr,
            Ok(State::Transition(_, tr)) => tr,
            Err(e) => return Err(e),
        })
    }
}
