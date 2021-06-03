use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Background {
    #[serde(rename = "$value")]
    pub images: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub enum Image {
    #[serde(rename = "starttime")]
    StartTime {
        year: u32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    },
    #[serde(rename = "static")]
    Static { duration: f64, file: String },
    #[serde(rename = "transition")]
    Transition {
        #[serde(rename = "type")]
        kind: String,
        duration: f64,
        from: String,
        to: String,
    },
}
