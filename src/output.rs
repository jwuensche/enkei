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

use crate::{outputs::ScaledMode, ApplicationError};

use super::outputs::Output;
use crossbeam_channel::unbounded;
use khronos_egl::{Context as eglContext, Display as eglDisplay, Surface as eglSurface};
use log::debug;
use wayland_egl::WlEglSurface;

use super::opengl::context::Context as glContext;

use super::egl;
use wayland_client::{
    protocol::{wl_compositor::WlCompositor, wl_surface::WlSurface},
    EventQueue, Main,
};

use std::rc::Rc;
use std::sync::RwLock;

use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, Event as LayerEvent,
};

#[derive(Debug)]
pub struct OutputRendering {
    pub output: Rc<RwLock<Output>>,
    output_id: u32,
    surface: Main<WlSurface>,
    egl_context: eglContext,
    egl_display: eglDisplay,
    _wl_egl_surface: WlEglSurface,
    egl_surface: eglSurface,
    gl_context: glContext,
    pub resolution: ScaledMode,
    pub scale: i32,
}

impl OutputRendering {
    pub fn new(
        compositor: &Main<WlCompositor>,
        layers: &Main<ZwlrLayerShellV1>,
        event_queue: &mut EventQueue,
        output: Rc<RwLock<Output>>,
        egl_display: eglDisplay,
    ) -> Result<Self, ApplicationError> {
        let lock = output
            .read()
            .map_err(|_| ApplicationError::locked_out(line!(), file!()))?;
        let scale = lock.scale();
        drop(lock);

        let surface = compositor.create_surface();
        surface.commit();
        surface.set_buffer_scale(scale);
        surface.commit();
        let lock = output
            .read()
            .map_err(|_| ApplicationError::locked_out(line!(), file!()))?;
        let output_id = lock.id();
        let background = layers.get_layer_surface(
            &surface,
            Some(lock.inner()),
            Layer::Background,
            "wallpaper".into(),
        );
        background.set_layer(Layer::Background);
        background.set_anchor(Anchor::all());
        background.set_exclusive_zone(-1);
        background.set_size(0, 0);
        surface.commit();
        let (tx, rx) = unbounded();
        background.quick_assign(move |layer, event, _| {
            if let LayerEvent::Configure {
                serial,
                width,
                height,
            } = event
            {
                debug!("Surface registered {{ {width}x{height} }}");
                // On uneven widths we may encounter problems. This happens mostly in situations where we use scaling for Hidpi.
                // But may happen to for virtual displays etc not bound to conventional display limits.
                // Widening the width by one in these cases avoids this issue.
                //
                // If this sending fails, we have to refresh our output, though this is already accomplished via the WlOutput
                // interface so we can drop this error here for good.
                let _ = tx.send((width + (width % 2), height));
                layer.ack_configure(serial);
            }
        });
        surface.commit();
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
            .map_err(|e| ApplicationError::io_error(e, line!(), file!()))?;

        let (width, height) = rx
            .recv()
            .map_err(|_| ApplicationError::OutputDataNotReady)?;
        debug!("Scaling output by factor: {scale}");
        let scaled_mode = ScaledMode {
            width: width as i32 * scale,
            height: height as i32 * scale,
        };
        debug!(
            "Create EGL surface {{ {}x{} }}",
            scaled_mode.width, scaled_mode.height
        );
        let wl_egl_surface =
            wayland_egl::WlEglSurface::new(&surface, scaled_mode.width, scaled_mode.height);
        let (egl_context, egl_config) = create_context(egl_display)?;
        let egl_surface = unsafe {
            egl.create_window_surface(
                egl_display,
                egl_config,
                wl_egl_surface.ptr() as egl::NativeWindowType,
                None,
            )
            .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?
        };
        egl.make_current(
            egl_display,
            Some(egl_surface),
            Some(egl_surface),
            Some(egl_context),
        )
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        egl.swap_interval(egl_display, 0)
            .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        surface.commit();
        // Rendering with the `gl` bindings are all unsafe let's block this away
        let context = super::opengl::context::Context::new();
        // Make the buffer the current one
        egl.swap_buffers(egl_display, egl_surface)
            .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        surface.commit();

        let input = compositor.create_region();
        surface.set_input_region(Some(&input));
        surface.commit();

        event_queue
            .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
            .map_err(|e| ApplicationError::io_error(e, line!(), file!()))?;

        surface.damage(0, 0, i32::max_value(), i32::max_value());
        surface.commit();

        drop(lock);
        Ok(OutputRendering {
            output,
            output_id,
            surface,
            egl_context,
            _wl_egl_surface: wl_egl_surface,
            egl_display,
            egl_surface,
            gl_context: context,
            resolution: scaled_mode,
            scale,
        })
    }

    pub fn set_to(&mut self, image: &[u8], mode: &ScaledMode) -> Result<(), ApplicationError> {
        egl.make_current(
            self.egl_display,
            Some(self.egl_surface),
            Some(self.egl_surface),
            Some(self.egl_context),
        )
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        self.gl_context
            .set_to(image, mode.width, mode.height);
        Ok(())
    }

    pub fn set_from(&mut self, image: &[u8], mode: &ScaledMode) -> Result<(), ApplicationError> {
        egl.make_current(
            self.egl_display,
            Some(self.egl_surface),
            Some(self.egl_surface),
            Some(self.egl_context),
        )
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        self.gl_context
            .set_from(image, mode.width, mode.height);
        Ok(())
    }

    pub fn output_id(&self) -> u32 {
        self.output_id
    }

    pub fn draw(&self, process: f32) -> Result<(), ApplicationError> {
        egl.make_current(
            self.egl_display,
            Some(self.egl_surface),
            Some(self.egl_surface),
            Some(self.egl_context),
        )
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        self.gl_context.draw(process);
        egl.swap_buffers(self.egl_display, self.egl_surface)
            .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        self.surface
            .damage(0, 0, i32::max_value(), i32::max_value());
        self.surface.commit();
        Ok(())
    }

    pub fn destroy(&self) -> Result<(), ApplicationError> {
        self.surface.destroy();
        egl.destroy_surface(self.egl_display, self.egl_surface)
            .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        egl.destroy_context(self.egl_display, self.egl_context)
            .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;
        // self.gl_context.destroy();
        Ok(())
    }
}

fn create_context(display: egl::Display) -> Result<(egl::Context, egl::Config), ApplicationError> {
    let attributes = [
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::NONE,
    ];

    let config = egl
        .choose_first_config(display, &attributes)
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?
        .ok_or_else(|| ApplicationError::egl_error(crate::EglError::BadAccess, line!(), file!()))?;

    let context_attributes = [
        egl::CONTEXT_MAJOR_VERSION,
        4,
        egl::CONTEXT_MINOR_VERSION,
        0,
        egl::CONTEXT_OPENGL_PROFILE_MASK,
        egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
        egl::NONE,
    ];

    let context = egl
        .create_context(display, config, None, &context_attributes)
        .map_err(|e| ApplicationError::egl_error(e, line!(), file!()))?;

    Ok((context, config))
}
