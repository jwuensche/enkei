extern crate gtk_layer_shell as gls;

use std::path::Path;

use super::main_tick;
use super::TransitionState;
use crate::metadata;
use crate::metadata::Metadata;
use crate::NAME;
use crate::{Filter, Scaling};
use cairo::{Context, ImageSurface};
use gdk::{Display, Monitor};
use gio::prelude::*;
use gio::ApplicationFlags;
use gtk::{prelude::*, Image};
use image::io::Reader as ImageReader;
use image::DynamicImage;

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
            scaling,
            filter,
        };
        bm.init_and_load()?;
        Ok(bm)
    }

    fn create_surface_from_path<P: AsRef<Path>>(path: P) -> Result<ImageSurface, String> {
        let img = ImageReader::open(path)
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;

        BackgroundManager::create_surface_with_alpha(img)
    }

    fn create_surface_with_alpha(img: DynamicImage) -> Result<ImageSurface, String> {
        // This is in reverse to what we actually would need but is correct for
        // how cairo reads the received buffers. Why is this the case? The
        // documentation states that the "upper" part of Argb is the alpha value
        // followed by red, green, and blue. Either their "upper" is the end of
        // the buffer or similar mix-ups happen in between.
        let buf = img.to_bgra8();
        let stride = cairo::Format::ARgb32.stride_for_width(buf.width()).map_err(|_| "Stride calculation failed.")?;
        // Meh more clone
        let pxls = buf.as_raw().clone();

        cairo::ImageSurface::create_for_data(
            pxls,
            cairo::Format::ARgb32,
            buf.width() as i32,
            buf.height() as i32,
            stride
        ).map_err(|_| "Could not create Surface from Image data".into())
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

        let first = BackgroundManager::create_surface_from_path(transition.from)?;
        let second = BackgroundManager::create_surface_from_path(transition.to)?;
        for output in self.monitors.iter_mut() {
            output.duration_in_sec = transition.duration_transition as u64;
            output.image_from =
                self.scaling
                    .scale(&first, &output.monitor.get_geometry(), self.filter);
            output.image_to =
                self.scaling
                    .scale(&second, &output.monitor.get_geometry(), self.filter);
            let geometry = output.monitor.get_geometry();
            let sur = cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height).unwrap();
            let ctx = Context::new(&sur);
            ctx.set_source_surface(&output.image_from, 0.0, 0.0);
            ctx.paint();
            ctx.set_source_surface(&output.image_to, 0.0, 0.0);
            ctx.paint_with_alpha(progress as f64 / transition.duration_transition as f64);
            output.time =
                std::time::Instant::now() - std::time::Duration::from_secs(progress as u64);
            output.pic.set_from_surface(Some(&sur))
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

        self.app.run(&[NAME.to_string()]);
    }
}
