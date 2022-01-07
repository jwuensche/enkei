use std::sync::mpsc::{
    Sender, Receiver,
};

use std::sync::Arc;

use wayland_client::{
    GlobalManager,
    Display, EventQueue,
};

use wayland_client::protocol::{
    wl_compositor,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use image::GenericImageView;

use crate::messages::{self, WorkerMessage};
use crate::watchdog;
use crate::ApplicationError;

pub fn work(
    globals: GlobalManager,
    display: Display,
    messages: Receiver<WorkerMessage>,
    senders: Sender<WorkerMessage>,
    mut event_queue: EventQueue,
) -> Result<(), ApplicationError> {
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

    let image = image::open("/home/fred/Pictures/Taktop/cosette.png").unwrap();
    let image2 = image::open("/home/fred/Pictures/Taktop/more_cosette.png").unwrap();

    // buffer (and window) width and height
    let buf_x: u32 = image.width();
    let buf_y: u32 = image.height();


    // Process all pending requests
    watchdog::timer::initialize(std::time::Duration::from_millis(16), 60, senders.clone());
    loop {
        event_queue
            .sync_roundtrip(&mut (), |_, event, _| {
                dbg!(event);
            })
            .unwrap();

        if let Ok(msg) = messages.try_recv() {
            // do something with new found messages
            match msg {
                messages::WorkerMessage::AddOutput(output) => {
                    dbg!("AddOutput");
                    let lock = output.read().unwrap();
                    if let (Some(geo), Some(mode)) = (lock.geometry(), lock.mode()) {
                        println!("Found output {} {}:", geo.make(), geo.model());
                        println!("      Resolution: {}x{}", mode.width(), mode.height());
                        println!("      Position: {}x{}", geo.x(), geo.y());
                    }
                    drop(lock);
                    println!("Starting window on monitor..");
                    renders.push(OutputRendering::new(&compositor, &layers, &mut event_queue, Arc::clone(&output), egl_context, egl_display, egl_config, buf_x, buf_y, &image, &image2));
                    dbg!(&renders);
                },
                messages::WorkerMessage::RemoveOutput(output) => {
                    todo!()
                },
                messages::WorkerMessage::AnimationStep(process) => {
                    dbg!(process);
                    for output in renders.iter() {
                        output.draw(ezing::quad_inout(process));
                    }
                },
                messages::WorkerMessage::Refresh => todo!(),
            }
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

    }
}


use crate::egl;
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
