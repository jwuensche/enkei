use super::outputs::Output;
use image::DynamicImage;
use khronos_egl::{
    Display as eglDisplay,
    Context as eglContext,
    Surface as eglSurface,
    Config as eglConfig,
};

use super::opengl::context::Context as glContext;

use wayland_client::{
    protocol::{wl_surface::WlSurface, wl_compositor::WlCompositor}, EventQueue,
    Main,
};
use super::egl;

use std::sync::{
    Arc,
    RwLock,
};

use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, Event as LayerEvent, ZwlrLayerSurfaceV1,
};

pub struct OutputRendering {
    output: Arc<RwLock<Output>>,
    surface: Main<WlSurface>,
    egl_context: eglContext,
    egl_display: eglDisplay,
    egl_surface: eglSurface,
    gl_context: glContext,
}

impl OutputRendering {
    pub fn new(
        compositor: &Main<WlCompositor>,
        layers: &Main<ZwlrLayerShellV1>,
        event_queue: &mut EventQueue,
        output: Arc<RwLock<Output>>,
        egl_context: eglContext,
        egl_display: eglDisplay,
        egl_config: eglConfig,
        buf_x: u32,
        buf_y: u32,
        image: &DynamicImage,
        image2: &DynamicImage,
    ) -> Self {
        let surface = compositor.create_surface();
        surface.commit();
        let lock = output.read().unwrap();
        let background =
            layers.get_layer_surface(&surface, Some(lock.inner()), Layer::Background, "wallpaper".into());
        background.set_layer(Layer::Background);
        background.set_anchor(Anchor::all());
        surface.commit();
        background.quick_assign(|layer, event, _| match event {
            LayerEvent::Configure {
                serial,
                width,
                height,
            } => {
                layer.ack_configure(serial);
            }
            _ => {}
        });
        surface.commit();
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
            .unwrap();

        let wl_egl_surface = wayland_egl::WlEglSurface::new(&surface, buf_x as i32, buf_y as i32);
        let egl_surface = unsafe { egl.create_window_surface(egl_display, egl_config, wl_egl_surface.ptr() as egl::NativeWindowType, None).unwrap() };
        egl.make_current(egl_display, Some(egl_surface), Some(egl_surface), Some(egl_context)).unwrap();
        surface.commit();
        // Rendering with the `gl` bindings are all unsafe let's block this away
        let context = super::opengl::context::Context::new(&mut image.as_rgb8().unwrap().as_raw().clone(), buf_x as i32, buf_y as i32);
        context.set_to(&mut image2.as_rgb8().unwrap().as_raw().clone(), buf_x as i32, buf_y as i32);
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
            surface,
            egl_context,
            egl_display,
            egl_surface,
            gl_context: context,
        }
    }
}


