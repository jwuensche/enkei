extern crate gtk_layer_shell as gls;

use cairo::ImageSurface;
use clap::{arg_enum, value_t, App, Arg};
use gdk::Rectangle;
use lazy_regex::regex_is_match;
use metadata::{Metadata, MetadataReader};

mod bg_manager;
mod metadata;
mod schema;

use bg_manager::BackgroundManager;
impl Scaling {
    fn scale(
        &self,
        sur: &ImageSurface,
        geometry: &Rectangle,
        filter: Filter,
    ) -> Result<ImageSurface, String> {
        match self {
            Scaling::Fill => Scaling::fill(sur, geometry, filter),
            Scaling::Fit => Scaling::fit(sur, geometry, filter),
            Scaling::None => Scaling::none(sur, geometry),
        }
    }

    fn none(buf: &ImageSurface, geometry: &Rectangle) -> Result<ImageSurface, String> {
        let pad_width = (geometry.width - buf.width()) as f64 / 2.0;
        let pad_height = (geometry.height - buf.height()) as f64 / 2.0;

        {
            let target =
                cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height)
                    .map_err(|e| format!("Surface creation failed: {}", e))?;
            let ctx = cairo::Context::new(&target).map_err(|e| format!("Could not create surface: {:?}", e))?;
            ctx.set_source_surface(buf, pad_width, pad_height).map_err(|e| format!("Could not set source surface: {:?}", e))?;
            ctx.paint().map_err(|e| format!("Could not paint context: {:?}", e))?;

            Ok(target)
        }
    }

    fn fit(
        buf: &ImageSurface,
        geometry: &Rectangle,
        filter: Filter,
    ) -> Result<ImageSurface, String> {
        Scaling::fill_or_fit(buf, geometry, filter, f64::min)
    }

    fn fill(
        buf: &ImageSurface,
        geometry: &Rectangle,
        filter: Filter,
    ) -> Result<ImageSurface, String> {
        Scaling::fill_or_fit(buf, geometry, filter, f64::max)
    }

    fn fill_or_fit<F: Fn(f64, f64) -> f64>(
        buf: &ImageSurface,
        geometry: &Rectangle,
        filter: Filter,
        comp: F,
    ) -> Result<ImageSurface, String> {
        // 1. Crop the image if necessary
        // 2. Scale the image to the proper size

        let height_ratio = geometry.height as f64 / buf.height() as f64;
        let width_ratio = geometry.width as f64 / buf.width() as f64;
        let max_ratio = comp(height_ratio, width_ratio);

        // Get cropping edges (aspect)
        let crop_height = ((buf.height() as f64 * max_ratio) as i32)
            .checked_sub(geometry.height)
            .map(|elem| (elem / 2) as f64 / max_ratio)
            .unwrap_or(0.0)
            .clamp(-geometry.height as f64, geometry.height as f64);
        let crop_width = ((buf.width() as f64 * max_ratio) as i32)
            .checked_sub(geometry.width)
            .map(|elem| (elem / 2) as f64 / max_ratio)
            .unwrap_or(0.0)
            .clamp(-geometry.width as f64, geometry.width as f64);
        // Create context and scale and crop to fit
        {
            let target =
                cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height)
                    .map_err(|e| format!("Encountered error while creating Surface: {}", e))?;
            let ctx = cairo::Context::new(&target).map_err(|e| format!("Could not create surface: {:?}", e))?;
            ctx.scale(max_ratio, max_ratio);
            ctx.set_source_surface(buf, -crop_width, -crop_height).map_err(|e| format!("Could not set source surface: {:?}", e))?;
            ctx.source().set_filter(filter.into());
            ctx.paint().map_err(|e| format!("Could not paint context: {:?}", e))?;

            Ok(target)
        }
    }
}

arg_enum! {
    #[derive(PartialEq, Debug, Clone)]
    pub enum Scaling {
        Fill,
        Fit,
        None,
    }
}

arg_enum! {
    #[derive(PartialEq, Debug)]
    pub enum Mode {
        Static,
        Dynamic,
    }
}

arg_enum! {
    #[derive(PartialEq, Debug, Clone, Copy)]
    pub enum Filter {
        Fast,
        Good,
        Best,
        Nearest,
        Bilinear,
        Gaussian,
    }
}

