// enkei: An OpenGL accelerated wallpaper tool for wayland
// Copyright (C) 2022 Johannes Wünsche
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crossbeam_channel::{unbounded, Receiver, Sender};

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
    stop_all_timers: Sender<()>,
    timer_receiver: Receiver<()>,
}

impl State {
    fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            fps: 1f64,
            ticker_active: false,
            renders: HashMap::new(),
            stop_all_timers: tx,
            timer_receiver: rx,
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
    let layers = globals
        .instantiate_exact::<ZwlrLayerShellV1>(2)
        .map_err(ApplicationError::WaylandObject)?;

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
                        debug!("Output {{ id: {id} }} updated and not new. Refreshing.");
                        // On Change we have to reinitialize the output, though this needs
                        // to be a complete reinit with all surfaces so for simplicity we
                        // "destroy" the output here and add it anew.
                        senders
                            .send(WorkerMessage::RemoveOutput(id))
                            .expect("Cannot fail");
                        senders
                            .send(WorkerMessage::AddOutput(output, id))
                            .expect("Cannot fail");
                    } else {
                        let lock = output
                            .read()
                            .map_err(|_| ApplicationError::locked_out(line!(), file!()))?;
                        if let Some(geo) = lock.geometry() {
                            debug!("Rendering on output {{ make: {}, model: {}, position: {}x{} }}", geo.make(), geo.model(), geo.x(), geo.y());
                        }
                        if let Some(fps) = lock.refresh_rate() {
                            state.set_fps(fps);
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
                        )?;
                        state.ticker_active = state_draw(
                            &animation_state,
                            output,
                            state.ticker_active,
                            state.fps,
                            senders.clone(),
                            state.timer_receiver.clone(),
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
                        state.timer_receiver.clone(),
                    );
                }
                WorkerMessage::Refresh => {
                    debug!("Message: Refresh");
                    let start = std::time::Instant::now();
                    state.ticker_active = false;
                    state.stop_all_timers.send(()).expect("Cannot fail");
                    // drain the current channel
                    while let Ok(_) = state.timer_receiver.try_recv() {}
                    let animation_state = metadata.current()?;
                    for (_, output) in state.renders.iter_mut() {
                        refresh_output(
                            output,
                            &mut resource_loader,
                            &animation_state,
                            Scaling::Fill,
                            Filter::Best,
                        )?;
                        state.ticker_active = state_draw(
                            &animation_state,
                            output,
                            state.ticker_active,
                            state.fps,
                            senders.clone(),
                            state.timer_receiver.clone(),
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
    recv: Receiver<()>,
) -> Result<bool, ApplicationError> {
    match animation_state {
        AnimationState::Static(progress, transition) => {
            debug!(
                "Current state is static {{ duration_static: {}, progress: {progress} }}",
                transition.duration_static()
            );
            if transition.is_animated() && !ticker_active {
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    recv,
                    WorkerMessage::AnimationStart(transition.duration_transition()),
                );
                ticker_active = true;
            } else if !ticker_active {
                timer::spawn_simple_timer(
                    std::time::Duration::from_secs_f64(transition.duration_static() - progress),
                    senders,
                    recv,
                    WorkerMessage::Refresh,
                );
                ticker_active = true;
            }
            output.draw(0.0)?;
            Ok(ticker_active)
        }
        AnimationState::Transition(progress, transition) => {
            // This state is always animated
            debug!(
                "Current state is dynamic {{ duration_transition: {}, progress: {progress} }}",
                transition.duration_transition()
            );
            let count = calc_frame_updates(transition.duration_transition(), fps);
            let step = transition.duration_transition() / count;
            let finished = progress / step;
            if !ticker_active {
                timer::spawn_animation_ticker(
                    std::time::Duration::from_secs_f64(step),
                    count as u64,
                    finished as u64,
                    senders,
                    recv,
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
    let transition = {
        match metadata {
            AnimationState::Static(_, t) => t,
            AnimationState::Transition(_, t) => t,
        }
    };
    let scaled_mode = output.resolution.clone();

    let from = resources.load(transition.from(), &scaled_mode, scaling, filter)?;
    let start = std::time::Instant::now();
    output.set_from(from, &scaled_mode)?;
    debug!(
        "Sending of image texture to shader took {}ms",
        start.elapsed().as_millis()
    );
    if transition.is_animated() {
        let to = resources.load(
            transition.to().expect("Cannot fail."),
            &scaled_mode,
            scaling,
            filter,
        )?;
        output.set_to(to, &scaled_mode)?;
    } else {
        output.set_to(from, &scaled_mode)?;
    }
    Ok(())
}

use crate::egl;
use crate::output::OutputRendering;

fn setup_egl(display: &Display) -> Result<egl::Display, ApplicationError> {
    egl.bind_api(egl::OPENGL_API)
        .expect("unable to select OpenGL API");
    gl::load_with(|name| {
        egl.get_proc_address(name)
            .expect("Could not get process address. FATAL.") as *const std::ffi::c_void
    });

    let egl_display = egl
        .get_display(display.get_display_ptr() as *mut std::ffi::c_void)
        .ok_or_else(|| ApplicationError::EGLSetup("Could not get EGL display.".into()))?;
    egl.initialize(egl_display)
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
    Ok(egl_display)
}
