#![warn(clippy::all)]

#[macro_use]
extern crate clap;

use import::{Blender, Import};
use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::{atomic, Arc};
use std::{fs, thread, time};

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

        let scene = if path.ends_with(".blend") {
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
                Blender::new(
                    Path::new(path)
                        .parent()
                        .ok_or("Cannot get parent directory")?
                        .to_str()
                        .ok_or("Path contains invalid characters")?,
                    &json_text,
                    window_w,
                    window_h,
                )
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
            Blender::new(
                Path::new(path)
                    .parent()
                    .ok_or("Cannot get parent directory")?
                    .to_str()
                    .ok_or("Path contains invalid characters")?,
                &file_text,
                window_w,
                window_h,
            )
            .import()
            .map_err(|e| format!("Error during Blender JSON import: {}", e))
        } else {
            Err("Unknown input format.".to_owned())
        }?;

        let end_time = time::Instant::now();
        eprintln!("Parsing input file: {} ms", (end_time - start_time).as_millis());

        scene
    });

    let (pixel_sender, pixel_receiver) = crossbeam_channel::unbounded();
    let want_quit = Arc::new(atomic::AtomicBool::new(false));

    let window_thread = {
        let want_quit = Arc::clone(&want_quit);
        thread::Builder::new()
            .name("GUI".to_owned())
            .spawn(move || {
                gui::main_loop(window_w, window_h, exposure, pixel_receiver, &want_quit);
            })
            .unwrap()
    };

    tracing::main(
        scene,
        antialiasing,
        window_w,
        window_h,
        thread_count,
        seed,
        want_quit,
        pixel_sender,
    );

    window_thread.join().unwrap();
    Ok(())
}
