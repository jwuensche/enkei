use std::sync::mpsc::{Receiver, Sender};

use std::rc::Rc;

use log::debug;
use wayland_client::{Display, EventQueue, GlobalManager};

use wayland_client::protocol::wl_compositor;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use crate::image::scaling::{Filter, Scaling};

use crate::messages::WorkerMessage;
use crate::metadata::{AnimationState, Metadata};
use crate::util::ResourceLoader;
use crate::watchdog::timer;
use crate::ApplicationError;
use std::collections::HashMap;

pub struct State {
    fps: f64,
    ticker_active: bool,
    renders: HashMap<u32, OutputRendering>,
}

impl State {
    fn new() -> Self {
        Self {
            fps: 1f64,
            ticker_active: false,
            renders: HashMap::new(),
        }
    }

    fn set_fps(&mut self, new: f64) {
        self.fps = f64::max(self.fps, new);
    }
}

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
        .map_err(ApplicationError::WaylandObject)?;

    // First define the layer shell interface and configure it before continuing
    let layers = globals.instantiate_exact::<ZwlrLayerShellV1>(2)
        .map_err(ApplicationError::WaylandObject)?;

    // Create the egl surfaces here and setup the whole party, this should be taken into it's own module but for testing reasons
    // it can still be found here.
    egl.bind_api(egl::OPENGL_API)
        .expect("unable to select OpenGL API");
    gl::load_with(|name| egl.get_proc_address(name).expect("Could not get process address. FATAL.") as *const std::ffi::c_void);
    let egl_display = setup_egl(&display)?;

    // Use an output independent store for loaded images, allows for some reduction in IO time
    let mut resource_loader = ResourceLoader::new();
    let mut state = State::new();

    // Process all pending requests
    loop {
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| {
                // NO-OP
            })
            .map_err(|e| ApplicationError::io_error(e, line!(), file!()))?;

        if let Ok(msg) = messages.recv_timeout(std::time::Duration::from_millis(500)) {
            // do something with new found messages
            match msg {
                WorkerMessage::AddOutput(output, id) => {
                    debug!("Message: AddOutput {{ id: {} }}", id);

                    if state.renders.contains_key(&id) {
                        debug!("Output {{ id: {id} }} updated and not new.");
                    } else {
                        let lock = output.read()
                            .map_err(|_| ApplicationError::locked_out(line!(), file!()))?;
                        if let (Some(geo), Some(mode)) = (lock.geometry(), lock.mode()) {
                            debug!("Rendering on output {{ make: {}, model: {}, resolution: {}x{}, position: {}x{} }}", geo.make(), geo.model(), mode.width(), mode.height(), geo.x(), geo.y());
                        }
                        let width;
                        let height;
                        if let Some(mode) = lock.mode() {
                            width = *mode.width();
                            height = *mode.height();
                            state.set_fps(*mode.refresh() as f64 / 1000f64);
                        } else {
                            return Err(ApplicationError::OutputDataNotReady)
                        }
                        drop(lock);
                        state.renders.insert(
                            id,
                            OutputRendering::new(
                                &compositor,
                                &layers,
                                &mut event_queue,
                                Rc::clone(&output),
                                egl_display,
                                width as u32,
                                height as u32,
                            )?,
                        );
                        let output = state.renders.get_mut(&id).expect("Cannot fail");
                        let animation_state = metadata.current()?;
                        refresh_output(
                            output,
                            &mut resource_loader,
                            &animation_state,
                            Scaling::Fill,
                            Filter::Best,
                        )
                        .expect("Could not refresh");
                        state.ticker_active = state_draw(
                            &animation_state,
                            output,
                            state.ticker_active,
                            state.fps,
                            senders.clone(),
                        )?;
                    }
                }
                WorkerMessage::RemoveOutput(id) => {
                    debug!("Message: RemoveOutput {{ id: {} }}", id);

                    if let Some(output) = state.renders.remove(&id) {
                        debug!("Removing WlOuput Renderer {{ id: {} }}", output.output_id());
                        output.destroy()?;
                    }
                }
                WorkerMessage::AnimationStep(process) => {
                    debug!("Message: AnimationStep {{ process: {} }}", process);
                    for (id, output) in state.renders.iter() {
                        debug!("Drawing on WlOutput {{ id: {} }}", id);
                        output.draw(ezing::quad_inout(process))?;
                    }
                    if process >= 1.0 {
                        senders
                            .send(WorkerMessage::Refresh)
                            .expect("This should never break");
                    }
                }
                WorkerMessage::AnimationStart(duration) => {
                    debug!("Message: AnimationStart {{ duration: {}s }}", duration);
                    let count = calc_frame_updates(duration, state.fps);
                    timer::spawn_animation_ticker(
                        std::time::Duration::from_secs_f64(duration / count),
                        count as u64,
                        0,
                        senders.clone(),
                    );
                }
                WorkerMessage::Refresh => {
                    debug!("Message: Refresh");
                    let start = std::time::Instant::now();
                    state.ticker_active = false;
                    // TODO: Cancel all running timer watchdogs
                    let animation_state = metadata.current()?;
                    for (_, output) in state.renders.iter_mut() {
                        refresh_output(
                            output,
                            &mut resource_loader,
                            &animation_state,
                            Scaling::Fill,
                            Filter::Best,
                        )
                        .expect("Could not refresh");
                        state.ticker_active = state_draw(
                            &animation_state,
                            output,
                            state.ticker_active,
                            state.fps,
                            senders.clone(),
                        )?;
                    }
                    debug!(
                        "Refreshing of all outputs took {}ms",
                        start.elapsed().as_millis()
                    );
                }
            }
        }
    }
}

