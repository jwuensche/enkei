// This file contains the main time based loops of enkei
pub enum TransitionState {
    Animation(u32),
    AnimationStart(Transition),
    Change,
    Start,
}

enum Response {
    Continue,
    Finished,
}

use super::{BackgroundManager, OutputState};
use crate::metadata;
use gtk::prelude::*;
use log::debug;
use metadata::Transition;

pub fn calc_interval(transition_duration: u32) -> u32 {
    ((transition_duration * 1000) as f64 / 60.0).clamp(1.0, 60000.0) as u32
}

pub fn main_tick(mut bm: BackgroundManager, op: TransitionState) -> glib::Continue {
    match op {
        TransitionState::Animation(length) => {
            let start = std::time::Instant::now();
            if let Response::Finished = animation_tick(&mut bm.monitors) {
                main_tick(bm, TransitionState::Change);
                return glib::Continue(false);
            }
            let elapsed = start.elapsed().as_millis();
            if elapsed > length as u128 {
                let factor = (elapsed / length as u128) + 1;
                debug!(
                    "System too slow, increasing frame time by factor {}",
                    factor
                );
                glib::timeout_add_local(length * factor as u32, move || {
                    main_tick(
                        bm.clone(),
                        TransitionState::Animation(length * factor as u32),
                    )
                });
                return glib::Continue(false);
            }
            debug!("Elapsed: {}ms", start.elapsed().as_millis());
            glib::Continue(true)
        }
        TransitionState::AnimationStart(slide) => {
            debug!("{}", "ANIMATION_WRAPPER");
            for output in bm.monitors.iter_mut() {
                output.time = std::time::Instant::now();
            }

            glib::timeout_add_local(calc_interval(slide.duration_transition), move || {
                main_tick(
                    bm.clone(),
                    TransitionState::Animation(calc_interval(slide.duration_transition)),
                )
            });
            glib::Continue(false)
        }
        TransitionState::Change | TransitionState::Start => {
            // Load new Image and send loop to create next transition
            let slide;
            let progress;
            debug!("{}", "SLIDE");
            match bm.config.current() {
                Ok(metadata::State::Static(p, tr)) => {
                    progress = p;
                    slide = tr;
                }
                Ok(metadata::State::Transition(p, tr)) => {
                    progress = p + tr.duration_static;
                    slide = tr;
                }
                Err(e) => {
                    eprintln!("Failed to fetch current transition state, likely a problem with implementation details or current slide show. Continuing to avoid crash...
    Details: {}", e);
                    return glib::Continue(true);
                }
            }

            if let Err(e) = bm.init_and_load() {
                eprintln!("Failed due to erroneous loading process. Continuing to avoid crash...
Details: {}",e);
                return glib::Continue(true);
            }

            if progress < slide.duration_static {
                // Animation not yet started
                // Wrapper for animation
                glib::timeout_add_seconds_local(slide.duration_static, move || {
                    main_tick(bm.clone(), TransitionState::AnimationStart(slide.clone()))
                });
            } else {
                // Animation has started let's hurry up!
                debug!("{}", "ANIMATION_RUSH");
                glib::timeout_add_local(calc_interval(slide.duration_transition), move || {
                    main_tick(
                        bm.clone(),
                        TransitionState::Animation(calc_interval(slide.duration_transition)),
                    )
                });
            }

            glib::Continue(false)
        }
    }
}

fn animation_tick(outputs: &mut Vec<OutputState>) -> Response {
    debug!("{}", "ANIMATION");
    for output in outputs.iter_mut() {
        let per = (output.time.elapsed().as_millis() as f64
            / (output.duration_in_sec * 1000) as f64)
            .clamp(0.0, 1.0);
        if per < 1.0 {
            // The composite pixbuf is inefficient let's try cairo
            // let ctx = cairo::Context::new(&output.image_from);
            debug!("{}", per);
            let geometry = output.monitor.get_geometry();
            let target =
                cairo::ImageSurface::create(cairo::Format::ARgb32, geometry.width, geometry.height)
                    .expect("Cannot create animaion with output geometry as defined in ticks. This is an untreatable error. Please Report.");
            let ctx = cairo::Context::new(&target);
            ctx.set_source_surface(&output.image_from, 0.0, 0.0);
            ctx.paint();
            ctx.set_source_surface(&output.image_to, 0.0, 0.0);
            ctx.paint_with_alpha(ezing::quad_inout(per));

            output.pic.set_from_surface(Some(&target));
        } else {
            return Response::Finished;
        }
    }
    Response::Continue
}
