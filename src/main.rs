use error_report::ErrorReport;
use log::debug;
use metadata::MetadataError;
use wayland_client::{
    global_filter, protocol::wl_registry::WlRegistry, Attached, GlobalEvent, Main,
};

use std::{
    os::unix::prelude::MetadataExt,
    sync::{mpsc::channel, Arc, RwLock}, io::Read,
};

use wayland_client::{
    protocol::{wl_compositor, wl_output},
    Display, GlobalManager,
};

use clap::ArgEnum;
use lazy_regex::regex_is_match;

mod image;
mod messages;
mod metadata;
mod opengl;
mod output;
mod outputs;
mod schema;
mod util;
mod watchdog;
mod worker;
mod error_report;

use crate::image::error::ImageError;

use thiserror::Error;

use khronos_egl as egl;
// global api object
use egl::API as egl;

use outputs::{handle_output_events, Output};

use crate::image::scaling::{Filter, Scaling};

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Could not access the member `{0}` in some struct.")]
    AccessError(String),
    #[error("Image Processing failed: `{0}`")]
    ErrorWhileImageProcessing(ImageError),
    #[error("Reading of metadata failed: `{0}`")]
    MetadataError(MetadataError),
    #[error("Could not determine data type, try to specify via --mode. Or check given file")]
    InvalidDataType,
}

impl<'a> From<outputs::OutputError<'a>> for ApplicationError {
    fn from(err: outputs::OutputError<'a>) -> Self {
        match err {
            outputs::OutputError::KeyNotDefined(key) => Self::AccessError(key.into()),
        }
    }
}

impl From<image::error::ImageError> for ApplicationError {
    fn from(e: image::error::ImageError) -> Self {
        ApplicationError::ErrorWhileImageProcessing(e)
    }
}

impl From<MetadataError> for ApplicationError {
    fn from(e: MetadataError) -> Self {
        ApplicationError::MetadataError(e)
    }
}

const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const DESC: &str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const FILE_HELP: &str = "The path to the wallpaper to be shown. The mode, static or dynamic, gets determined automatically by default, based on the file suffix.";
const MODE_HELP: &str = "The display mode, static or dynamic, to be used for the given wallpaper. Normally this gets detected automatically based on the file suffix. If this is not possible set it explicitly here.";
const SCALE_HELP: &str = "The scaling mode, which should be used to fit the image to the screen. Fit will try to fit the whole image to the screen, while Fill will try to fill the screen completely upscaling and cropping the image if necessary.";
const FILTER_HELP: &str = "The filter method which should be applied when a wallpaper is scaled. Variants correspond to cairo filters.";

use clap::Parser;

#[derive(clap::Parser, Debug)]
#[clap(
    name = NAME,
    author = AUTHOR,
    version = VERSION,
    about = DESC,
)]
struct Args {
    #[clap(
        index = 1,
        help = "The file to display.",
        long_help = FILE_HELP,
        takes_value = true,
        required = true,
    )]
    file: String,
    #[clap(
        arg_enum,
        short = 'f',
        long = "filter",
        help = "How to filter scaled images.",
        long_help = FILTER_HELP,
        default_value = "good",
        takes_value = true,
        ignore_case = true,
    )]
    filter: Filter,
    #[clap(
        arg_enum,
        short = 's',
        long = "scale",
        help = "How to scale or crop images.",
        long_help = SCALE_HELP,
        default_value = "fill",
        takes_value = true,
        ignore_case = true,
    )]
    scale: Scaling,
    #[clap(
        arg_enum,
        short = 'm',
        long = "mode",
        help = "The display mode which should be used for the given file.",
        long_help = MODE_HELP,
        takes_value = true,
        ignore_case = true,
    )]
    mode: Option<Mode>,
}

#[derive(ArgEnum, Clone, Debug)]
pub enum Mode {
    Static,
    Dynamic,
}

use crate::metadata::{Metadata, MetadataReader};

fn main() {
    let args = Args::parse();
    env_logger::init();
    /*
     * Setup display initials for wayland
     */
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let wl_outputs = Arc::new(RwLock::new(Vec::new()));
    let pass_outputs = Arc::clone(&wl_outputs);

    let (message_tx, message_rx) = channel();
    let tx = message_tx.clone();

    let globals = GlobalManager::new_with_cb(
        &attached_display,
        move |event: GlobalEvent, data: Attached<WlRegistry>, _| {
            match event {
                GlobalEvent::New {
                    id,
                    interface,
                    version,
                } if interface == "wl_output" => {
                    debug!("Registering WlOutput Interface {{ id: {}, version: {} }}", id, version);
                    let output: Main<wl_output::WlOutput> = data.bind(version, id);
                    let new_output = Arc::new(RwLock::new(Output::new(output.clone(), id)));
                    let pass = Arc::clone(&new_output);
                    let added = tx.clone();
                    output.quick_assign(move |_, event, _| {
                        handle_output_events(&pass, event, &added, id);
                    });
                    let mut lock = pass_outputs.write().unwrap();
                    lock.push(new_output);
                    drop(lock);
                }
                GlobalEvent::Removed { id, interface } if interface == "wl_output" => {
                    debug!("Removing WlOutput Interface {{ id: {} }}", id);
                    let mut lock = pass_outputs.write().unwrap();
                    let mut pos = lock.iter().enumerate().filter_map(|elem| {
                        let out = elem.1.read().unwrap();
                        if out.id() == id {
                            Some(elem.0)
                        } else {
                            None
                        }
                    });
                    if let Some(valid) = pos.next() {
                        let _data = lock.swap_remove(valid);
                        tx.send(messages::WorkerMessage::RemoveOutput(id)).unwrap();
                    }
                }
                _ => {}
            }
        },
    );

    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    /*
     * Initialize Watchdogs for Suspension Cycles
     */
    watchdog::sleeping::initialize(message_tx.clone());

    /*
     * Read Metadata or Prepare Static Mode
     */
    let metadata = {
        match args.mode {
            Some(Mode::Static) => Ok(MetadataReader::static_configuration(&args.file)),
            Some(Mode::Dynamic) => MetadataReader::read(args.file),
            None => {
                if args.file.ends_with(".xml") {
                    MetadataReader::read(args.file)
                } else if regex_is_match!(
                    r"\.(png|jpg|jpeg|gif|webp|farbfeld|tif|tiff|bmp|ico){1}$",
                    &args.file
                ) {
                    Ok(MetadataReader::static_configuration(&args.file))
                } else {
                    let error = ErrorReport::new(ApplicationError::InvalidDataType);
                    error.report();
                    std::process::exit(1);
                }
            }
        }
    };

    if let Ok(meta) = metadata {
        let result = worker::work(
            globals,
            display,
            message_rx,
            message_tx,
            event_queue,
            meta.clone(),
        );
        if let Err(e) = result {
            let report: ErrorReport = e.into();
            report.with_metadata(meta).with_outputs(wl_outputs).report();
            std::process::exit(1);
        }
    } else {
        let report: ErrorReport = metadata.unwrap_err().into();
        report.report();
        std::process::exit(1);
    }
    std::process::exit(0);
}
