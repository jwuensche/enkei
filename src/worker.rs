use std::io::Read;
use std::sync::mpsc::{Receiver, Sender};

use std::sync::{Arc, RwLock};

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
                unreachable!();
            })
            .unwrap();
        println!("Processed messages");

        if let Ok(msg) = messages.try_recv() {
            // do something with new found messages
            match msg {
                messages::WorkerMessage::AddOutput(output, id) => {
                    println!("Message: AddOutput");

                    if renders
                        .iter()
                        .filter(|elem: &&OutputRendering| elem.output_id() == id)
                        .count()
                        > 0
                    {
                        println!("Display updated and not new.");
                    } else {
                        let lock = output.read().unwrap();
                        if let (Some(geo), Some(mode)) = (lock.geometry(), lock.mode()) {
                            println!("Found output {} {}:", geo.make(), geo.model());
                            println!("      Resolution: {}x{}", mode.width(), mode.height());
                            println!("      Position: {}x{}", geo.x(), geo.y());
                        }
                        let width = *lock.mode().unwrap().width();
                        let height = *lock.mode().unwrap().height();
                        drop(lock);
                        println!("Starting window on monitor..");
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
                    println!("Message: RemoveOutput");
                    let mut res = renders.iter().enumerate().filter_map(|elem| {
                        let lock = elem.1.output.read().unwrap();
                        if lock.id() == id {
                            Some(elem.0)
                        } else {
                            None
                        }
                    });

                    if let Some(valid) = res.next() {
                        println!("Removing Output");
                        dbg!(&renders);
                        renders.swap_remove(valid);
                        dbg!(&renders);
                    }
                }
                messages::WorkerMessage::AnimationStep(process) => {
                    println!("Message: AnimationStep");
                    for output in renders.iter() {
                        println!("Drawing output: {:?}", output);
                        output.draw(ezing::quad_inout(process));
                    }
                    println!("Finished animation drawing");
                    if process >= 1.0 {
                        senders
                            .send(WorkerMessage::Refresh)
                            .expect("This should never break");
                    }
                }
                messages::WorkerMessage::AnimationStart(duration) => {
                    println!("Message: AnimationStart");
                    let count = (duration * FPS).clamp(1.0, 600.0);
                    println!(
                        "Spawn Ticker (step duration: {}s, count: {})",
                        duration / count,
                        count as u64
                    );
                    timer::spawn_animation_ticker(
                        std::time::Duration::from_secs_f64(duration / count),
                        count as u64,
                        0,
                        senders.clone(),
                    );
                }
                messages::WorkerMessage::Refresh => {
                    ticker_active = false;
                    println!("Message: Refresh");
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
                    // Cancel all running timer watchdogs
                }
            }
        } else {
            println!("Got no message, waiting...");
            std::thread::sleep(std::time::Duration::from_millis(500));
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
                println!(
                    "Spawn Simple Timer(duration: {}s, transition: {}s)",
                    transition.duration_static() - progress,
                    transition.duration_transition()
                );
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    WorkerMessage::AnimationStart(transition.duration_transition()),
                );
                *ticker_active = true;
            } else if !*ticker_active {
                println!(
                    "Spawn Simple Timer(duration: {}s)",
                    transition.duration_static() - progress
                );
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

    let mut from = resources
        .load(transition.from(), scaling, filter)
        .unwrap()
        .process(&mode)
        .expect("Could not get Image data");
    output.set_from(&mut from, width, height);
    if transition.is_animated() {
        let mut to = resources
            .load(transition.to().unwrap(), scaling, filter)
            .unwrap()
            .process(&mode)
            .expect("Could not get Image data");
        output.set_to(&mut to, width, height);
    } else {
        output.set_to(&mut from, width, height);
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
