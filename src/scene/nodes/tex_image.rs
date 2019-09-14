use super::graph;
use super::graph::{EvaluationContext, LinkType, Output};

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

        // Bilinear interpolation between pixel centers
        let ideal_x = tex_coord.x() * image.w() as f64;
        let ideal_y = tex_coord.y() * image.h() as f64;

        let p1 = image.get(
            real_mod(floor05(ideal_x).floor() as isize, image.w() as isize),
            real_mod(floor05(ideal_y).floor() as isize, image.h() as isize),
        );
        let p2 = image.get(
            real_mod(floor05(ideal_x).floor() as isize + 1, image.w() as isize),
            real_mod(floor05(ideal_y).floor() as isize, image.h() as isize),
        );
        let p12 = p2 * (ideal_x - floor05(ideal_x)) + p1 * (floor05(ideal_x) + 1.0 - ideal_x);

        let p3 = image.get(
            real_mod(floor05(ideal_x).floor() as isize, image.w() as isize),
            real_mod(floor05(ideal_y).floor() as isize + 1, image.h() as isize),
        );
        let p4 = image.get(
            real_mod(floor05(ideal_x).floor() as isize + 1, image.w() as isize),
            real_mod(floor05(ideal_y).floor() as isize + 1, image.h() as isize),
        );
        let p34 = p4 * (ideal_x - floor05(ideal_x)) + p3 * (floor05(ideal_x) + 1.0 - ideal_x);

        let p1234 = p34 * (ideal_y - floor05(ideal_y)) + p12 * (floor05(ideal_y) + 1.0 - ideal_y);

        return vec![p1234.to_output(), p1234.w().to_output()];
    }
}

fn real_mod(num: isize, mod_by: isize) -> usize {
    if num >= 0 {
        (num % mod_by) as usize
    } else {
        (-(-num % mod_by) + mod_by) as usize
    }
}

fn floor05(num: f64) -> f64 {
    (num - 0.5).trunc() + 0.5
}
