extern crate gtk_layer_shell as gls;

use cairo::{Context, ImageSurface};
use gdk::{Display, Monitor, Rectangle};
use gdk_pixbuf::{InterpType, Pixbuf};
use gio::prelude::*;
use glib::{self, timeout_add_local};
use gtk::{prelude::*, Image};
use metadata::{Metadata, MetadataReader};
use timesync::main_tick;
use clap::{App, Arg, arg_enum, value_t};
use gio::ApplicationFlags;

use std::{env::args, fs::OpenOptions};

mod image;
mod metadata;
mod schema;
mod timesync;

use crate::timesync::TransitionState;

// Managment structure holding all the windows rendering the
// separate windows. For each display on window is created
#[derive(Clone, Debug)]
pub struct OutputState {
    monitor: Monitor,
    image_from: ImageSurface,
    image_to: ImageSurface,
    ctx: Context,
    duration_in_sec: u64,
    time: std::time::Instant,
    pic: Image,
}

#[derive(Clone)]
pub struct BackgroundManager {
    monitors: Vec<OutputState>,
    config: Metadata,
    app: gtk::Application,
}



pub fn scale_or_crop(buf: &ImageSurface, geometry: &Rectangle) -> ImageSurface {
    // 1. Crop the image if necessary
    // 2. Scale the image to the proper size

    let height_ratio = geometry.height as f64 / buf.get_height() as f64;
    let width_ratio = geometry.width as f64 / buf.get_width() as f64;
    let max_ratio = height_ratio.max(width_ratio);

    // Get cropping edges (aspect)
    let crop_height = ((buf.get_height() as f64 * max_ratio) as i32)
        .checked_sub(geometry.height)
        .map(|elem| (elem / 2) as f64 / max_ratio)
        .unwrap_or(0.0)
        .clamp(0.0, geometry.height as f64);
    let crop_width = ((buf.get_width() as f64 * max_ratio) as i32)
        .checked_sub(geometry.width)
        .map(|elem| (elem / 2) as f64 / max_ratio)
        .unwrap_or(0.0)
        .clamp(0.0, geometry.width as f64);

    // Create context and scale and crop to fit
    let target = {
        let target =
            cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height)
                .unwrap();
        let ctx = cairo::Context::new(&target);
        ctx.scale(max_ratio, max_ratio);
        ctx.set_source_surface(buf, -(crop_width as f64), -(crop_height));
        ctx.get_source().set_filter(cairo::Filter::Good);
        ctx.paint();

        target
    };

    return target;
}

impl BackgroundManager {
    pub fn new(config: Metadata) -> Result<Self, String> {
        let mut monitors = vec![];
        // initialize gdk to find attached monitors at this stage is already
        gdk::init();
        gtk::init().unwrap();

        let display = Display::get_default().expect("Could not get display");
        for mon_id in 0..display.get_n_monitors() {
            if let Some(monitor) = display.get_monitor(mon_id) {
                let img = ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
                let ctx = Context::new(&img);
                monitors.push(OutputState {
                    image_from: img.clone(),
                    image_to: img,
                    ctx,
                    monitor,
                    duration_in_sec: 5,
                    time: std::time::Instant::now(),
                    pic: gtk::Image::new(),
                });
            }
        }

        let flags: ApplicationFlags = Default::default();
        let app = gtk::Application::new(Some("com.gtk-layer-example"), flags)
            .expect("Initialization failed...");

        let mut bm = Self {
            monitors,
            config,
            app,
        };
        bm.init_and_load()?;
        Ok(bm)
    }

    pub fn init_and_load(&mut self) -> Result<(), String> {
        let transition;
        let progress;
        match self.config.current()? {
            metadata::State::Static(_, tr) => {
                transition = tr;
                progress = 0;
            }
            metadata::State::Transition(p, tr) => {
                transition = tr;
                progress = p;
            }
        }


        let first = {
            let mut image_file = std::fs::OpenOptions::new()
                .read(true)
                .open(transition.from)
                .map_err(|_| "Could not open file specified in dynamic transition.")?;
            cairo::ImageSurface::create_from_png(&mut image_file).unwrap()
        };
        let second = {
            let mut image_file = std::fs::OpenOptions::new()
                .read(true)
                .open(transition.to)
                .map_err(|_| "Could not open file specified in dynamic transition.")?;
            cairo::ImageSurface::create_from_png(&mut image_file).unwrap()
        };

        for output in self.monitors.iter_mut() {
            output.duration_in_sec = transition.duration_transition as u64;
            output.image_from = scale_or_crop(&first, &output.monitor.get_geometry());
            output.image_to = scale_or_crop(&second, &output.monitor.get_geometry());
            let ctx = Context::new(&output.image_from);
            ctx.set_source_surface(&output.image_to, 0.0, 0.0);
            ctx.paint_with_alpha(progress as f64 / transition.duration_transition as f64);
            output.time =
                std::time::Instant::now() - std::time::Duration::from_secs(progress as u64);
            output.pic.set_from_surface(Some(&output.image_from))
        }

        Ok(())
    }

