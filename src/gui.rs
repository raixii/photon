use crate::math::Vec4;
use gl::types::*;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::video::{GLProfile, SwapInterval};
use std::ffi::c_void;
use std::mem::size_of_val;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

const VERTEX_SHADER: &str = r#"
    #version 330

    in vec2 in_pos;

    void main() {
        gl_Position = vec4(in_pos, 0.0, 1.0);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330
    #extension GL_ARB_explicit_uniform_location : enable

    out vec4 out_color;

    layout(location = 0) uniform sampler2D tex;
    layout(location = 1) uniform float exposure;

    void main() {
        ivec2 resolution = textureSize(tex, 0);
        ivec2 pixel = ivec2(gl_FragCoord.x, resolution.y - int(gl_FragCoord.y) - 1);

        vec4 colora = vec4(0.0);
        for (int power_of_two = 0;; ++power_of_two) {
            // t = floor(p / 2^i) * 2^i
            ivec2 tex_pixel = (pixel >> ivec2(power_of_two)) << ivec2(power_of_two);
            colora = texelFetch(tex, tex_pixel, 0);
            if (colora.a != 0.0 || tex_pixel == ivec2(0, 0)) {
                break;
            }
        }

        vec3 color = colora.xyz;
        color = color * exp(exposure); // exposure
        color = color / vec3(1.0 + max(color.x, max(color.y, color.z))); // tone mapping (Reinhard)        
        // gamma correction is enabled in the framebuffer

        out_color = vec4(color, 1.0);
    }
"#;

const QUAD: &[f32] = &[-1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0];

pub fn main_loop(
    window_w: usize,
    window_h: usize,
    exposure: f64,
    receiver: crossbeam_channel::Receiver<(usize, usize, Vec4)>,
    want_quit: &AtomicBool,
) {
    let mut exposure = exposure as f32;
    let mut display_buffer = vec![0.0f32; window_w * window_h * 4];
    let mut buffer_changed = true;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_flags().forward_compatible().set();
    gl_attr.set_framebuffer_srgb_compatible(true);
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
            gl::RGBA32F as GLint,
            window_w as GLsizei,
            window_h as GLsizei,
            0,
            gl::RGBA,
            gl::FLOAT,
            display_buffer.as_ptr() as *const c_void,
        );
        texture
    };

    unsafe {
        gl::Enable(gl::FRAMEBUFFER_SRGB);
        gl::UseProgram(program);
        gl::Uniform1i(0, 0);
        gl::Uniform1f(1, exposure);
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
                    unsafe {
                        gl::Uniform1f(1, exposure);
                    }
                    window.set_title(&format!("Photon: exposure={:+.1}", exposure)).unwrap();
                }
                Event::KeyDown { keycode: Some(Keycode::F4), keymod, .. } => {
                    exposure +=
                        if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                            0.1
                        } else {
                            1.0
                        };
                    unsafe {
                        gl::Uniform1f(1, exposure);
                    }
                    window.set_title(&format!("Photon: exposure={:+.1}", exposure)).unwrap();
                }
                _ => {}
            }
        }

        while let Ok((x, y, Vec4([r, g, b, _a]))) = receiver.try_recv() {
            buffer_changed = true;
            display_buffer[(y * window_w + x) * 4] = r as f32;
            display_buffer[(y * window_w + x) * 4 + 1] = g as f32;
            display_buffer[(y * window_w + x) * 4 + 2] = b as f32;
            display_buffer[(y * window_w + x) * 4 + 3] = 1.0;
        }
        if buffer_changed {
            unsafe {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA32F as GLint,
                    window_w as GLsizei,
                    window_h as GLsizei,
                    0,
                    gl::RGBA,
                    gl::FLOAT,
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
