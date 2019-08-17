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
    let mut prev_child: Option<Node<T>> = None;

    for child in children.drain(..) {
        if let Some(unwrapped_prev_child) = prev_child {
            parents.push(Node {
                aabb_min: unwrapped_prev_child.aabb_min.min(child.aabb_min),
                aabb_max: unwrapped_prev_child.aabb_max.max(child.aabb_max),
                value: Value::Node(Box::new(unwrapped_prev_child), Box::new(child)),
            });
            prev_child = None;
        } else {
            prev_child = Some(child);
        }
    }

    if let Some(unwrapped_prev_child) = prev_child {
        parents.push(unwrapped_prev_child);
    }

    drop(children);
    if parents.len() <= 1 {
        parents
    } else {
        build_layer(parents)
    }
}
