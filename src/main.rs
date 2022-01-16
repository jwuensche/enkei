use metadata::MetadataError;
use wayland_client::{Main, global_filter};


use std::{sync::{Arc, RwLock, mpsc::channel}, os::unix::prelude::MetadataExt};

use wayland_client::{
    protocol::{wl_compositor, wl_output},
    Display, GlobalManager,
};

use clap::ArgEnum;
use lazy_regex::regex_is_match;

mod outputs;
mod output;
mod schema;
mod opengl;
mod metadata;
mod messages;
mod watchdog;
mod worker;
mod image;

use crate::image::error::ImageError;

use thiserror::Error;

use khronos_egl as egl;
// global api object
use egl::API as egl;

use outputs::{
    Output,
    handle_output_events,
};

use crate::image::scaling::{
    Filter,
    Scaling,
};

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

use crate::metadata::{
    Metadata,
    MetadataReader,
};

fn main() -> Result<(), ApplicationError> {
    let args = Args::parse();

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
        // Let's use the global filter macro provided with the wayland-client crate here
        // The advantage of this that we will get all initially advertised objects (like WlOutput) as a freebe here and don't have to concern with getting
        // all available ones later.
        global_filter!(
            [wl_output::WlOutput, 2, move |output: Main<wl_output::WlOutput>, _: DispatchData| {
                println!("Got a new WlOutput instance!");
                let new_output = Arc::new(RwLock::new(Output::new(output.clone())));
                let pass = Arc::clone(&new_output);
                let added = tx.clone();
                output.quick_assign(move |_, event, _| {
                    handle_output_events(&pass, event, &added);
                });
                let mut lock = pass_outputs.write().unwrap();
                lock.push(new_output);
                drop(lock);
            }]
        )
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
            Some(Mode::Static) => {
                MetadataReader::static_configuration(&args.file)
            },
            Some(Mode::Dynamic) => MetadataReader::read(args.file)?,
            None => {
                if args.file.ends_with(".xml") {
                    MetadataReader::read(args.file)?
                } else if regex_is_match!(
                    r"\.(png|jpg|jpeg|gif|webp|farbfeld|tif|tiff|bmp|ico){1}$",
                    &args.file
                ) {
                    MetadataReader::static_configuration(&args.file)
                } else {
                    return Err(ApplicationError::InvalidDataType)
                }
            },
        }
    };

    worker::work(globals, display, message_rx, message_tx, event_queue, metadata)?;
    Ok(())
}
