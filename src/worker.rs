use std::io::Read;
use std::sync::mpsc::{Receiver, Sender};

use std::sync::{Arc, RwLock};

use log::debug;
use wayland_client::{Display, EventQueue, GlobalManager};

use wayland_client::protocol::wl_compositor;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use image::GenericImageView;

use crate::image::scaling::{Filter, Scaling};

use crate::messages::{self, WorkerMessage};
use crate::metadata::{Metadata, MetadataError, MetadataReader, State};
use crate::outputs::Output;
use crate::util::ResourceLoader;
use crate::watchdog::{self, timer};
use crate::ApplicationError;

const FPS: f64 = 60.0;

pub fn work(
    globals: GlobalManager,
    display: Display,
    messages: Receiver<WorkerMessage>,
    senders: Sender<WorkerMessage>,
    mut event_queue: EventQueue,
    metadata: Metadata,
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

    // Use an output independent store for loaded images, allows for some reduction in IO time
    let mut resource_loader = ResourceLoader::new();

    let mut renders = Vec::new();
    let mut ticker_active = false;

    // Process all pending requests
    loop {
        event_queue
            .sync_roundtrip(&mut (), |_, event, _| {
                // NO-OP
            })
            .unwrap();

        if let Ok(msg) = messages.recv_timeout(std::time::Duration::from_millis(500)) {
            // do something with new found messages
            match msg {
                messages::WorkerMessage::AddOutput(output, id) => {
                    debug!("Message: AddOutput {{ id: {} }}", id);

                    if renders
                        .iter()
                        .filter(|elem: &&OutputRendering| elem.output_id() == id)
                        .count()
                        > 0
                    {
                        debug!("Display updated and not new.");
                    } else {
                        let lock = output.read().unwrap();
                        if let (Some(geo), Some(mode)) = (lock.geometry(), lock.mode()) {
                            debug!("Rendering on output {{ make: {}, model: {}, resolution: {}x{}, position: {}x{} }}", geo.make(), geo.model(), mode.width(), mode.height(), geo.x(), geo.y());
                        }
                        let width = *lock.mode().unwrap().width();
                        let height = *lock.mode().unwrap().height();
                        drop(lock);
                        renders.push(OutputRendering::new(
                            &compositor,
                            &layers,
                            &mut event_queue,
                            Arc::clone(&output),
                            egl_display,
                            width as u32,
                            height as u32,
                        ));
                        let output = renders.last_mut().unwrap();
                        let state = metadata.current()?;
                        refresh_output(
                            output,
                            &mut resource_loader,
                            &state,
                            Scaling::Fill,
                            Filter::Best,
                        )
                        .expect("Could not refresh");
                        state_draw(&state, output, &mut ticker_active, senders.clone());
                    }
                }
                messages::WorkerMessage::RemoveOutput(id) => {
                    debug!("Message: RemoveOutput {{ id: {} }}", id);
                    let mut res = renders.iter().enumerate().filter_map(|elem| {
                        let lock = elem.1.output.read().unwrap();
                        if lock.id() == id {
                            Some(elem.0)
                        } else {
                            None
                        }
                    });

                    if let Some(valid) = res.next() {
                        debug!(
                            "Removing WlOuput Renderer {{ id: {} }}",
                            renders[valid].output_id()
                        );
                        renders[valid].destroy();
                        renders.swap_remove(valid);
                    }
                }
                messages::WorkerMessage::AnimationStep(process) => {
                    debug!("Message: AnimationStep {{ process: {} }}", process);
                    for output in renders.iter() {
                        debug!("Drawing on WlOutput {{ id: {} }}", output.output_id());
                        output.draw(ezing::quad_inout(process));
                    }
                    if process >= 1.0 {
                        senders
                            .send(WorkerMessage::Refresh)
                            .expect("This should never break");
                    }
                }
                messages::WorkerMessage::AnimationStart(duration) => {
                    debug!("Message: AnimationStart {{ duration: {}s }}", duration);
                    let count = (duration * FPS).clamp(1.0, 600.0);
                    timer::spawn_animation_ticker(
                        std::time::Duration::from_secs_f64(duration / count),
                        count as u64,
                        0,
                        senders.clone(),
                    );
                }
                messages::WorkerMessage::Refresh => {
                    debug!("Message: Refresh");
                    let start = std::time::Instant::now();
                    ticker_active = false;
                    let state = metadata.current()?;
                    for output in renders.iter_mut() {
                        refresh_output(
                            output,
                            &mut resource_loader,
                            &state,
                            Scaling::Fill,
                            Filter::Best,
                        )
                        .expect("Could not refresh");
                        state_draw(&state, output, &mut ticker_active, senders.clone());
                    }
                    debug!(
                        "Refreshing of all outputs took {}ms",
                        start.elapsed().as_millis()
                    );
                    // Cancel all running timer watchdogs
                }
            }
        }
    }
}

fn state_draw(
    state: &State,
    output: &mut OutputRendering,
    ticker_active: &mut bool,
    senders: Sender<WorkerMessage>,
) {
    match state {
        State::Static(progress, transition) => {
            if transition.is_animated() && !*ticker_active {
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    WorkerMessage::AnimationStart(transition.duration_transition()),
                );
                *ticker_active = true;
            } else if !*ticker_active {
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    WorkerMessage::Refresh,
                );
                *ticker_active = true;
            }
            output.draw(0.0);
        }
        State::Transition(progress, transition) => {
            // This state is always animated
            let count = (transition.duration_transition() * FPS).clamp(1.0, 600.0);
            let step = transition.duration_transition() / count;
            let finished = progress / step;
            if !*ticker_active {
                timer::spawn_animation_ticker(
                    std::time::Duration::from_secs_f64(step),
                    count as u64,
                    finished as u64,
                    senders,
                );
                *ticker_active = true;
            }
            output.draw((finished / count) as f32);
        }
    }
}

fn refresh_output(
    output: &mut OutputRendering,
    resources: &mut ResourceLoader,
    metadata: &State,
    scaling: Scaling,
    filter: Filter,
) -> Result<(), MetadataError> {
    let lock = output.output.read().unwrap();
    let width = *lock.mode().unwrap().width();
    let height = *lock.mode().unwrap().height();
    let mode: crate::outputs::Mode = lock.mode().unwrap().clone();
    drop(lock);

    let transition;
    let progress;
    match metadata {
        State::Static(p, t) => {
            progress = p;
            transition = t;
        }
        State::Transition(p, t) => {
            progress = p;
            transition = t;
        }
    }

    // TODO: Handle errors here
    let from = resources
        .load(transition.from(), &mode, scaling, filter)
        .expect("Could not get Image data");
    let start = std::time::Instant::now();
    output.set_from(from, width, height);
    debug!(
        "Sending of image texture to shader took {}ms",
        start.elapsed().as_millis()
    );
    if transition.is_animated() {
        let to = resources
            .load(transition.to().unwrap(), &mode, scaling, filter)
            .expect("Could not get Image data");
        output.set_to(to, width, height);
    } else {
        output.set_to(from, width, height);
    }
    Ok(())
}

use crate::egl;
use crate::output::OutputRendering;

fn setup_egl(display: &Display) -> egl::Display {
    let egl_display = egl
        .get_display(display.get_display_ptr() as *mut std::ffi::c_void)
        .unwrap();
    egl.initialize(egl_display).unwrap();
    egl_display
}
