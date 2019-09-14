use super::graph;
use super::graph::{Bsdf, EvaluationContext, Link, LinkType, Output};
use crate::math::Vec4;

pub mod outputs {
    pub const BSDF: usize = 0;
}

#[derive(Debug)]
pub struct Node {
    pub base_color: Link<Vec4>,
    // Stored in 1/0.08ths
    pub specular: Link<f64>,
    pub metallic: Link<f64>,
}

impl graph::Node for Node {
    fn evaluate(&self, ctx: &mut EvaluationContext) -> Vec<Output> {
        let bsdf = Bsdf {
            color: ctx.evaluate_link(self.base_color).xyz(),
            specular: ctx.evaluate_link(self.specular) * 0.08,
            metallic: ctx.evaluate_link(self.metallic),
        };
        return vec![bsdf.to_output()];
    }
}
