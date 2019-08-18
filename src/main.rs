#![warn(clippy::all)]

#[macro_use]
extern crate clap;

use bvh::Bvh;
use image_buffer::ImageBuffer;
use import::collada::Collada;
use import::Import;
use std::fmt::{Debug, Formatter};
use std::{fs, io::Read, str::FromStr, sync::atomic, sync::Arc, sync::Mutex, thread, time};
use tracing::raytrace;

mod bvh;
mod gui;
mod image_buffer;
mod import;
mod math;
mod scene;
mod tracing;

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
    let cpu_count_str = format!("{}", num_cpus::get());
    let clap_app = clap_app!(photon =>
        (version: crate_version!())
        (author: crate_authors!("; "))
        (about: crate_description!())
        (@arg INPUT: +required "DAE file to render")
        (@arg OUTPUT: +required "PNG file to write")
        (@arg headless: -H --headless "Do not show the GUI")
        (@arg threads: -t --threads +takes_value default_value(&cpu_count_str) "Number of worker threads")
        (@arg exposure: -e --exposure +takes_value default_value("0.0") "Exposure multiplier of the camera given as a power of two")
        (@arg width: -x --width +takes_value default_value("1600") "Image width in pixels")
        (@arg height: -y --height +takes_value default_value("900") "Image height in pixels")
    );
    let matches = clap_app.get_matches();
    let thread_count = FromStr::from_str(matches.value_of("threads").unwrap()).unwrap();
    let window_w = FromStr::from_str(matches.value_of("width").unwrap()).unwrap();
    let window_h = FromStr::from_str(matches.value_of("height").unwrap()).unwrap();
    let exposure = FromStr::from_str(matches.value_of("exposure").unwrap()).unwrap();

    let scene = Arc::new({
        let start_time = time::Instant::now();
        let path = matches.value_of("INPUT").unwrap();
        let mut infile =
            fs::File::open(path).map_err(|e| format!("File {} cannot be opened: {}", path, e))?;
        let mut buffer = Vec::new();
        infile
            .read_to_end(&mut buffer)
            .map_err(|e| format!("File {} cannot be read: {}", path, e))?;
        let collada_xml = String::from_utf8(buffer).map_err(|e| format!("{}", e))?;
        let mut scene = Collada { xml: collada_xml }
            .import()
            .map_err(|e| format!("Error during Collada import: {}", e))?;
        let end_time = time::Instant::now();
        eprintln!("Parsing COLLADA: {} ms", (end_time - start_time).as_millis());

        let start_time = time::Instant::now();
        let bvh = Bvh::new(&scene.triangles);
        scene.triangles_bvh = Some(bvh);
        let end_time = time::Instant::now();
        eprintln!("Building BVH: {} ms", (end_time - start_time).as_millis());

        scene
    });

    let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(window_w, window_h)));
    let want_quit = Arc::new(atomic::AtomicBool::new(false));
    let pixel_at = Arc::new(atomic::AtomicUsize::new(0));

    let window_thread = {
        let image_buffer = Arc::clone(&image_buffer);
        let want_quit = Arc::clone(&want_quit);
        thread::spawn(move || {
            gui::main_loop(window_w, window_h, exposure, &image_buffer, &want_quit);
        })
    };

    let start_time = time::Instant::now();
    let mut worker_threads = Vec::with_capacity(thread_count);
    for _t in 0..thread_count {
        let scene = Arc::clone(&scene);
        let want_quit = Arc::clone(&want_quit);
        let pixel_at = Arc::clone(&pixel_at);
        let image_buffer = Arc::clone(&image_buffer);
        let worker_thread = thread::spawn(move || {
            while !want_quit.load(atomic::Ordering::Relaxed) {
                let my_pixel = pixel_at.fetch_add(1, atomic::Ordering::Relaxed);
                let (my_x, my_y) = (my_pixel % window_w, my_pixel / window_w);
                if my_y >= window_h {
                    break;
                }
                let color =
                    raytrace(&scene, my_x as f64, my_y as f64, window_w as f64, window_h as f64);
                if let Some(color) = color {
                    let mut buffer = image_buffer.lock().unwrap();
                    buffer.set_pixel(my_x, my_y, color.xyz1());
                }
            }
        });
        worker_threads.push(worker_thread);
    }
    for worker_thread in worker_threads {
        worker_thread.join().unwrap();
    }
    let end_time = time::Instant::now();
    eprintln!("Raytracing: {} ms", (end_time - start_time).as_millis());

    window_thread.join().unwrap();

    Ok(())
}
