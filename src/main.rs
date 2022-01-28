use error_report::ErrorReport;
use log::debug;
use metadata::MetadataError;
use wayland_client::{protocol::wl_registry::WlRegistry, Attached, GlobalEvent, Main};
use wayland_client::{ConnectError, GlobalError};

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{mpsc::channel, RwLock};

use wayland_client::{protocol::wl_output, Display, GlobalManager};

use clap::ArgEnum;
use lazy_regex::regex_is_match;

mod error_report;
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

use crate::image::error::ImageError;
use crate::metadata::MetadataReader;

use thiserror::Error;

use khronos_egl as egl;
// global api object
use egl::API as egl;

use outputs::{handle_output_events, Output};

use crate::image::scaling::{Filter, Scaling};
use khronos_egl::Error as EglError;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Image Processing failed: `{0}`")]
    ErrorWhileImageProcessing(ImageError),
    #[error("Reading of metadata failed: `{0}`")]
    MetadataError(MetadataError),
    #[error("Could not determine data type, try to specify via --mode. Or check given file")]
    InvalidDataType,
    #[error("Encountered an error while handling EGL - Location: `{0}` Cause: `{1}`")]
    EGL(String, EglError),
    #[error("EGL Setup failed: `{0}`")]
    EGLSetup(String),
    #[error("Could not lock data - Location: `{0}`")]
    LockedOut(String),
    #[error("Io Error: Location: `{0}` Cause: `{1}`")]
    Io(String, std::io::Error),
    #[error("Wayland Connection could not be established")]
    WaylandConnection(ConnectError),
    #[error("WaylandObject could not be initialized")]
    WaylandObject(GlobalError),
    #[error("Output Data was not ready, field value 'None' encountered")]
    OutputDataNotReady,
    #[error("The path `{0}` is not a file or does not exist")]
    NotAFile(PathBuf)

}

impl ApplicationError {
    fn locked_out(line: u32, file: &str) -> ApplicationError {
        ApplicationError::LockedOut(format!("{file}:{line}"))
    }

    fn io_error(e: std::io::Error, line: u32, file: &str) -> ApplicationError {
        ApplicationError::Io(format!("{file}:{line}"), e)
    }

    fn egl_error(e: crate::EglError, line: u32, file: &str) -> ApplicationError {
        ApplicationError::EGL(format!("{file}:{line}"), e)
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
    file: PathBuf,
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

const CB_ERR_MSG: &str = "WlOutput Handler panicked. Cannot continue.";

fn main() -> Result<(), ErrorReport> {
    let args = Args::parse();
    env_logger::init();
    /*
     * Setup display initials for wayland
     */
    let display = Display::connect_to_env().map_err(ApplicationError::WaylandConnection)?;
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let wl_outputs = Rc::new(RwLock::new(Vec::new()));
    let pass_outputs = Rc::clone(&wl_outputs);

    let (message_tx, message_rx) = channel();
    let tx = message_tx.clone();

    let globals = GlobalManager::new_with_cb(
        &attached_display,
        move |event: GlobalEvent, data: Attached<WlRegistry>, _| match event {
            GlobalEvent::New {
                id,
                interface,
                version,
            } if interface == "wl_output" => {
                debug!(
                    "Registering WlOutput Interface {{ id: {}, version: {} }}",
                    id, version
                );
                let output: Main<wl_output::WlOutput> = data.bind(version, id);
                let new_output = Rc::new(RwLock::new(Output::new(output.clone(), id)));
                let pass = Rc::clone(&new_output);
                let added = tx.clone();
                output.quick_assign(move |_, event, _| {
                    handle_output_events(&pass, event, &added, id);
                });
                let mut lock = pass_outputs.write().expect(CB_ERR_MSG);
                lock.push(new_output);
                drop(lock);
            }
            GlobalEvent::Removed { id, interface } if interface == "wl_output" => {
                debug!("Removing WlOutput Interface {{ id: {} }}", id);
                let mut lock = pass_outputs.write().expect(CB_ERR_MSG);
                let mut pos = lock.iter().enumerate().filter_map(|elem| {
                    let out = elem.1.read().expect(CB_ERR_MSG);
                    if out.id() == id {
                        Some(elem.0)
                    } else {
                        None
                    }
                });
                if let Some(valid) = pos.next() {
                    let _data = lock.swap_remove(valid);
                    tx.send(messages::WorkerMessage::RemoveOutput(id))
                        .expect(CB_ERR_MSG);
                }
            }
            _ => {}
        },
    );

    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .map_err(|e| ApplicationError::io_error(e, line!(), file!()))?;

    // Preliminary check for file existence for better errors
    if !args.file.is_file() {
        return Err(ApplicationError::NotAFile(args.file).into());
    }

    // Read Metadata or Prepare Static Mode
    let metadata = {
        match args.mode {
            Some(Mode::Static) => MetadataReader::static_configuration(&args.file),
            Some(Mode::Dynamic) => MetadataReader::read(args.file)?,
            None => {
                if args.file.ends_with(".xml") {
                    MetadataReader::read(args.file)?
                } else if regex_is_match!(
                    r"\.(?i)(png|jpg|jpeg|gif|webp|farbfeld|tif|tiff|bmp|ico){1}$",
                    args.file.to_str().expect("Could not deciper given path")
                ) {
                    MetadataReader::static_configuration(&args.file)
                } else {
                    return Err(ErrorReport::new(ApplicationError::InvalidDataType));
                }
            }
        }
    };

    let result = worker::work(
        globals,
        display,
        message_rx,
        message_tx,
        event_queue,
        metadata.clone(),
    );
    if let Err(e) = result {
        let report: ErrorReport = ErrorReport::from(e)
            .with_metadata(metadata)
            .with_outputs(wl_outputs);
        return Err(report);
    }
    Ok(())
}