fn calc_frame_updates(duration: f64, fps: f64) -> f64 {
    (duration * fps).clamp(1.0, 300.0)
}

fn state_draw(
    animation_state: &AnimationState,
    output: &mut OutputRendering,
    mut ticker_active: bool,
    fps: f64,
    senders: Sender<WorkerMessage>,
) -> Result<bool, ApplicationError> {
    match animation_state {
        AnimationState::Static(progress, transition) => {
            if transition.is_animated() && !ticker_active {
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    WorkerMessage::AnimationStart(transition.duration_transition()),
                );
                ticker_active = true;
            } else if !ticker_active {
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    WorkerMessage::Refresh,
                );
                ticker_active = true;
            }
            output.draw(0.0)?;
            Ok(ticker_active)
        }
        AnimationState::Transition(progress, transition) => {
            // This state is always animated
            let count = calc_frame_updates(transition.duration_transition(), fps);
            let step = transition.duration_transition() / count;
            let finished = progress / step;
            if !ticker_active {
                timer::spawn_animation_ticker(
                    std::time::Duration::from_secs_f64(step),
                    count as u64,
                    finished as u64,
                    senders,
                );
                ticker_active = true;
            }
            output.draw((finished / count) as f32)?;
            Ok(ticker_active)
        }
    }
}

fn refresh_output(
    output: &mut OutputRendering,
    resources: &mut ResourceLoader,
    metadata: &AnimationState,
    scaling: Scaling,
    filter: Filter,
) -> Result<(), ApplicationError> {
    let lock = output.output.read()
                         .map_err(|_| ApplicationError::locked_out(line!(), file!()))?;
    let width;
    let height;
    let mode;
    if let Some(output_mode) = lock.mode() {
        width = *output_mode.width();
        height = *output_mode.height();
        mode = *output_mode;
    } else {
        return Err(ApplicationError::OutputDataNotReady);
    }
    drop(lock);

    let transition = {
        match metadata {
            AnimationState::Static(_, t) => t,
            AnimationState::Transition(_, t) => t,
        }
    };

    let from = resources
        .load(transition.from(), &mode, scaling, filter)?;
    let start = std::time::Instant::now();
    output.set_from(from, width, height)?;
    debug!(
        "Sending of image texture to shader took {}ms",
        start.elapsed().as_millis()
    );
    if transition.is_animated() {
        let to = resources
            .load(transition.to().expect("Cannot fail."), &mode, scaling, filter)?;
        output.set_to(to, width, height)?;
    } else {
        output.set_to(from, width, height)?;
    }
    Ok(())
}

use crate::egl;
use crate::output::OutputRendering;

fn setup_egl(display: &Display) -> Result<egl::Display, ApplicationError> {
    let egl_display = egl
        .get_display(display.get_display_ptr() as *mut std::ffi::c_void)
        .ok_or(ApplicationError::EGLSetup("Could not get EGL display.".into()))?;
    egl.initialize(egl_display).map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
    Ok(egl_display)
}
