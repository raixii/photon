use crate::image_buffer::ImageBuffer;
use crate::math::Vec4;
use gl::types::*;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::video::SwapInterval;
use std::ffi::c_void;
use std::mem::size_of_val;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Mutex;

const VERTEX_SHADER: &str = r#"
    #version 320 es
    in vec2 in_pos;
    out vec2 out_pos;
    void main() {
        out_pos = in_pos;
        gl_Position = vec4(in_pos, 0.0, 1.0);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 320 es
    #extension GL_ARB_explicit_uniform_location : enable
    in highp vec2 out_pos;
    out highp vec4 color;
    layout(location = 0) uniform sampler2D tex;
    void main() {
        color = vec4(texture(tex, (out_pos + vec2(1.0, 1.0)) * vec2(0.5, -0.5)).xyz, 1.0);
    }
"#;

const QUAD: &[f32] = &[-1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0];

pub fn main_loop(
    window_w: usize,
    window_h: usize,
    mut exposure: f64,
    image: &Mutex<ImageBuffer>,
    want_quit: &AtomicBool,
) {
    let mut float_buffer = vec![Vec4([0.0; 4]); window_w * window_h];
    let mut display_buffer = vec![0u8; window_w * window_h * 3];
    let mut buffer_version = std::usize::MAX;
    let mut buffer_changed = true;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut window = video_subsystem
        .window(&format!("Photon: exposure={:+.1}", exposure), window_w as u32, window_h as u32)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let _gl_context = window.gl_create_context().unwrap();
    video_subsystem.gl_set_swap_interval(SwapInterval::VSync).unwrap();
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::ffi::c_void);

    let vertex_shader = unsafe {
        let shader = gl::CreateShader(gl::VERTEX_SHADER);
        let source_ptr = VERTEX_SHADER.as_ptr() as *const GLchar;
        let source_len = VERTEX_SHADER.len() as GLint;
        gl::ShaderSource(shader, 1, &source_ptr, &source_len);
        gl::CompileShader(shader);
        let mut result = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut result);
        if result != 1 {
            let mut buf = vec![0u8; 10000];
            gl::GetShaderInfoLog(
                shader,
                buf.len() as GLsizei,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!("GLSL output: {}", String::from_utf8_lossy(&buf[..]));
        }
        shader
    };

    let fragment_shader = unsafe {
        let shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        let source_ptr = FRAGMENT_SHADER.as_ptr() as *const GLchar;
        let source_len = FRAGMENT_SHADER.len() as GLint;
        gl::ShaderSource(shader, 1, &source_ptr, &source_len);
        gl::CompileShader(shader);
        let mut result = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut result);
        if result != 1 {
            let mut buf = vec![0u8; 10000];
            gl::GetShaderInfoLog(
                shader,
                buf.len() as GLsizei,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!("GLSL output: {}", String::from_utf8_lossy(&buf[..]));
        }
        shader
    };

    let program = unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex_shader);
        gl::AttachShader(program, fragment_shader);
        gl::LinkProgram(program);
        let mut result = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut result);
        if result != 1 {
            let mut buf = vec![0u8; 10000];
            gl::GetProgramInfoLog(
                program,
                buf.len() as GLsizei,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!("GLSL output: {}", String::from_utf8_lossy(&buf[..]));
        }
        program
    };

    let buffer = unsafe {
        let mut buffer = 0;
        gl::GenBuffers(1, &mut buffer);
        gl::BindBuffer(gl::ARRAY_BUFFER, buffer);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (QUAD.len() * size_of_val(&QUAD[0])) as GLsizeiptr,
            QUAD.as_ptr() as *const c_void,
            gl::STATIC_DRAW,
        );
        buffer
    };

    let _vao = unsafe {
        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, buffer);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
        gl::EnableVertexArrayAttrib(vao, 0);
        vao
    };

    let _texture = unsafe {
        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as GLint,
            window_w as GLsizei,
            window_h as GLsizei,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            display_buffer.as_ptr() as *const c_void,
        );
        texture
    };

    unsafe {
        gl::UseProgram(program);
        gl::Uniform1i(0, 0);
    }

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                }
                Event::KeyDown { keycode: Some(Keycode::F3), keymod, .. } => {
                    exposure -=
                        if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                            0.1
                        } else {
                            1.0
                        };
                    buffer_changed = true;
                    window.set_title(&format!("Photon: exposure={:+.1}", exposure)).unwrap();
                }
                Event::KeyDown { keycode: Some(Keycode::F4), keymod, .. } => {
                    exposure +=
                        if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                            0.1
                        } else {
                            1.0
                        };
                    buffer_changed = true;
                    window.set_title(&format!("Photon: exposure={:+.1}", exposure)).unwrap();
                }
                _ => {}
            }
        }

        {
            let image = image.lock().unwrap();
            if buffer_version != image.version() {
                float_buffer.copy_from_slice(image.get_buffer());
                buffer_version = image.version();
                buffer_changed = true;
            }
        }

        if buffer_changed {
            for (j, color) in float_buffer.iter().enumerate() {
                let mut c = color.xyz();
                for i in 0..3 {
                    c.0[i] *= 2.0f64.powf(exposure); // exposure
                }
                let max_color = c.x().max(c.y()).max(c.z());
                for i in 0..3 {
                    c.0[i] /= 1.0 + max_color; // tone mapping (Reinhard)
                    c.0[i] = c.0[i].min(1.0).max(0.0); // clamp between [0; 1]
                    c.0[i] = c.0[i].powf(2.2); // gamma correction
                    display_buffer[j * 3 + i] = (c.0[i] * 255.0) as u8; // machine numbers
                }
            }

            unsafe {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGB as GLint,
                    window_w as GLsizei,
                    window_h as GLsizei,
                    0,
                    gl::RGB,
                    gl::UNSIGNED_BYTE,
                    display_buffer.as_ptr() as *const c_void,
                );
            }

            buffer_changed = false;
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawArrays(gl::TRIANGLES, 0, QUAD.len() as GLsizei);
        }
        window.gl_swap_window();
    }

    want_quit.store(true, Relaxed);
}
