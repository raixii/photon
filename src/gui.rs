use crate::image_buffer::ImageBuffer;
use crate::math::Vec4;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::{sync::Mutex, thread, time};

pub fn main_loop(
    window_w: usize,
    window_h: usize,
    mut exposure: f64,
    image: &Mutex<ImageBuffer>,
    want_quit: &AtomicBool,
) {
    let mut float_buffer = vec![Vec4([0.0; 4]); window_w * window_h];
    let mut display_buffer = vec![0; window_w * window_h];
    let mut buffer_version = std::usize::MAX;
    let mut buffer_changed = true;

    let mut window = Window::new("Photon", window_w, window_h, WindowOptions::default()).unwrap();
    while window.is_open() {
        {
            let image = image.lock().unwrap();
            if buffer_version != image.version() {
                float_buffer.copy_from_slice(image.get_buffer());
                buffer_version = image.version();
                buffer_changed = true;
            }
        }

        if let Some(pressed_keys) = window.get_keys_pressed(KeyRepeat::No) {
            let shift_down =
                window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
            for pressed_key in pressed_keys {
                match pressed_key {
                    Key::F3 => {
                        exposure -= if shift_down { 0.1 } else { 1.0 };
                        buffer_changed = true;
                    }
                    Key::F4 => {
                        exposure += if shift_down { 0.1 } else { 1.0 };
                        buffer_changed = true;
                    }
                    _ => {}
                }
            }
        }

        if buffer_changed {
            for (i, color) in float_buffer.iter().enumerate() {
                let mut c = color.xyz();
                for i in 0..3 {
                    c.0[i] *= 2.0f64.powf(exposure); // exposure
                }
                let max_color = c.x().max(c.y()).max(c.z());
                for i in 0..3 {
                    c.0[i] /= 1.0 + max_color; // tone mapping (Reinhard)
                    c.0[i] = c.0[i].min(1.0).max(0.0); // clamp between [0; 1]
                    c.0[i] = c.0[i].powf(2.2); // gamma correction
                    c.0[i] *= 255.0; // machine numbers
                }
                display_buffer[i] =
                    ((c.0[0] as u32) << 16) | ((c.0[1] as u32) << 8) | (c.0[2] as u32);
            }
            window.update_with_buffer(&display_buffer).unwrap();
            window.set_title(&format!("Photon: exposure={:+.1}", exposure));
            buffer_changed = false;
        } else {
            window.update();
        }

        thread::sleep(time::Duration::from_millis(16));
    }

    want_quit.store(true, Relaxed);
}