    pub fn run(self) {
        let monitors = self.monitors.clone();

        self.app.connect_activate(move |app| {
            for monitor in monitors.iter() {
                let window = gtk::ApplicationWindow::new(app);

                gls::init_for_window(&window);
                // Push other windows out of the way
                gls::set_exclusive_zone(&window, -1);
                // Anchors are if the window is pinned to each edge of the output
                gls::set_margin(&window, gls::Edge::Left, 0);
                gls::set_margin(&window, gls::Edge::Right, 0);
                gls::set_margin(&window, gls::Edge::Top, 0);
                gls::set_margin(&window, gls::Edge::Bottom, 0);
                gls::set_layer(&window, gls::Layer::Background);
                gls::set_monitor(&window, &monitor.monitor);

                // Set up a widget
                monitor.pic.set_from_surface(Some(&monitor.image_from));
                window.add(&monitor.pic);
                window.show_all();
            }
        });

        let origin = self.clone();
        main_tick(origin, TransitionState::Start);

        self.app.run(&vec!(NAME.to_string()));
    }
}

arg_enum!{
    #[derive(PartialEq, Debug)]
    pub enum Scaling {
        Fill,
        Fit,
        None,
    }
}

arg_enum!{
    #[derive(PartialEq, Debug)]
    pub enum Mode {
        Static,
        Dynamic,
    }
}

arg_enum!{
    #[derive(PartialEq, Debug)]
    pub enum Filter {
        Fast,
        Good,
        Best,
        Nearest,
        Bilinear,
        Gaussian,
    }
}

const FILE: &str = "FILE";
const MODE: &str = "MODE";
const SCALE: &str = "SCALE";
const FILTER: &str = "FILTER";

const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const DESC: &str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const FILE_HELP: &str = "The path to the wallpaper to be shown. The mode, static or dynamic gets determined automatically by default, based on the file suffix.";
const MODE_HELP: &str = "The display mode, static or dynamic, to be used for the given wallpaper. Normally this gets detected automatically based on the file suffix. If this is not possible set it explicitly here.";
const SCALE_HELP: &str = "The scaling mode, which should be used to fit the image to the screen. Fit will try to fit the whole image to the screen, while Fill will try to fill the screen completely upscaling and cropping the image if necessary.";
const FILTER_HELP: &str = "The filter method which should be applied when a wallpaper is scaled. Varitants correspond to cairo filters.";

fn main() {
    let matches = App::new(NAME)
        .about(DESC)
        .version(VERSION)
        .author(AUTHOR)
        .arg(Arg::with_name(FILE)
             .help("The file to display.")
             .long_help(FILE_HELP)
             .index(1)
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name(MODE)
             .help("The display mode which should be used for the given file.")
             .long_help(MODE_HELP)
             .takes_value(true)
             .possible_values(&Mode::variants())
             .case_insensitive(true)
             .short("m")
             .long("mode"))
        .arg(Arg::with_name(SCALE)
             .help("How to scale or crop images.")
             .long_help(SCALE_HELP)
             .takes_value(true)
             .possible_values(&Scaling::variants())
             .case_insensitive(true)
             .short("s")
             .long("scale"))
        .arg(Arg::with_name(FILTER)
             .help("How to filter scaled images.")
             .long_help(FILTER_HELP)
             .takes_value(true)
             .short("f")
             .long("filter")
             .possible_values(&Filter::variants())
             .case_insensitive(true))
        .get_matches();

    let image = matches.value_of(FILE).expect("No FILE given");

    let scaling: Option<Scaling> = value_t!(matches, SCALE, Scaling).ok();
    let filter: Option<Filter> = value_t!(matches, FILTER, Filter).ok();
    let mode: Option<Mode> =  value_t!(matches, MODE, Mode).ok();

    let config;
    if let Some(chosen_mode) = mode {
        match chosen_mode {
            Mode::Dynamic => {
                config = load_dynamic(image)
            }
            Mode::Static => {
                config = load_static(image)
            }
        }
    } else {
        match detect_wp_type(image) {
            Some(Mode::Static) => {
                config = load_static(image)
            }
            Some(Mode::Dynamic) => {
                config = load_dynamic(image)
            }
            None => {
                eprintln!("Could not determine wallpaper type, please specify dynamic or static.");
                std::process::exit(1);
            }
        }
    }


    // dbg!(config.current_transition());

    if let Ok(bm) = BackgroundManager::new(config) {
        bm.run()
    } else {
        eprintln!("Could not load config.")
    }
}

use anyhow::Result;

fn load_dynamic(image: &str) -> Metadata {
    let res = MetadataReader::read(image);
    if let Ok(conf) = res {
        return conf
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
        return Some(Mode::Dynamic)
    }
    if image.ends_with(".png") {
        return Some(Mode::Static)
    }
    None
}
