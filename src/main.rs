use image::GenericImageView;
use wayland_client::{Main, global_filter};

use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use std::sync::{Arc, RwLock};

use wayland_client::{
    protocol::{wl_compositor, wl_output},
    Display, GlobalManager,
};

mod outputs;
mod output;
mod schema;
mod opengl;
mod metadata;

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
    let buf_x: u32 = image.width();
    let buf_y: u32 = image.height();

    /*
     * Init wayland objects
     */

    // The compositor allows us to creates surfaces
    let compositor = globals
        .instantiate_exact::<wl_compositor::WlCompositor>(4)
        .unwrap();

    // First define the layer shell interface and configure it before continuing
    let layers = globals.instantiate_exact::<ZwlrLayerShellV1>(2).unwrap();

    // Create the egl surfaces here and setup the whole party, this should be taken into it's own module but for testing reasons
    // it can still be found here.
    egl.bind_api(egl::OPENGL_API)
                .expect("unable to select OpenGL API");
    gl::load_with(|name| egl.get_proc_address(name).unwrap() as *const std::ffi::c_void);
    let egl_display = setup_egl(&display);
    let (egl_context, egl_config) = create_context(egl_display);

    let mut renders = Vec::new();

    for output in output_manager.outputs().iter() {
        let lock = output.read().unwrap();
        if let (Some(geo), Some(mode)) = (lock.geometry(), lock.mode()) {
            println!("Found output {} {}:", geo.make(), geo.model());
            println!("  Resolution: {}x{}", mode.width(), mode.height());
            println!("  Position: {}x{}", geo.x(), geo.y());
        }
        drop(lock);
        println!("Starting window on monitor..");
        renders.push(OutputRendering::new(&compositor, &layers, &mut event_queue, Arc::clone(&output), egl_context, egl_display, egl_config, buf_x, buf_y, &image, &image2));
    }

    // Process all pending requests
    let mut process = 0.0;
    let mut reverse = false;
    loop {
        event_queue
            .sync_roundtrip(&mut (), |_, event, _| {
                dbg!(event);
            })
            .unwrap();
        // context.draw(process);
        // egl.swap_buffers(egl_display, egl_surface).unwrap();
        // surface.damage(0, 0, i32::max_value(), i32::max_value());
        // surface.commit();
        for foo in renders.iter() {
            foo.draw(process);
        }
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
        // dbg!(process);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

use khronos_egl as egl;
// global api object
use egl::API as egl;

use crate::output::OutputRendering;

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
