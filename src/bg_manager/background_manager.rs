extern crate gtk_layer_shell as gls;

use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use super::main_tick;
use super::TransitionState;
use crate::metadata;
use crate::metadata::Metadata;
use crate::metadata::Transition;
use crate::{Filter, Scaling};
use crate::{IDENT, NAME};
use cairo::{Context, ImageSurface};
use gdk::{Display, Monitor};
use gio::prelude::*;
use gio::ApplicationFlags;
use glib::timeout_add_local;
use gtk::{prelude::*, Image};
use image::io::Reader as ImageReader;
use image::DynamicImage;
use log::debug;

const WAITING_INTERVAL: u64 = 16;

// Managment structure holding all the windows rendering the
// separate windows. For each display on window is created
#[derive(Clone, Debug)]
pub struct OutputState {
    pub monitor: Monitor,
    pub image_from: ImageSurface,
    pub image_to: Option<ImageSurface>,
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
        gtk::init().map_err(|e| format!("Failed to initialize gtk: {}", e))?;

        let display =
            Display::default().ok_or_else(|| "Could not get default display.".to_string())?;
        for mon_id in 0..display.n_monitors() {
            if let Some(monitor) = display.monitor(mon_id) {
                let img = ImageSurface::create(cairo::Format::ARgb32, 1, 1)
                    .map_err(|e| format!("Surface creation failed: {}", e))?;
                monitors.push(OutputState {
                    image_from: img.clone(),
                    image_to: Some(img),
                    monitor,
                    duration_in_sec: 5,
                    time: std::time::Instant::now(),
                    pic: gtk::Image::new(),
                });
            }
        }

        let flags: ApplicationFlags = Default::default();
        let app =
            gtk::Application::new(Some(IDENT), flags);

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
        let stride = cairo::Format::ARgb32
            .stride_for_width(buf.width())
            .map_err(|_| "Stride calculation failed.")?;
        // Meh more clone
        let pxls = buf.as_raw().clone();

        cairo::ImageSurface::create_for_data(
            pxls,
            cairo::Format::ARgb32,
            buf.width() as i32,
            buf.height() as i32,
            stride,
        )
        .map_err(|_| "Could not create Surface from Image data".into())
    }

    fn load_images_and_state(
        &self,
    ) -> Result<(ImageSurface, Option<ImageSurface>, Transition, f64), String> {
        let transition;
        let progress;
        match self.config.current()? {
            metadata::State::Static(_, tr) => {
                transition = tr;
                progress = 0f64;
            }
            metadata::State::Transition(p, tr) => {
                transition = tr;
                progress = p;
            }
        }

        let first = BackgroundManager::create_surface_from_path(transition.from())?;

        let second;
        if let Some(image_path) = transition.to() {
            second = Some(BackgroundManager::create_surface_from_path(image_path)?);
        } else {
            second = None;
        }
        Ok((first, second, transition, progress))
    }

    fn init_monitor(
        scaling: &Scaling,
        filter: &Filter,
        output: &mut OutputState,
        first: &ImageSurface,
        second: &Option<ImageSurface>,
        transition: &Transition,
        progress: f64,
    ) -> Result<(), String> {
        output.duration_in_sec = transition.duration_transition() as u64;
        output.image_from = scaling.scale(&first, &output.monitor.geometry(), *filter)?;
        if let Some(image_to) = &second {
            output.image_to =
                Some(scaling.scale(&image_to, &output.monitor.geometry(), *filter)?);
        } else {
            output.image_to = None;
        }
        let geometry = output.monitor.geometry();
        let sur =
            cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height)
                .map_err(|e| format!("Surface creation failed: {}", e))?;
        let ctx = Context::new(&sur).map_err(|e| format!("Could not create surface: {:?}", e))?;
        ctx.set_source_surface(&output.image_from, 0.0, 0.0).map_err(|e| format!("Could not set source surface: {:?}", e))?;
        ctx.paint().map_err(|e| format!("Could not paint context: {:?}", e))?;
        if let Some(image_to) = &output.image_to {
            ctx.set_source_surface(&image_to, 0.0, 0.0).map_err(|e| format!("Could not set source surface: {:?}", e))?;
            ctx.paint_with_alpha(progress as f64 / transition.duration_transition() as f64).map_err(|e| format!("Could not paint context: {:?}", e))?;
        }
        output.time = std::time::Instant::now() - std::time::Duration::from_secs(progress as u64);
        output.pic.set_from_surface(Some(&sur));
        Ok(())
    }

    pub fn init_and_load(&mut self) -> Result<(), String> {
        let (first, second, transition, progress) = self.load_images_and_state()?;
        for output in self.monitors.iter_mut() {
            BackgroundManager::init_monitor(
                &self.scaling,
                &self.filter,
                output,
                &first,
                &second,
                &transition,
                progress,
            )?;
        }

        Ok(())
    }

    pub fn run(self) {
        let monitors = self.monitors.clone();

        self.app.connect_activate(move |app| {
            for monitor in monitors.iter() {
                add_image_window_to_monitor(&monitor.pic, app, &monitor.monitor);
            }
        });

        let origin = Rc::new(Mutex::new(self.clone()));
        main_tick(origin.clone(), TransitionState::Start);

        if let Some(display) = Display::default() {
            let remover = origin.clone();
            display.connect_monitor_removed(move |_, mon| {
                debug!("Monitor Removed!");
                let mut lock = remover.lock().unwrap();
                lock.monitors.retain(|elem| elem.monitor != *mon);
            });
            let adder = origin;
            display.connect_monitor_added(move |_, mon| {
                debug!("New Monitor detected {:?}.", mon);
                let mo = mon.clone();
                let adder = adder.clone();
                timeout_add_local(std::time::Duration::from_millis(WAITING_INTERVAL), move || {
                    if mo.geometry().height < 1 || mo.geometry().width < 1 {
                        return glib::Continue(true)
                    }
                    let mut lock = adder.lock().unwrap();

                    let img = ImageSurface::create(cairo::Format::ARgb32, 1, 1)
                        .map_err(|e| format!("Surface creation failed: {}", e))
                        .unwrap();
                    lock.monitors.push(OutputState {
                        image_from: img.clone(),
                        image_to: Some(img),
                        monitor: mo.clone(),
                        duration_in_sec: 5,
                        time: std::time::Instant::now(),
                        pic: gtk::Image::new(),
                    });

                    let (first, second, transition, progress) = lock.load_images_and_state().unwrap();
                    let s = lock.scaling.clone();
                    let f = lock.filter;
                    let out = lock.monitors.last_mut().unwrap();
                    debug!("Initiating new monitor {:?}.", out);
                    BackgroundManager::init_monitor(
                        &s,
                        &f,
                        out,
                        &first,
                        &second,
                        &transition,
                        progress,
                    )
                    .unwrap();

                    let pic = &lock.monitors.last().unwrap().pic;
                    add_image_window_to_monitor(pic, &lock.app, &mo);

                    glib::Continue(false)
                });
            });
        }

        self.app.run_with_args(&[NAME.to_string()]);
    }
}

fn add_image_window_to_monitor(img: &gtk::Image, app: &gtk::Application, monitor: &Monitor) {
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
    gls::set_monitor(&window, monitor);

    // Set up a widget
    window.add(img);
    window.show_all();
}
