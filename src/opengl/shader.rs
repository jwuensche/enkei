use gl::types::{
    GLenum,
    GLint,
};

use super::error::check_error;

pub struct Shader {
    id: u32,
    kind: GLenum,
}


const VSHADER: &[u8] = b"#version 150 core
in vec2 position;
in vec2 texcoord;

out vec2 Texcoord;

void main()
{
    Texcoord = texcoord;
    gl_Position = vec4(position, 0.0, 1.0);
}
\0";

const FRAGSHADER: &[u8] = b"#version 150 core
in vec2 Texcoord;

out vec4 outColor;

uniform sampler2D from;
uniform sampler2D to;
uniform float ratio;

void main()
{
    vec4 colFrom = texture(from, Texcoord);
    vec4 colTo = texture(to, Texcoord);
    outColor = mix(colFrom, colTo, ratio);
}
\0";

impl Shader {
    pub fn new_vertex() -> Self {
        unsafe {
            Self::new(VSHADER, gl::VERTEX_SHADER)
        }
    }

    pub fn new_fragment() -> Self {
        unsafe {
            Self::new(FRAGSHADER, gl::FRAGMENT_SHADER)
        }
    }

    unsafe fn new(src: &[u8], kind: GLenum) -> Self {
        let shader = gl::CreateShader(kind);
        let src = std::ffi::CStr::from_bytes_with_nul_unchecked(src).as_ptr();
        gl::ShaderSource(shader, 1, (&[src]).as_ptr(), std::ptr::null());
        gl::CompileShader(shader);
        let mut status = 0i32;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE.into() {
            eprintln!("Shader did not compile.");
            std::process::exit(1);
        }
        check_error("Shader Creation");
        Self {
            id: shader,
            kind,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
