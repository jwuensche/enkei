use crate::image::error::ImageError;
use crate::{
    image::scaling::{Filter, Scaling},
    outputs::Mode,
    util::ResourceLoader,
};

use super::outputs::Output;
use image::DynamicImage;
use khronos_egl::{
    Config as eglConfig, Context as eglContext, Display as eglDisplay, Surface as eglSurface,
};
use wayland_egl::WlEglSurface;

use super::opengl::context::Context as glContext;

use super::egl;
use wayland_client::{
    protocol::{wl_compositor::WlCompositor, wl_display, wl_surface::WlSurface},
    EventQueue, Main,
};

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, Event as LayerEvent, ZwlrLayerSurfaceV1,
};

#[derive(Debug)]
pub struct OutputRendering {
    pub output: Arc<RwLock<Output>>,
    output_id: u32,
    scaled: HashMap<String, Vec<u8>>,
    surface: Main<WlSurface>,
    egl_context: eglContext,
    egl_display: eglDisplay,
    wl_egl_surface: WlEglSurface,
    egl_surface: eglSurface,
    gl_context: glContext,
}

impl OutputRendering {
    pub fn new(
        compositor: &Main<WlCompositor>,
        layers: &Main<ZwlrLayerShellV1>,
        event_queue: &mut EventQueue,
        output: Arc<RwLock<Output>>,
        egl_display: eglDisplay,
        buf_x: u32,
        buf_y: u32,
    ) -> Self {
        let surface = compositor.create_surface();
        surface.commit();
        let lock = output.read().unwrap();
        let output_id = lock.id();
        let background = layers.get_layer_surface(
            &surface,
            Some(lock.inner()),
            Layer::Background,
            "wallpaper".into(),
        );
        background.set_layer(Layer::Background);
        background.set_anchor(Anchor::all());
        surface.commit();
        background.quick_assign(|layer, event, _| {
            if let LayerEvent::Configure {
                serial,
                width,
                height,
            } = event
            {
                // Ignore the resolution received while registering, we know on which output we are.
                layer.ack_configure(serial);
            }
        });
        surface.commit();
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
            .unwrap();

        let wl_egl_surface = wayland_egl::WlEglSurface::new(&surface, buf_x as i32, buf_y as i32);
        let (egl_context, egl_config) = create_context(egl_display);
        let egl_surface = unsafe {
            egl.create_window_surface(
                egl_display,
                egl_config,
                wl_egl_surface.ptr() as egl::NativeWindowType,
                None,
            )
            .unwrap()
        };
        egl.make_current(
            egl_display,
            Some(egl_surface),
            Some(egl_surface),
            Some(egl_context),
        )
        .unwrap();
        egl.swap_interval(egl_display, 0).unwrap();
        surface.commit();
        // Rendering with the `gl` bindings are all unsafe let's block this away
        let context = super::opengl::context::Context::new(buf_x as i32, buf_y as i32);
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

        drop(lock);
        OutputRendering {
            output,
            output_id,
            scaled: HashMap::new(),
            surface,
            egl_context,
            wl_egl_surface,
            egl_display,
            egl_surface,
            gl_context: context,
        }
    }

    pub fn set_to<I: Into<i32>>(&mut self, image: &[u8], width: I, height: I) {
        self.gl_context.set_to(image, width.into(), height.into())
    }

    pub fn set_from<I: Into<i32>>(&mut self, image: &[u8], width: I, height: I) {
        egl.make_current(
            self.egl_display,
            Some(self.egl_surface),
            Some(self.egl_surface),
            Some(self.egl_context),
        )
        .unwrap();
        self.gl_context.set_from(image, width.into(), height.into())
    }

    pub fn output_id(&self) -> u32 {
        self.output_id
    }

    pub fn draw(&self, process: f32) {
        egl.make_current(
            self.egl_display,
            Some(self.egl_surface),
            Some(self.egl_surface),
            Some(self.egl_context),
        )
        .unwrap();
        self.gl_context.draw(process);
        egl.swap_buffers(self.egl_display, self.egl_surface)
            .unwrap();
        self.surface.commit();
    }

    pub fn destroy(&self) {
        self.surface.destroy();
        egl.destroy_surface(self.egl_display, self.egl_surface)
            .expect("EGL surface could not be destroyed");
        egl.destroy_context(self.egl_display, self.egl_context)
            .expect("EGL context could not be destroyed");
        // self.gl_context.destroy();
    }
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

    let config = egl
        .choose_first_config(display, &attributes)
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

    let context = egl
        .create_context(display, config, None, &context_attributes)
        .expect("unable to create an EGL context");

    (context, config)
}
