use clap::ArgEnum;
use std::{io::Write, os::unix::net::UnixStream, path::PathBuf};

#[derive(Clone, Debug, ArgEnum, Serialize, Deserialize)]
enum Scaling {
    Fill,
    Fit,
    None,
}

#[derive(Clone, Debug, ArgEnum, Serialize, Deserialize)]
enum Filter {
    Fast,
    Good,
    Best,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    filter: Option<Filter>,
    scaling: Option<Scaling>,
    path: PathBuf,
    mode: Option<Mode>,
}
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;

const NAME: &str = "enkeictl";
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const DESC: &str = "Control application for enkei.";
const VERSION: &str = env!("CARGO_PKG_VERSION");

const FILE_HELP: &str = "The path to the wallpaper to be shown. The mode, static or dynamic, gets determined automatically by default, based on the file suffix.";
const MODE_HELP: &str = "The display mode, static or dynamic, to be used for the given wallpaper. Normally this gets detected automatically based on the file suffix. If this is not possible set it explicitly here.";
const SCALE_HELP: &str = "The scaling mode, which should be used to fit the image to the screen. Fit will try to fit the whole image to the screen, while Fill will try to fill the screen completely upscaling and cropping the image if necessary.";
const FILTER_HELP: &str = "The filter method which should be applied when a wallpaper is scaled. Variants correspond to cairo filters.";

#[derive(clap::Parser, Debug)]
#[clap(
    name = NAME,
    author = AUTHOR,
    version = VERSION,
    about = DESC,
)]
pub struct Args {
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
        takes_value = true,
        ignore_case = true,
    )]
    filter: Option<Filter>,
    #[clap(
        arg_enum,
        short = 's',
        long = "scale",
        help = "How to scale or crop images.",
        long_help = SCALE_HELP,
        takes_value = true,
        ignore_case = true,
    )]
    scale: Option<Scaling>,
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

#[derive(ArgEnum, Clone, Debug, Serialize, Deserialize)]
pub enum Mode {
    Static,
    Dynamic,
}

fn main() {
    let args = Args::parse();

    let msg = Message {
        filter: args.filter,
        scaling: args.scale,
        path: args.file,
        mode: args.mode,
    };
    if write(msg).is_err() {
        eprintln!("Could not connect to enkei. Please make sure that $XDG_RUNTIME_DIR is set and enkei is running.");
        std::process::exit(1);
    }
}

fn write(msg: Message) -> Result<(), ()> {
    let runtime_dir: PathBuf = std::env::var("XDG_RUNTIME_DIR").map_err(|_| ())?.into();
    let socket_path = runtime_dir.join("enkei-ipc.sock");
    let mut socket = UnixStream::connect(&socket_path).map_err(|_| ())?;
    socket
        .write(&bincode::serialize(&msg).expect("Could not serialize message"))
        .map_err(|_| ())?;
    Ok(())
}
