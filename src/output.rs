use super::outputs::Output;
use khronos_egl::{
    Display as eglDisplay,
    Context as eglContext,
    Surface as eglSurface,
};

use super::opengl::context::Context as glContext;

use wayland_client::{
    protocol::wl_surface::WlSurface,
};

use std::sync::{
    Arc,
    RwLock,
};

pub struct OutputRendering {
    output: Arc<RwLock<Output>>,
    surface: WlSurface,
    egl_context: eglContext,
    egl_display: eglDisplay,
    egl_surface: eglSurface,
    gl_context: glContext,
}
