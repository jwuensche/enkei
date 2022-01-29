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

use super::error::check_error;
use super::shader::Shader;
use log::debug;

#[derive(Debug)]
pub struct Context {
    _vao: u32,
    _vbo: u32,
    _ebo: u32,
    tex_from: u32,
    tex_to: u32,
    shader_program: Program,
}

#[derive(Debug)]
pub struct Program {
    id: u32,
    _vertex_shader: Shader,
    _fragment_shader: Shader,
}

impl Program {
    pub fn new() -> Self {
        let vertex_shader = Shader::new_vertex();
        let fragment_shader = Shader::new_fragment();

        unsafe {
            let shader_program = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex_shader.id());
            gl::AttachShader(shader_program, fragment_shader.id());
            check_error("Attach Shader to Program");
            // Link Program Parameters
            let out_color = std::ffi::CStr::from_bytes_with_nul_unchecked(b"outColor\0");
            gl::BindFragDataLocation(shader_program, 0, out_color.as_ptr());
            check_error("Bind Fragement Data Location");
            gl::LinkProgram(shader_program);
            check_error("Link Program");
            gl::UseProgram(shader_program);
            check_error("Use Program");
            let program = Self {
                id: shader_program,
                _vertex_shader: vertex_shader,
                _fragment_shader: fragment_shader,
            };
            program.link_arguments();
            program
        }
    }

    fn link_arguments(&self) {
        debug!("Linking \"position\" argument");
        unsafe {
            let pos = std::ffi::CStr::from_bytes_with_nul_unchecked(b"position\0");
            let pos_attrib = gl::GetAttribLocation(self.id, pos.as_ptr());
            check_error("pre call");
            gl::EnableVertexAttribArray(pos_attrib as u32);
            check_error("Argument linking");
            gl::VertexAttribPointer(
                pos_attrib as u32,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as gl::types::GLint,
                std::ptr::null(),
            );
            check_error("shady call");
        }
        debug!("Linking \"texcoord\" argument");
        unsafe {
            let texture = std::ffi::CStr::from_bytes_with_nul_unchecked(b"texcoord\0");
            let tex_attrib = gl::GetAttribLocation(self.id, texture.as_ptr());
            check_error("pre call");
            gl::EnableVertexAttribArray(tex_attrib as u32);
            check_error("TexCoord Argument Linking");
            gl::VertexAttribPointer(
                tex_attrib as u32,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as gl::types::GLint,
                (2 * std::mem::size_of::<f32>()) as *const gl::types::GLvoid,
            );
            check_error("after tex attrib");
        }
        debug!("Done Linking.");
    }
}

impl Context {
    pub fn new() -> Self {
        let vertices: [f32; 16] = [
            // Positions    // TexCoords
            -1.0, 1.0, 0.0, 0.0, // top-left
            1.0, 1.0, 1.0, 0.0, // top-right
            1.0, -1.0, 1.0, 1.0, // bottom-right
            -1.0, -1.0, 0.0, 1.0, // bottom-left
        ];

        let elements: [i32; 6] = [
            0, 1, 2, // upper right
            0, 2, 3, // lower left
        ];

        let mut vao = 0u32;
        let mut vbo = 0u32;
        let mut ebo = 0u32;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(&vertices) as isize,
                vertices.as_ptr() as *const std::ffi::c_void,
                gl::STATIC_DRAW,
            );

            // Create Element Buffer
            gl::GenBuffers(1, &mut ebo);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                std::mem::size_of_val(&elements) as isize,
                elements.as_ptr() as *const std::ffi::c_void,
                gl::STATIC_DRAW,
            );

            check_error("Buffer Creation");
        }

        let program = Program::new();

        // SETUP THE TEXTURE TO BE USED & DRAW THE SCREEN THE INITIALLY
        let mut tex_from = 0u32;
        let mut tex_to = 0u32;
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::GenTextures(1, &mut tex_from);
            let name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"from\0");
            let from_location = gl::GetUniformLocation(program.id, name.as_ptr());
            gl::Uniform1i(from_location, 0);

            gl::ActiveTexture(gl::TEXTURE1);
            gl::GenTextures(1, &mut tex_to);
            let name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"to\0");
            let to_location = gl::GetUniformLocation(program.id, name.as_ptr());
            gl::Uniform1i(to_location, 1);
        }
        Self {
            _vao: vao,
            _ebo: ebo,
            _vbo: vbo,
            tex_from,
            tex_to,
            shader_program: program,
        }
    }

    pub fn set_from(&self, pic: &[u8], width: i32, height: i32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            self.set_image(pic, width, height, self.tex_from)
        }
    }

    pub fn set_to(&self, pic: &[u8], width: i32, height: i32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE1);
            self.set_image(pic, width, height, self.tex_to)
        }
    }

    unsafe fn set_image(&self, pic: &[u8], width: i32, height: i32, tex_id: u32) {
        gl::BindTexture(gl::TEXTURE_2D, tex_id);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            width,
            height,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            pic.as_ptr() as *const gl::types::GLvoid,
        );
        check_error("image load");
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        check_error("set mag filter");
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S,
            gl::CLAMP_TO_BORDER as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T,
            gl::CLAMP_TO_BORDER as i32,
        );
        check_error("set border");
    }

    pub fn draw(&self, ratio: f32) {
        unsafe {
            let name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"ratio\0");
            let ratio_location = gl::GetUniformLocation(self.shader_program.id, name.as_ptr());
            gl::Uniform1f(ratio_location, ratio);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            check_error("Drawing");
        }
    }
}
