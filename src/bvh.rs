use crate::math::{HasAABB, Vec3};
use std::fmt::Debug;

#[derive(Debug)]
pub enum Value<T: HasAABB + Debug> {
    Node(Box<Node<T>>, Box<Node<T>>),
    Leaf(T),
}

#[derive(Debug)]
pub struct Node<T: HasAABB + Debug> {
    aabb_min: Vec3,
    aabb_max: Vec3,
    value: Value<T>,
}

#[derive(Debug)]
pub struct Bvh<T: HasAABB + Debug> {
    nodes: Vec<Node<T>>,
    // root = 0
    // child1 = parent*2 + 1
    // child2 = parent*2 + 2
}

impl<T: HasAABB + Debug> Node<T> {
    pub fn aabb_min(&self) -> Vec3 {
        self.aabb_min
    }

    pub fn aabb_max(&self) -> Vec3 {
        self.aabb_max
    }

    pub fn value(&self) -> &Value<T> {
        &self.value
    }
}

pub fn build<T: HasAABB + Clone + Debug>(objects: &[T]) -> Option<Node<T>> {
    // let layer_count = (objects.len() as f64).log2().ceil() as usize + 1;
    let mut leafs = Vec::with_capacity(objects.len());
    for object in objects {
        let (aabb_min, aabb_max) = object.calculate_aabb();
        leafs.push(Node {
            aabb_min,
            aabb_max,
            value: Value::Leaf(object.clone()),
        });
    }
    let mut root_layer = build_layer(leafs);
    if root_layer.is_empty() {
        None
    } else if root_layer.len() == 1 {
        Some(root_layer.remove(0))
    } else {
        unreachable!()
    }
}

fn build_layer<T: HasAABB + Debug>(mut children: Vec<Node<T>>) -> Vec<Node<T>> {
    let mut parents = Vec::with_capacity(children.len() / 2 + 1);

    while children.len() > 1 {
        let mut min_diag = std::f32::INFINITY;
        let mut min_i = 0;
        for i in 0..children.len() - 1 {
            let candiate_aabb_min = children[i]
                .aabb_min()
                .min(children.last().unwrap().aabb_min());
            let candiate_aabb_max = children[i]
                .aabb_max()
                .max(children.last().unwrap().aabb_max());
            let diag = (candiate_aabb_max - candiate_aabb_min).sqlen();
            if diag < min_diag {
                min_diag = diag;
                min_i = i;
            }
        }

        let a = children.swap_remove(children.len() - 1);
        let b = children.swap_remove(min_i);
        parents.push(Node {
            aabb_min: a.aabb_min().min(b.aabb_min()),
            aabb_max: a.aabb_max().max(b.aabb_max()),
            value: Value::Node(Box::new(a), Box::new(b)),
        });
    }

    if !children.is_empty() {
        parents.push(children.remove(0));
    }

    assert!(children.is_empty());

    if parents.len() <= 1 {
        parents
    } else {
        build_layer(parents)
    }
}
