use crate::math::Vec4;
use image::GenericImageView;
use std::fmt::{Debug, Formatter};

pub struct Image {
    w: usize,
    h: usize,
    content: Vec<Vec4>,
}

impl Image {
    pub fn from_path(path: &str) -> Result<Image, String> {
        let image = image::open(path)
            .map_err(|e| format!("Error while reading image {}: {}", path, e))?
            .flipv();

        let (w, h) = image.dimensions();
        let w = w as usize;
        let h = h as usize;
        let mut content = vec![Vec4([0.0; 4]); w * h];
        for x in 0..w {
            for y in 0..h {
                let p = image.get_pixel(x as u32, y as u32);
                content[w * y + x] = Vec4([
                    f64::from(p.0[0]) / 255.0,
                    f64::from(p.0[1]) / 255.0,
                    f64::from(p.0[2]) / 255.0,
                    f64::from(p.0[3]) / 255.0,
                ])
                .srgb_to_linear();
            }
        }

        Ok(Image { w, h, content })
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
