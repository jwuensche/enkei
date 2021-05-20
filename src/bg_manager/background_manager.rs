
extern crate gtk_layer_shell as gls;

use gio::prelude::*;
use cairo::{Context, ImageSurface};
use gdk::{Display, Monitor};
use gtk::{prelude::*, Image};
use crate::metadata;
use crate::metadata::Metadata;
use gio::ApplicationFlags;
use crate::{Scaling, Filter};
use super::TransitionState;
use super::main_tick;
use crate::NAME;

// Managment structure holding all the windows rendering the
// separate windows. For each display on window is created
#[derive(Clone, Debug)]
pub struct OutputState {
    pub monitor: Monitor,
    pub image_from: ImageSurface,
    pub image_to: ImageSurface,
    pub ctx: Context,
    pub duration_in_sec: u64,
    pub time: std::time::Instant,
    pub pic: Image,
}

#[derive(Clone)]
pub struct BackgroundManager {
    pub monitors: Vec<OutputState>,
    pub config: Metadata,
    pub app: gtk::Application,
    pub scaling: Scaling,
    pub filter: Filter,
}

impl BackgroundManager {
    pub fn new(config: Metadata, filter: Filter, scaling: Scaling) -> Result<Self, String> {
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
            filter,
            scaling
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
            output.image_from = self.scaling.scale(&first, &output.monitor.get_geometry(), self.filter);
            output.image_to = self.scaling.scale(&second, &output.monitor.get_geometry(), self.filter);
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
