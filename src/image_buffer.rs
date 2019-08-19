use crate::math::Vec4;

pub struct ImageBuffer {
    w: usize,
    buffer: Vec<Vec4>,
    version: usize,
}

impl ImageBuffer {
    pub fn new(w: usize, h: usize) -> ImageBuffer {
        ImageBuffer { w, buffer: vec![Vec4([0.0; 4]); w * h], version: 0 }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Vec4) {
        self.buffer[y * self.w + x] = color;
        self.version += 1;
    }

    pub fn get_buffer(&self) -> &[Vec4] {
        &self.buffer
    }

    pub fn version(&self) -> usize {
        self.version
    }
}
