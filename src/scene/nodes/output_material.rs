use super::graph;
use super::graph::{Bsdf, EvaluationContext, Link, LinkType, Output};

pub mod outputs {
    pub const SURFACE: usize = 0;
}

#[derive(Debug)]
pub struct Node {
    pub surface: Link<Bsdf>,
}

impl graph::Node for Node {
    fn evaluate(&self, ctx: &mut EvaluationContext) -> Vec<Output> {
        return vec![ctx.evaluate_link(self.surface).to_output()];
    }
}