impl From<Filter> for cairo::Filter {
    fn from(filter: Filter) -> Self {
        match filter {
            Filter::Fast => cairo::Filter::Fast,
            Filter::Good => cairo::Filter::Good,
            Filter::Best => cairo::Filter::Best,
            Filter::Nearest => cairo::Filter::Nearest,
            Filter::Bilinear => cairo::Filter::Bilinear,
            Filter::Gaussian => cairo::Filter::Gaussian,
        }
    }
    // fn into(self) -> cairo::Filter {
    // }
}

const FILE: &str = "FILE";
const MODE: &str = "MODE";
const SCALE: &str = "SCALE";
const FILTER: &str = "FILTER";

const NAME: &str = env!("CARGO_PKG_NAME");
const IDENT: &str = "rocks.spacesnek.enkei";
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const DESC: &str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const FILE_HELP: &str = "The path to the wallpaper to be shown. The mode, static or dynamic gets determined automatically by default, based on the file suffix.";
const MODE_HELP: &str = "The display mode, static or dynamic, to be used for the given wallpaper. Normally this gets detected automatically based on the file suffix. If this is not possible set it explicitly here.";
const SCALE_HELP: &str = "The scaling mode, which should be used to fit the image to the screen. Fit will try to fit the whole image to the screen, while Fill will try to fill the screen completely upscaling and cropping the image if necessary.";
const FILTER_HELP: &str = "The filter method which should be applied when a wallpaper is scaled. Variants correspond to cairo filters.";

fn main() {
    env_logger::init();
    let matches = App::new(NAME)
        .about(DESC)
        .version(VERSION)
        .author(AUTHOR)
        .arg(
            Arg::with_name(FILE)
                .help("The file to display.")
                .long_help(FILE_HELP)
                .index(1)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(MODE)
                .help("The display mode which should be used for the given file.")
                .long_help(MODE_HELP)
                .takes_value(true)
                .possible_values(&Mode::variants())
                .case_insensitive(true)
                .short("m")
                .long("mode"),
        )
        .arg(
            Arg::with_name(SCALE)
                .help("How to scale or crop images.")
                .long_help(SCALE_HELP)
                .takes_value(true)
                .possible_values(&Scaling::variants())
                .case_insensitive(true)
                .default_value("fill")
                .short("s")
                .long("scale"),
        )
        .arg(
            Arg::with_name(FILTER)
                .help("How to filter scaled images.")
                .long_help(FILTER_HELP)
                .takes_value(true)
                .short("f")
                .long("filter")
                .default_value("good")
                .possible_values(&Filter::variants())
                .case_insensitive(true),
        )
        .get_matches();
    let image = matches.value_of(FILE).expect("No FILE given");

    let scaling: Scaling = value_t!(matches, SCALE, Scaling)
        .expect("Something went wrong decoding the given scale mode.");
    let filter: Filter =
        value_t!(matches, FILTER, Filter).expect("Something went wrong decoding the given filter.");
    let mode: Option<Mode> = value_t!(matches, MODE, Mode).ok();

    if let Err(e) = std::fs::OpenOptions::new().read(true).open(image) {
        eprintln!("Could not open file {}: {}", image, e);
        std::process::exit(1);
    }

    let config;
    if let Some(chosen_mode) = mode {
        match chosen_mode {
            Mode::Dynamic => config = load_dynamic(image),
            Mode::Static => config = load_static(image),
        }
    } else {
        match detect_wp_type(image) {
            Some(Mode::Static) => config = load_static(image),
            Some(Mode::Dynamic) => config = load_dynamic(image),
            None => {
                eprintln!("Could not determine wallpaper type, please specify dynamic or static.");
                std::process::exit(1);
            }
        }
    }

    match BackgroundManager::new(config, filter, scaling) {
        Ok(bm) => bm.run(),
        Err(e) => {
            eprintln!(
                "Creation of Background Manager failed. Due to Reason: {}",
                e
            )
        }
    }
}

fn load_dynamic(image: &str) -> Metadata {
    let res = MetadataReader::read(image);
    if let Ok(conf) = res {
        conf
    } else {
        eprintln!("Could not load {} as dynamic background: {:?}", image, res);
        std::process::exit(1);
    }
}

fn load_static(image: &str) -> Metadata {
    MetadataReader::stat(image)
}

// Only detect based on their suffix for now, if any changes are made including
// which files to accept we might need to expand this enum here
fn detect_wp_type(image: &str) -> Option<Mode> {
    if image.ends_with(".xml") {
        return Some(Mode::Dynamic);
    }
    if regex_is_match!(
        r"\.(png|jpg|jpeg|gif|webp|farbfeld|tif|tiff|bmp|ico){1}$",
        image
    ) {
        return Some(Mode::Static);
    }
    None
}
