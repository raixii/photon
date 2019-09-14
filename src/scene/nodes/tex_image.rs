use super::graph;
use super::graph::{EvaluationContext, LinkType, Output};
use crate::math::Vec4;

pub mod outputs {
    pub const COLOR: usize = 0;
    pub const ALPHA: usize = 1;
}

#[derive(Debug)]
pub struct Node {
    pub image: usize,
}

impl graph::Node for Node {
    fn evaluate(&self, ctx: &mut EvaluationContext) -> Vec<Output> {
        let image = &ctx.scene().images[self.image];
        let tex_coord = ctx.tex_coord();

        // Bilinear interpolation
        let ideal_x = tex_coord.x() * image.w() as f64;
        let ideal_y = tex_coord.y() * image.h() as f64;

        let p1 = image.get(
            real_mod(ideal_x.trunc() as isize, image.w() as isize),
            real_mod(ideal_y.trunc() as isize, image.h() as isize),
        );
        let p2 = image.get(
            real_mod(ideal_x.trunc() as isize + 1, image.w() as isize),
            real_mod(ideal_y.trunc() as isize, image.h() as isize),
        );
        let p12 = p2 * (ideal_x - ideal_x.trunc()) + p1 * (ideal_x.trunc() + 1.0 - ideal_x);

        let p3 = image.get(
            real_mod(ideal_x.trunc() as isize, image.w() as isize),
            real_mod(ideal_y.trunc() as isize + 1, image.h() as isize),
        );
        let p4 = image.get(
            real_mod(ideal_x.trunc() as isize + 1, image.w() as isize),
            real_mod(ideal_y.trunc() as isize + 1, image.h() as isize),
        );
        let p34 = p4 * (ideal_x - ideal_x.trunc()) + p3 * (ideal_x.trunc() + 1.0 - ideal_x);

        let p1234 = p34 * (ideal_y + ideal_y.trunc()) + p12 * (ideal_y.trunc() + 1.0 - ideal_y);

        // Convert sRGB to linear colorspace
        let c = Vec4([p1234.x().powf(2.2), p1234.y().powf(2.2), p1234.z().powf(2.2), p1234.w()]);

        return vec![c.to_output(), c.w().to_output()];
    }
}

fn real_mod(num: isize, mod_by: isize) -> usize {
    if num >= 0 {
        (num % mod_by) as usize
    } else {
        (-(-num % mod_by) + mod_by) as usize
    }
}
