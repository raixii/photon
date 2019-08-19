use crate::image_buffer::ImageBuffer;
use crate::math::Vec4;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::{sync::Mutex, thread, time};

extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::PixelFormatEnum;

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

    let window = video_subsystem
        .window(&format!("Photon: exposure={:+.1}", exposure), window_w as u32, window_h as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, window_w as u32, window_h as u32)
        .unwrap();

    texture
        .with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..window_h {
                for x in 0..window_w {
                    let offset = y * pitch + x * 3;
                    buffer[offset] = 255 as u8;
                    buffer[offset + 1] = 255 as u8;
                    buffer[offset + 2] = 255;
                }
            }
        })
        .unwrap();

    canvas.copy(&texture, None, None).unwrap();
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
                    canvas
                        .window_mut()
                        .set_title(&format!("Photon: exposure={:+.1}", exposure))
                        .unwrap();
                }
                Event::KeyDown { keycode: Some(Keycode::F4), keymod, .. } => {
                    exposure +=
                        if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                            0.1
                        } else {
                            1.0
                        };
                    buffer_changed = true;
                    canvas
                        .window_mut()
                        .set_title(&format!("Photon: exposure={:+.1}", exposure))
                        .unwrap();
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
            texture.update(None, &display_buffer, window_w * 3).unwrap();
            buffer_changed = false;
        }
        // The rest of the game loop goes here...
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        thread::sleep(time::Duration::from_millis(50));
    }
    want_quit.store(true, Relaxed);
}
