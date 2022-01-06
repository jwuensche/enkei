use egl::api::EGL1_5;
use image::GenericImageView;
use wayland_client::{Main, global_filter};

use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, Event as LayerEvent, ZwlrLayerSurfaceV1,
};

use std::mem::{size_of, size_of_val};
use std::sync::{Arc, RwLock};
use std::{
    io::{BufWriter, Seek, Write},
    os::unix::io::AsRawFd,
    time::Instant,
};

use wayland_client::{
    protocol::{wl_compositor, wl_shm, wl_output},
    Display, GlobalManager,
};

mod outputs;
mod output;
mod schema;
mod opengl;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Could not access the member `{0}` in some struct.")]
    AccessError(String),
}

impl<'a> From<outputs::OutputError<'a>> for ApplicationError {
    fn from(err: outputs::OutputError<'a>) -> Self {
        match err {
            outputs::OutputError::KeyNotDefined(key) => Self::AccessError(key.into()),
        }
    }
}

fn main() -> Result<(), ApplicationError> {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let wl_outputs = Arc::new(RwLock::new(Vec::new()));
    let pass_outputs = Arc::clone(&wl_outputs);
    let globals = GlobalManager::new_with_cb(
        &attached_display,
        // Let's use the global filter macro provided with the wayland-client crate here
        // The advantage of this that we will get all initially advertised objects (like WlOutput) as a freebe here and don't have to concern with getting
        // all available ones later.
        global_filter!(
            [wl_output::WlOutput, 2, move |output: Main<wl_output::WlOutput>, _: DispatchData| {
                println!("Got a new WlOutput instance!");
                let mut lock = pass_outputs.write().unwrap();
                lock.push(output);
            }]
        )
    );
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    let output_manager = outputs::OutputManager::new(wl_outputs);
    event_queue
        .sync_roundtrip(&mut (), |event, _, _| {
            dbg!(event);
        })
        .unwrap();


    let image = image::open("/home/fred/Pictures/slice/bloke.jpg").unwrap();
    let image2 = image::open("/home/fred/Pictures/slice/bloke2.jpg").unwrap();

    // buffer (and window) width and height
    let mut buf_x: u32 = image.width();
    let mut buf_y: u32 = image.height();

    for output in output_manager.outputs().iter() {
        let lock = output.read().unwrap();
        if let (Some(geo), Some(mode)) = (lock.geometry(), lock.mode()) {
            println!("Found output {} {}:", geo.make(), geo.model());
            println!("  Resolution: {}x{}", mode.width(), mode.height());
            println!("  Position: {}x{}", geo.x(), geo.y());
        }
    }

    /*
     * Init wayland objects
     */

    // The compositor allows us to creates surfaces
    let compositor = globals
        .instantiate_exact::<wl_compositor::WlCompositor>(4)
        .unwrap();
    let surface = compositor.create_surface();
    surface.commit();

    // First define the layer shell interface and configure it before continuing
    let layers = globals.instantiate_exact::<ZwlrLayerShellV1>(2).unwrap();
    let background =
        layers.get_layer_surface(&surface, None, Layer::Background, "wallpaper".into());
    background.set_layer(Layer::Background);
    background.set_anchor(Anchor::all());
    surface.commit();
    background.quick_assign(|layer, event, _| match event {
        LayerEvent::Configure {
            serial,
            width,
            height,
        } => {
            layer.ack_configure(serial);
        }
        _ => {}
    });
    surface.commit();
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
        .unwrap();

    // Create the egl surfaces here and setup the whole party, this should be taken into it's own module but for testing reasons
    // it can still be found here.
    egl.bind_api(egl::OPENGL_API)
                .expect("unable to select OpenGL API");
    gl::load_with(|name| egl.get_proc_address(name).unwrap() as *const std::ffi::c_void);
    let egl_display = setup_egl(&display);
    let (egl_context, egl_config) = create_context(egl_display);
    let wl_egl_surface = wayland_egl::WlEglSurface::new(&surface, buf_x as i32, buf_y as i32);
    let egl_surface = unsafe { egl.create_window_surface(egl_display, egl_config, wl_egl_surface.ptr() as egl::NativeWindowType, None).unwrap() };
    egl.make_current(egl_display, Some(egl_surface), Some(egl_surface), Some(egl_context)).unwrap();
    surface.commit();
    // Rendering with the `gl` bindings are all unsafe let's block this away
    let context = opengl::context::Context::new(&mut image.as_rgb8().unwrap().as_raw().clone(), buf_x as i32, buf_y as i32);
    context.set_to(&mut image2.as_rgb8().unwrap().as_raw().clone(), buf_x as i32, buf_y as i32);
    // Make the buffer the current one
    egl.swap_buffers(egl_display, egl_surface).unwrap();
    surface.commit();

    let input = compositor.create_region();
    surface.set_input_region(Some(&input));
    surface.commit();

    event_queue
        .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
        .unwrap();

    surface.damage(0, 0, i32::max_value(), i32::max_value());
    surface.commit();

    // Process all pending requests
    let mut process = 0.0;
    let mut reverse = false;
    loop {
        event_queue
            .sync_roundtrip(&mut (), |_, event, _| {
                dbg!(event);
            })
            .unwrap();
        context.draw(process);
        egl.swap_buffers(egl_display, egl_surface).unwrap();
        surface.damage(0, 0, i32::max_value(), i32::max_value());
        surface.commit();
        if process >= 1.0 {
            reverse = true;
        }
        if process <= 0.0 {
            reverse = false;
        }
        if reverse {
            process -= 0.016;
        } else {
            process += 0.016;
        }
        dbg!(process);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

use khronos_egl as egl;
// global api object
use egl::API as egl;

fn setup_egl(display: &Display) -> egl::Display {
        let egl_display = egl.get_display(display.get_display_ptr() as *mut std::ffi::c_void).unwrap();
        egl.initialize(egl_display).unwrap();

        egl_display
}

fn create_context(display: egl::Display) -> (egl::Context, egl::Config) {
        let attributes = [
                egl::RED_SIZE,
                8,
                egl::GREEN_SIZE,
                8,
                egl::BLUE_SIZE,
                8,
                egl::NONE,
        ];

        let config = egl.choose_first_config(display, &attributes)
                .expect("unable to choose an EGL configuration")
                .expect("no EGL configuration found");

        let context_attributes = [
                egl::CONTEXT_MAJOR_VERSION,
                4,
                egl::CONTEXT_MINOR_VERSION,
                0,
                egl::CONTEXT_OPENGL_PROFILE_MASK,
                egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                egl::NONE,
        ];

        let context = egl.create_context(display, config, None, &context_attributes)
                .expect("unable to create an EGL context");

        (context, config)
}
