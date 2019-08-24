use std::ops::{Index, IndexMut};

#[repr(C, align(32))]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Simd4(pub [f64; 4]);

impl Simd4 {
    pub fn as_ptr(&self) -> *const f64 {
        self.0.as_ptr()
    }
}

impl Index<usize> for Simd4 {
    type Output = f64;

    #[inline(always)]
    fn index(&self, index: usize) -> &f64 {
        &self.0[index]
    }
}

impl IndexMut<usize> for Simd4 {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut f64 {
        &mut self.0[index]
    }
}
