use crate::math::{Vec2, Vec3, Vec4};
use std::fmt::Debug;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Bsdf {
    pub color: Vec3,
    pub specular: f64,
    pub metallic: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum Output {
    Vec3(Vec3),
    Vec4(Vec4),
    F64(f64),
    Bsdf(Bsdf),
    Null,
}

pub trait LinkType: Debug + Clone + Copy {
    fn from_output(output: Output) -> Self;
    fn to_output(self) -> Output;
}

impl LinkType for f64 {
    fn from_output(o: Output) -> f64 {
        match o {
            Output::F64(v) => v,
            _ => panic!("Type error in graph"),
        }
    }

    fn to_output(self) -> Output {
        Output::F64(self)
    }
}

impl LinkType for Vec4 {
    fn from_output(o: Output) -> Vec4 {
        match o {
            Output::Vec4(v) => v,
            _ => panic!("Type error in graph"),
        }
    }

    fn to_output(self) -> Output {
        Output::Vec4(self)
    }
}

impl LinkType for Bsdf {
    fn from_output(o: Output) -> Bsdf {
        match o {
            Output::Bsdf(v) => v,
            _ => panic!("Type error in graph"),
        }
    }

    fn to_output(self) -> Output {
        Output::Bsdf(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Link<T: LinkType> {
    Constant(T),
    Node(usize, usize),
}

pub struct EvaluationContext<'a> {
    tex_coord: Vec2,
    graph: &'a Graph,
    node_results: Vec<Option<Vec<Output>>>,
}

impl<'a> EvaluationContext<'a> {
    pub fn evaluate_link<T: LinkType>(&mut self, link: Link<T>) -> T {
        match link {
            Link::Constant(c) => c,
            Link::Node(idx, socket) => {
                if self.node_results[idx].is_none() {
                    self.node_results[idx] = Some(self.graph.nodes[idx].evaluate(self))
                }
                LinkType::from_output(self.node_results[idx].as_ref().unwrap()[socket])
            }
        }
    }
}

pub trait Node: Debug + Sync + Send {
    fn evaluate(&self, ctx: &mut EvaluationContext) -> Vec<Output>;
}

#[derive(Debug)]
pub struct Graph {
    nodes: Vec<Box<dyn Node>>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph { nodes: vec![] }
    }

    pub fn add_node(&mut self, node: Box<dyn Node>) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    pub fn new_context(&self, tex_coord: Vec2) -> EvaluationContext {
        EvaluationContext { tex_coord, graph: &self, node_results: vec![None; self.nodes.len()] }
    }
}
