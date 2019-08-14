#[macro_use]
extern crate clap;

use quick_xml::Reader;
use quick_xml::events::Event;
use minifb::{Window, WindowOptions};
use std::{thread, time};

fn main() {
    let matches = clap_app!(photon =>
        (version: crate_version!())
        (author: crate_authors!("; "))
        (about: crate_description!())
        (@arg INPUT: +required "DAE file to render")
        (@arg OUTPUT: +required "PNG file to write")
        (@arg headless: -H --headless "Do not show the GUI")
    ).get_matches();

    let mut reader = Reader::from_file("examples/cube.dae").unwrap();
    reader.trim_text(true);
    let mut buffer = vec![];
    loop {
        match reader.read_event(&mut buffer) {
            Ok(Event::Start(start)) => {
                print!("<{}", String::from_utf8_lossy(start.local_name()));
                for attribute in start.attributes() {
                    let attribute = attribute.unwrap();
                    print!(" {}={}", String::from_utf8_lossy(attribute.key), attribute.unescape_and_decode_value(&mut reader).unwrap());
                }
                println!(">");
            },
            Ok(Event::Text(text)) => println!("{}", text.unescape_and_decode(&mut reader).unwrap()),
            Ok(Event::End(end)) => println!("<{}>", String::from_utf8_lossy(end.local_name())),
            Ok(Event::Eof) => break,
            Ok(_) => {},
            Err(e) => panic!("Error while reading XML {}", e),
        }
    }

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
