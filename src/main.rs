#![warn(clippy::all)]

#[macro_use]
extern crate clap;

use bvh::Bvh;
use import::{Blender, Import};
use math::Vec3;
use rand::SeedableRng;
use scene::Geometry;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::{atomic, Arc};
use std::{fs, thread, time};
use tracing::raytrace;

mod bvh;
mod gui;
mod import;
mod math;
mod scene;
mod simd;
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
        (@arg INPUT: +required "file to render")
        (@arg OUTPUT: "file to write")
        (@arg headless: -H --headless "Do not show the GUI")
        (@arg threads: -t --threads +takes_value default_value(&cpu_count_str) "Number of worker threads")
        (@arg exposure: -e --exposure +takes_value default_value("0.0") "Exposure multiplier of the camera given as a power of two")
        (@arg width: -x --width +takes_value default_value("1600") "Image width in pixels")
        (@arg height: -y --height +takes_value default_value("900") "Image height in pixels")
        (@arg antialiasing: -a --antialiasing +takes_value default_value("1") "Number of samples (as a power of four) to use per pixel")
        (@arg seed: -s --seed +takes_value default_value("4103685768640310862782726084387274121") "Seed to use for random stuff")
    );
    let matches = clap_app.get_matches();
    let thread_count: usize = FromStr::from_str(matches.value_of("threads").unwrap()).unwrap();
    let window_w: usize = FromStr::from_str(matches.value_of("width").unwrap()).unwrap();
    let window_h: usize = FromStr::from_str(matches.value_of("height").unwrap()).unwrap();
    let exposure: f64 = FromStr::from_str(matches.value_of("exposure").unwrap()).unwrap();
    let antialiasing: u32 = FromStr::from_str(matches.value_of("antialiasing").unwrap()).unwrap();
    let seed: u128 = FromStr::from_str(matches.value_of("seed").unwrap()).unwrap();

    let scene = Arc::new({
        let start_time = time::Instant::now();

        let path = matches.value_of("INPUT").unwrap();

        let mut scene = if path.ends_with(".blend") {
            eprintln!("Starting Blender ...");
            let result = Command::new("blender")
                .args(&[path, "-b", "--log-level", "0", "-P", "blender_ray_exporter.py", "--"])
                .stderr(Stdio::null())
                .stdout(Stdio::piped())
                .stdin(Stdio::null())
                .output()
                .map_err(|e| format!("Could not execute blender: {}", e))?;
            eprintln!("Blender done.");
            if !result.status.success() {
                Err("Blender export did not exit successfully!".to_owned())
            } else {
                let json_text = String::from_utf8(result.stdout)
                    .map_err(|e| format!("Encoding error: {}", e))?;
                let json_text = &json_text[json_text.find('{').ok_or("Missing first { in JSON.")?
                    ..=json_text.rfind('}').ok_or("Missing last } in JSON.")?];
                Blender::new(&json_text, window_w, window_h)
                    .import()
                    .map_err(|e| format!("Error during Blender import: {}", e))
            }
        } else if path.ends_with(".blend.json") {
            let mut file_text = String::new();
            let mut infile = fs::File::open(path)
                .map_err(|e| format!("File {} cannot be opened: {}", path, e))?;
            infile
                .read_to_string(&mut file_text)
                .map_err(|e| format!("File {} cannot be read: {}", path, e))?;
            Blender::new(&file_text, window_w, window_h)
                .import()
                .map_err(|e| format!("Error during Blender JSON import: {}", e))
        } else {
            Err("Unknown input format.".to_owned())
        }?;

        let end_time = time::Instant::now();
        eprintln!("Parsing input file: {} ms", (end_time - start_time).as_millis());

        let start_time = time::Instant::now();
        let geometry = {
            let mut geometry = Vec::new();
            for t in &scene.triangles {
                geometry.push(Geometry::Triangle(*t));
            }
            for pl in &scene.point_lights {
                geometry.push(Geometry::PointLight(*pl));
            }
            geometry
        };
        let bvh = Bvh::new(&geometry);
        scene.triangles_bvh = Some(bvh);
        let end_time = time::Instant::now();
        eprintln!("Building BVH: {} ms", (end_time - start_time).as_millis());

        scene
    });

    let (pixel_sender, pixel_receiver) = crossbeam_channel::unbounded();
    let want_quit = Arc::new(atomic::AtomicBool::new(false));
    let (render_sender, render_receiver) = crossbeam_channel::unbounded();

    {
        let mut positions = vec![];
        for x in 0..window_w {
            for y in 0..window_h {
                for xaa in 0..2usize.pow(antialiasing) {
                    for yaa in 0..2usize.pow(antialiasing) {
                        positions.push(((x << antialiasing) + xaa, (y << antialiasing) + yaa));
                    }
                }
            }
        }
        positions.sort_by(|a, b| {
            let a_zeros = a.0.trailing_zeros().min(a.1.trailing_zeros());
            let b_zeros = b.0.trailing_zeros().min(b.1.trailing_zeros());
            if a_zeros > b_zeros {
                Ordering::Less
            } else if a_zeros < b_zeros {
                Ordering::Greater
            } else if a.0 < b.0 {
                Ordering::Less
            } else if a.0 > b.0 {
                Ordering::Greater
            } else if a.1 < b.1 {
                Ordering::Less
            } else if a.1 > b.1 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        assert_eq!(positions.len(), window_w * window_h * 4usize.pow(antialiasing));
        for p in positions {
            render_sender.send(p).unwrap();
        }
    }

    let window_thread = {
        let want_quit = Arc::clone(&want_quit);
        thread::Builder::new()
            .name("GUI".to_owned())
            .spawn(move || {
                gui::main_loop(window_w, window_h, exposure, pixel_receiver, &want_quit);
            })
            .unwrap()
    };

    let start_time = time::Instant::now();
    let mut worker_threads = Vec::with_capacity(thread_count);
    for t in 0..thread_count {
        let scene = Arc::clone(&scene);
        let want_quit = Arc::clone(&want_quit);
        let render_receiver = render_receiver.clone();
        let pixel_sender = pixel_sender.clone();
        let worker_thread = thread::Builder::new()
            .name(format!("Worker {}", t + 1))
            .spawn(move || {
                let mut rng = rand_pcg::Pcg32::from_seed(
                    seed.overflowing_mul(t as u128 + 123).0.to_be_bytes(),
                );
                while let Ok((my_x, my_y)) = render_receiver.try_recv() {
                    if want_quit.load(atomic::Ordering::Relaxed) {
                        break;
                    }

                    let (render_x, render_y) = if antialiasing == 0 {
                        // Use pixel center
                        (my_x as f64 + 0.5, my_y as f64 + 0.5)
                    } else {
                        // Use RGSS around the second-to-last (!!!) subpixel center

                        // First find the subpixel center
                        // pixel_left + subpixel_index * subpixel_size + subpixel_size / 2
                        // Hint: For x = 1 and aa = 1 this leads to 0.75.
                        //       For x = 0 and aa = 1 this leads to 0.25.
                        //       For x = 0 and aa = 2 this leads to 0.125.
                        //       For x = 1 and aa = 2 this leads to 0.25.
                        let subpixel_size = 1.0 / f64::from(1 << antialiasing);
                        let rgss_center_x = (my_x >> antialiasing) as f64
                            + (my_x & ((1 << antialiasing) - 1)) as f64 * subpixel_size
                            + subpixel_size / 2.0;
                        let rgss_center_y = (my_y >> antialiasing) as f64
                            + (my_y & ((1 << antialiasing) - 1)) as f64 * subpixel_size
                            + subpixel_size / 2.0;

                        // Pick one offset for each of the four remaining subpixels. Note that these
                        // offsets are relative to the subpixel center, *not* relative to the
                        // second-to-last subpixel center.
                        let (rgss_offset_x, rgss_offset_y) = [
                            (-1.0 / 8.0, 1.0 / 8.0),  // x % 2 == 0 && y % 2 == 0  =>  top-left
                            (-1.0 / 8.0, -1.0 / 8.0), // x % 2 == 1 && y % 2 == 0  =>  top-right
                            (1.0 / 8.0, 1.0 / 8.0),   // x % 2 == 0 && y % 2 == 1  =>  bottom-left
                            (1.0 / 8.0, -1.0 / 8.0),  // x % 2 == 1 && y % 2 == 1  =>  bottom-right
                        ][(my_x % 2) + 2 * (my_y % 2)];

                        // Divide the offsets to the correct subpixel size
                        let rgss_offset_x = rgss_offset_x / f64::from(1 << (antialiasing - 1));
                        let rgss_offset_y = rgss_offset_y / f64::from(1 << (antialiasing - 1));

                        (rgss_center_x + rgss_offset_x, rgss_center_y + rgss_offset_y)
                    };

                    let color = raytrace(
                        &scene,
                        &mut rng,
                        render_x,
                        render_y,
                        window_w as f64,
                        window_h as f64,
                    );
                    let color = color.unwrap_or(Vec3([0.0, 0.0, 0.0])).xyz1();

                    pixel_sender.send((my_x >> antialiasing, my_y >> antialiasing, color)).unwrap();
                }
            })
            .unwrap();
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
