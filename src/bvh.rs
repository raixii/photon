use crate::math::{HasAABB, Vec3};
use std::fmt::Debug;

#[derive(Debug, Clone)]
enum Value<T: HasAABB + Debug + Clone> {
    Node,
    Empty,
    Leaf(T),
}

impl<T: HasAABB + Debug + Clone> Value<T> {
    fn is_empty(&self) -> bool {
        match self {
            Value::Empty => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
struct Node<T: HasAABB + Debug + Clone> {
    aabb_min: Vec3,
    aabb_max: Vec3,
    value: Value<T>,
}
#[derive(Debug)]
pub struct Bvh<T: HasAABB + Debug + Clone> {
    // root = 0
    // child1 = parent*2 + 1
    // child2 = parent*2 + 2
    nodes: Vec<Node<T>>,
}

#[derive(Copy, Clone)]
pub struct BvhNode<'a, T: HasAABB + Debug + Clone> {
    bvh: &'a Bvh<T>,
    index: usize,
}

#[derive(Copy, Clone)]
pub enum BvhChild<'a, T: HasAABB + Debug + Clone> {
    Nodes(BvhNode<'a, T>, BvhNode<'a, T>),
    Leaf(&'a T),
}

impl<'a, T: HasAABB + Debug + Clone> BvhNode<'a, T> {
    pub fn aabb_min(&self) -> Vec3 {
        self.bvh.nodes[self.index].aabb_min
    }

    pub fn aabb_max(&self) -> Vec3 {
        self.bvh.nodes[self.index].aabb_max
    }

    pub fn value(&self) -> BvhChild<'a, T> {
        match self.bvh.nodes[self.index].value {
            Value::Empty => unreachable!(),
            Value::Leaf(ref v) => BvhChild::Leaf(v),
            Value::Node => BvhChild::Nodes(
                BvhNode { bvh: self.bvh, index: self.index * 2 + 1 },
                BvhNode { bvh: self.bvh, index: self.index * 2 + 2 },
            ),
        }
    }
}

impl<T: HasAABB + Debug + Clone> Bvh<T> {
    pub fn new(objects: &[T]) -> Bvh<T> {
        let layer_count = (objects.len() as f64).log2().ceil() as usize + 1;
        let node_count = (1 << layer_count) - 1; // there are 2^layer_count-1 nodes in a tree
        let mut nodes = vec![
            Node {
                aabb_min: Vec3([std::f64::NAN; 3]),
                aabb_max: Vec3([std::f64::NAN; 3]),
                value: Value::Empty
            };
            node_count
        ];

        // init leaves
        for i in 0..objects.len() {
            let (aabb_min, aabb_max) = objects[i].calculate_aabb();
            nodes[node_count / 2 + i] =
                Node { aabb_min, aabb_max, value: Value::Leaf(objects[i].clone()) };
        }
        sort_by_metric(&mut nodes, node_count / 2, node_count / 2 + objects.len());

        // init parent layers
        for layer in (0..(layer_count - 1)).rev() {
            let layer_start = (1 << layer) - 1;
            let layer_end = (1 << (layer + 1)) - 1;
            let mut layer_real_end = layer_end;
            for i in layer_start..layer_end {
                let child_a = &nodes[2 * i + 1];
                let child_b = &nodes[2 * i + 2];
                match (&child_a.value, &child_b.value) {
                    (Value::Empty, Value::Empty) => {
                        layer_real_end = i;
                        break;
                    }
                    (Value::Empty, _) => {
                        unreachable!();
                    }
                    (_, Value::Empty) => {
                        //      i
                        //    n   e
                        //  n1 n2
                        // ......
                        //
                        // 1. n -> i
                        // 2. n1 -> e
                        // 3. n2 -> n
                        let e = 2 * i + 2;
                        let n = 2 * i + 1;
                        // 1. n -> a
                        nodes.swap(n, i);
                        // 2. n1 -> e
                        swap_tree_rec(&mut nodes, n * 2 + 1, e);
                        // 3. n2 -> n
                        swap_tree_rec(&mut nodes, n * 2 + 2, n);
                        layer_real_end = i + 1;
                        break;
                    }
                    (_, _) => {
                        nodes[i] = Node {
                            aabb_min: child_a.aabb_min.min(child_b.aabb_min),
                            aabb_max: child_a.aabb_max.max(child_b.aabb_max),
                            value: Value::Node,
                        }
                    }
                }
            }
            sort_by_metric(&mut nodes, layer_start, layer_real_end);
        }

        Bvh { nodes }
    }

    pub fn root(&self) -> BvhNode<'_, T> {
        BvhNode { bvh: self, index: 0 }
    }
}

fn swap_tree_rec<T: HasAABB + Debug + Clone>(nodes: &mut [Node<T>], from: usize, to: usize) {
    if from < nodes.len() && to < nodes.len() && !nodes[from].value.is_empty() {
        nodes.swap(from, to);
        swap_tree_rec(nodes, from * 2 + 1, to * 2 + 1);
        swap_tree_rec(nodes, from * 2 + 2, to * 2 + 2);
    }
}

fn calc_metric<T: HasAABB + Debug + Clone>(a: &Node<T>, b: &Node<T>) -> f64 {
    assert!(!a.value.is_empty() && !b.value.is_empty());
    (a.aabb_min.min(b.aabb_min) - a.aabb_max.max(b.aabb_max)).manhattan_len()
}

fn sort_by_metric<T: HasAABB + Debug + Clone>(nodes: &mut [Node<T>], from: usize, to: usize) {
    for slot in from..to / 2 {
        let slot = slot * 2;
        let mut min_metric = std::f64::INFINITY;
        let mut min_i = 0;
        for i in slot + 1..nodes.len() {
            let metric = calc_metric(&nodes[slot], &nodes[i]);
            if metric < min_metric {
                min_metric = metric;
                min_i = i;
            }
        }
        if slot + 1 != min_i {
            swap_tree_rec(nodes, slot + 1, min_i);
        }
    }
}
