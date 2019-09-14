use crate::math::Vec4;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;

pub struct Image {
    w: usize,
    h: usize,
    content: Vec<Vec4>,
}

impl Image {
    pub fn from_path(path: &str) -> Result<Image, String> {
        let lower_path = path.to_lowercase();
        if lower_path.ends_with(".png") {
            let reader = BufReader::new(
                File::open(path).map_err(|e| format!("Error while reading PNG: {}", e))?,
            );
            let decoder = png::Decoder::new(reader);
            let (info, mut reader) =
                decoder.read_info().map_err(|e| format!("Error while reading PNG: {}", e))?;
            let mut buffer = vec![0; info.buffer_size()];
            reader
                .next_frame(&mut buffer)
                .map_err(|e| format!("Error while reading PNG: {}", e))?;

            let w = info.width as usize;
            let h = info.height as usize;
            let mut content = vec![Vec4([0.0; 4]); w * h];
            for x in 0..w {
                for y in 0..h {
                    content[w * (h - y - 1) + x] = Vec4([
                        f64::from(buffer[(w * y + x) * 3]) / 255.0,
                        f64::from(buffer[(w * y + x) * 3 + 1]) / 255.0,
                        f64::from(buffer[(w * y + x) * 3 + 2]) / 255.0,
                        1.0,
                    ]);
                }
            }
            Ok(Image { w, h, content })
        } else if lower_path.ends_with(".jpg") || lower_path.ends_with(".jpeg") {
            let reader = BufReader::new(
                File::open(path).map_err(|e| format!("Error while reading JPEG: {}", e))?,
            );
            let mut decoder = jpeg_decoder::Decoder::new(reader);
            let pixels =
                decoder.decode().map_err(|e| format!("Error while reading JPEG: {}", e))?;
            let metadata = decoder.info().ok_or("Error while reading JPEG metadata")?;

            let w = metadata.width as usize;
            let h = metadata.height as usize;
            let mut content = vec![Vec4([0.0; 4]); w * h];
            for x in 0..w {
                for y in 0..h {
                    content[w * (h - y - 1) + x] = Vec4([
                        f64::from(pixels[(w * y + x) * 3]) / 255.0,
                        f64::from(pixels[(w * y + x) * 3 + 1]) / 255.0,
                        f64::from(pixels[(w * y + x) * 3 + 2]) / 255.0,
                        1.0,
                    ]);
                }
            }
            Ok(Image { w, h, content })
        } else {
            Err("Unsupported image file type".to_owned())
        }
    }

    pub fn w(&self) -> usize {
        self.w
    }

    pub fn h(&self) -> usize {
        self.h
    }

    pub fn get(&self, x: usize, y: usize) -> Vec4 {
        self.content[self.w * y + x]
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image {{ w: {}, h: {}, .. }}", self.w, self.h)
    }
}
