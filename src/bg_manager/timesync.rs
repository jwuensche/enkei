// This file contains the main time based loops of enkei
pub enum TransitionState {
    Animation(u64),
    AnimationStart(Transition),
    Change,
    Start,
}

enum Response {
    Continue,
    Finished,
}

use std::{rc::Rc, sync::Mutex};

use super::{BackgroundManager, OutputState};
use crate::metadata;
use gtk::prelude::*;
use log::debug;
use metadata::Transition;

const SIXTYFPS: f32 = 1000.0 / 60.0;

pub fn calc_interval(transition_duration: f64) -> u64 {
    if transition_duration <= 5.0 {
        SIXTYFPS as u64
    } else {
        ((transition_duration * 1000_f64) / 60.0).clamp(1.0, 60000.0) as u64
    }
}

pub fn calc_seconds_to_milli(dur: f64) -> u64 {
    (dur * 1000_f64) as u64
}

pub fn main_tick(bm: Rc<Mutex<BackgroundManager>>, op: TransitionState) -> glib::Continue {
    debug!("MAIN TICK");
    match op {
        TransitionState::Animation(length) => {
            let start = std::time::Instant::now();
            let mut lock = bm.lock().unwrap();
            if let Response::Finished = animation_tick(&mut lock.monitors) {
                debug!("Animation Finished");
                drop(lock);
                main_tick(bm, TransitionState::Change);
                return glib::Continue(false);
            }
            let elapsed = start.elapsed().as_millis();
            if elapsed > length as u128 {
                let factor = ((elapsed / length as u128) + 1) as u64;
                debug!(
                    "System too slow, increasing frame time by factor {}",
                    factor
                );
                drop(lock);
                glib::timeout_add_local(std::time::Duration::from_millis(length * factor), move || {
                    main_tick(
                        bm.clone(),
                        TransitionState::Animation(length * factor),
                    )
                });
                return glib::Continue(false);
            }
            debug!("Elapsed: {}ms", start.elapsed().as_millis());
            glib::Continue(true)
        }
        TransitionState::AnimationStart(slide) => {
            debug!("ANIMATION START");
            let mut lock = bm.lock().unwrap();
            for output in lock.monitors.iter_mut() {
                output.time = std::time::Instant::now();
            }

            drop(lock);
            if slide.is_animated() {
                glib::timeout_add_local(std::time::Duration::from_millis(calc_interval(slide.duration_transition())), move || {
                    main_tick(
                        bm.clone(),
                        TransitionState::Animation(calc_interval(slide.duration_transition())),
                    )
                });
                glib::Continue(false)
            } else {
                main_tick(bm, TransitionState::Change)
            }
        }
        TransitionState::Change | TransitionState::Start => {
            // Load new Image and send loop to create next transition
            let slide;
            let progress;
            debug!("SLIDE");
            let mut lock = bm.lock().unwrap();
            match lock.config.current() {
                Ok(metadata::State::Static(p, tr)) => {
                    progress = p;
                    slide = tr;
                }
                Ok(metadata::State::Transition(p, tr)) => {
                    progress = p + tr.duration_static();
                    slide = tr;
                }
                Err(e) => {
                    eprintln!("Failed to fetch current transition state, likely a problem with implementation details or current slide show. Continuing to avoid crash...
Details: {}", e);
                    return glib::Continue(true);
                }
            }

            if let Err(e) = lock.init_and_load() {
                eprintln!(
                    "Failed due to erroneous loading process. Continuing to avoid crash...
Details: {}",
                    e
                );
                return glib::Continue(true);
            }

            drop(lock);

            if progress < slide.duration_static() {
                // Animation not yet started
                // Wrapper for animation
                glib::timeout_add_local(
                    std::time::Duration::from_millis(calc_seconds_to_milli(slide.duration_static())),
                    move || main_tick(bm.clone(), TransitionState::AnimationStart(slide.clone())),
                );
            } else {
                // Animation has started let's hurry up!
                debug!("ANIMATION_RUSH");
                glib::timeout_add_local(std::time::Duration::from_millis(calc_interval(slide.duration_transition())), move || {
                    main_tick(
                        bm.clone(),
                        TransitionState::Animation(calc_interval(slide.duration_transition())),
                    )
                });
            }

            glib::Continue(false)
        }
    }
}

fn animation_tick(outputs: &mut Vec<OutputState>) -> Response {
    debug!("ANIMATION");
    for output in outputs.iter_mut() {
        if let Some(image_to) = &output.image_to {
            let per = (output.time.elapsed().as_millis() as f64
                / (output.duration_in_sec * 1000) as f64)
                .clamp(0.0, 1.0);
            if per < 1.0 {
                // The composite pixbuf is inefficient let's try cairo
                // let ctx = cairo::Context::new(&output.image_from);
                debug!("{}", per);
                let geometry = output.monitor.geometry();
                let target =
                    cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height)
                        .expect("Cannot create animaion with output geometry as defined in ticks. This is an untreatable error. Please Report.");
                if let Ok(ctx) = cairo::Context::new(&target) {
                    ctx.set_source_surface(&output.image_from, 0.0, 0.0);
                    ctx.paint();
                    ctx.set_source_surface(&image_to, 0.0, 0.0);
                    ctx.paint_with_alpha(ezing::quad_inout(per));
                }

                output.pic.set_from_surface(Some(&target));
            } else {
                return Response::Finished;
            }
        }
    }
    Response::Continue
}
