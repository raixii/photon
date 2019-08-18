#![warn(clippy::all)]

#[macro_use]
extern crate clap;

use bvh::Bvh;
use minifb::{Window, WindowOptions};
use scene::Scene;
use std::fmt::{Debug, Formatter};
use std::{fs, io::Read, str::FromStr, sync::atomic, sync::mpsc, sync::Arc, thread, time};
use tracing::raytrace;

mod bvh;
mod collada;
mod math;
mod scene;
mod tracing;

struct ThreadData {
    pub scene: Scene,
    pub pixel_at: atomic::AtomicUsize,
    pub want_quit: atomic::AtomicBool,
    pub max_thread_time: atomic::AtomicU64,
    pub running_threads: atomic::AtomicUsize,
}

struct ErrorMessage(String);

impl Debug for ErrorMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ErrorMessage {
    fn from(error: String) -> Self {
        ErrorMessage(error)
    }
}

impl From<&str> for ErrorMessage {
    fn from(error: &str) -> Self {
        ErrorMessage(String::from(error))
    }
}

fn main() -> Result<(), ErrorMessage> {
    let clap_app = clap_app!(photon =>
        (version: crate_version!())
        (author: crate_authors!("; "))
        (about: crate_description!())
        (@arg INPUT: +required "DAE file to render")
        (@arg OUTPUT: +required "PNG file to write")
        (@arg headless: -H --headless "Do not show the GUI")
        (@arg threads: -t --threads +takes_value "Number of worker threads")
    );
    let matches = clap_app.get_matches();
    let thread_count = if let Some(tc) = matches.value_of("threads") {
        let tc = FromStr::from_str(tc).map_err(|_| "--threads expects a number.")?;
        if tc < 1 {
            Err("--threads must be at least 1.")?;
        }
        tc
    } else {
        num_cpus::get()
    };
    let window_w = 1600;
    let window_h = 900;

    let scene = {
        let start_time = time::Instant::now();
        let path = matches.value_of("INPUT").unwrap();
        let mut infile =
            fs::File::open(path).map_err(|e| format!("File {} cannot be opened: {}", path, e))?;
        let mut buffer = Vec::new();
        infile
            .read_to_end(&mut buffer)
            .map_err(|e| format!("File {} cannot be read: {}", path, e))?;
        let collada_xml = String::from_utf8_lossy(&buffer);
        let mut scene = collada::read(&collada_xml);
        let end_time = time::Instant::now();
        eprintln!("Parsing COLLADA: {} ms", (end_time - start_time).as_millis());

        let start_time = time::Instant::now();
        let bvh = Bvh::new(&scene.triangles);
        scene.triangles_bvh = Some(bvh);
        let end_time = time::Instant::now();
        eprintln!("Building BVH: {} ms", (end_time - start_time).as_millis());

        scene
    };

    let (sender, receiver) = mpsc::channel();
    let thread_data = Arc::new(ThreadData {
        scene,
        pixel_at: atomic::AtomicUsize::new(0),
        want_quit: atomic::AtomicBool::new(false),
        max_thread_time: atomic::AtomicU64::new(0),
        running_threads: atomic::AtomicUsize::new(thread_count),
    });
    let join_handles: Vec<_> = (0..thread_count)
        .map(|_| {
            let my_thread_data = Arc::clone(&thread_data);
            let my_sender = sender.clone();
            thread::spawn(move || {
                let start_time = time::Instant::now();
                while !my_thread_data.want_quit.load(atomic::Ordering::Relaxed) {
                    let my_pixel = my_thread_data.pixel_at.fetch_add(1, atomic::Ordering::Relaxed);
                    let (my_x, my_y) = (my_pixel % window_w, my_pixel / window_w);
                    if my_y >= window_h {
                        break;
                    }
                    let color = raytrace(
                        &my_thread_data.scene,
                        my_x as f64,
                        my_y as f64,
                        window_w as f64,
                        window_h as f64,
                    );
                    if let Some(color) = color {
                        my_sender.send((my_x, my_y, color)).unwrap();
                    }
                }
                let end_time = time::Instant::now();

                let mut stored_time =
                    my_thread_data.max_thread_time.load(atomic::Ordering::Relaxed);
                loop {
                    match my_thread_data.max_thread_time.compare_exchange_weak(
                        stored_time,
                        stored_time.max((end_time - start_time).as_millis() as u64),
                        atomic::Ordering::Relaxed,
                        atomic::Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(x) => stored_time = x,
                    }
                }

                my_thread_data.running_threads.fetch_sub(1, atomic::Ordering::Relaxed);
            })
        })
        .collect();

    let mut buffer = vec![0; window_w * window_h];
    for x in 0..window_w {
        for y in 0..window_h {
            buffer[y * window_w + x] =
                if (x / 32) % 2 == (y / 32) % 2 { 0xFF_FF_FF } else { 0xEE_EE_EE }
        }
    }
    let mut window = Window::new("Photon", window_w, window_h, WindowOptions::default())
        .map_err(|_| "Cannot open the window.")?;
    let mut printed_time = false;
    while window.is_open() {
        for (x, y, mut color) in receiver.try_iter() {
            for i in 0..3 {
                // color.0[i] = (color.0[i] / (1.0 + color.0[i])).powf(2.2);
                color.0[i] /= 1000.0;
            }
            buffer[y * window_w + x] = (((color.x() * 255.0) as u32) << 16)
                | (((color.y() * 255.0) as u32) << 8)
                | ((color.z() * 255.0) as u32);
        }
        window.update_with_buffer(&buffer).map_err(|_| "Cannot update the window.")?;
        thread::sleep(time::Duration::from_millis(250));

        if !printed_time && thread_data.running_threads.load(atomic::Ordering::Relaxed) == 0 {
            eprintln!(
                "Raytracing: {} ms",
                thread_data.max_thread_time.load(atomic::Ordering::Relaxed)
            );
            printed_time = true;
        }
    }

    thread_data.want_quit.store(true, atomic::Ordering::Relaxed);
    for join_handle in join_handles {
        join_handle.join().unwrap();
    }
    Ok(())
}
