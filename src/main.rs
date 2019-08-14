#[macro_use]
extern crate clap;

use minifb::{Window, WindowOptions};
use std::{thread, time, fs};
use std::io::Read;

mod collada;
mod scene;
mod math;

fn main() {
    let matches = clap_app!(photon =>
        (version: crate_version!())
        (author: crate_authors!("; "))
        (about: crate_description!())
        (@arg INPUT: +required "DAE file to render")
        (@arg OUTPUT: +required "PNG file to write")
        (@arg headless: -H --headless "Do not show the GUI")
    ).get_matches();

    let collada_xml = {
        let mut infile = fs::File::open(matches.value_of("INPUT").unwrap()).unwrap();
        let mut buffer = Vec::new();
        infile.read_to_end(&mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    };
    collada::read(&collada_xml);

    let window_w = 1024;
    let window_h = 768;
    let mut buffer = vec![0; window_w * window_h];
    for x in 0..1024 {
        for y in 0..768 {
            buffer[y * window_w + x] = if (x / 32) % 2 == (y / 32) % 2 { 0xFFFFFF } else { 0x000000 }
        }
    }
    
    let mut window = Window::new("Photon", window_w, window_h, WindowOptions::default()).unwrap();
    while window.is_open() {
        window.update_with_buffer(&buffer).unwrap();
        thread::sleep(time::Duration::from_millis(250));
    }
}
