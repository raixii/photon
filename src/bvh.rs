use crate::math::{HasAABB, Vec3};
use crate::simd::Simd4;
use std::f64::{INFINITY, NEG_INFINITY};
use std::fmt::{Debug, Formatter};

#[derive(Clone)]
enum Value<T: HasAABB + Clone> {
    Node,
    Empty,
    Leaf(T),
}

impl<T: HasAABB + Clone> Debug for Value<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Empty => write!(f, "ε"),
            Value::Node => write!(f, "N"),
            Value::Leaf(..) => write!(f, "L(..)"),
        }
    }
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
    aabb_min_x: Simd4,
    aabb_min_y: Simd4,
    aabb_min_z: Simd4,
    aabb_max_x: Simd4,
    aabb_max_y: Simd4,
    aabb_max_z: Simd4,
    value: [Value<T>; 4],
}

impl<T: HasAABB + Debug + Clone> Node<T> {
    fn get_aabb(&self, i: usize) -> (Vec3, Vec3) {
        let slot_aabb_min = Vec3([self.aabb_min_x[i], self.aabb_min_y[i], self.aabb_min_z[i]]);
        let slot_aabb_max = Vec3([self.aabb_max_x[i], self.aabb_max_y[i], self.aabb_max_z[i]]);
        (slot_aabb_min, slot_aabb_max)
    }
}

#[derive(Debug)]
pub struct Bvh<T: HasAABB + Debug + Clone> {
    // root = 0
    // child[i] = parent*4 + (i + 1)
    nodes: Vec<Node<T>>,
}

#[derive(Copy, Clone)]
pub struct BvhNode<'a, T: HasAABB + Debug + Clone> {
    bvh: &'a Bvh<T>,
    index: usize,
}

#[derive(Copy, Clone)]
pub enum BvhChild<'a, T: HasAABB + Debug + Clone> {
    Subtree(BvhNode<'a, T>),
    Value(&'a T),
    Empty,
}

impl<'a, T: HasAABB + Debug + Clone> BvhNode<'a, T> {
    pub fn aabb_min_x(&self) -> &Simd4 {
        &self.bvh.nodes[self.index].aabb_min_x
    }

    pub fn aabb_min_y(&self) -> &Simd4 {
        &self.bvh.nodes[self.index].aabb_min_y
    }

    pub fn aabb_min_z(&self) -> &Simd4 {
        &self.bvh.nodes[self.index].aabb_min_z
    }

    pub fn aabb_max_x(&self) -> &Simd4 {
        &self.bvh.nodes[self.index].aabb_max_x
    }

    pub fn aabb_max_y(&self) -> &Simd4 {
        &self.bvh.nodes[self.index].aabb_max_y
    }

    pub fn aabb_max_z(&self) -> &Simd4 {
        &self.bvh.nodes[self.index].aabb_max_z
    }

    pub fn value(&self, index: usize) -> BvhChild<'a, T> {
        match &self.bvh.nodes[self.index].value[index] {
            Value::Empty => BvhChild::Empty,
            Value::Leaf(value) => BvhChild::Value(value),
            Value::Node => {
                BvhChild::Subtree(BvhNode { bvh: self.bvh, index: self.index * 4 + index + 1 })
            }
        }
    }
}

impl<T: HasAABB + Clone + Debug> Bvh<T> {
    pub fn new(objects: &[T]) -> Bvh<T> {
        let layer_count = (objects.len() as f64).log(4.0).ceil() as u32;
        // node count = https://www.wolframalpha.com/input/?i=sum+4%5Ei+for+i+%3D+0+to+l-1
        let node_count = (4usize.pow(layer_count) - 1) / 3;
        let mut nodes = vec![
            Node {
                aabb_min_x: Simd4([INFINITY; 4]),
                aabb_min_y: Simd4([INFINITY; 4]),
                aabb_min_z: Simd4([INFINITY; 4]),
                aabb_max_x: Simd4([NEG_INFINITY; 4]),
                aabb_max_y: Simd4([NEG_INFINITY; 4]),
                aabb_max_z: Simd4([NEG_INFINITY; 4]),
                value: [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
            };
            node_count
        ];

        // init leaves
        let leafes_start_index = (4usize.pow(layer_count - 1) - 1) / 3;
        let leafes_end_index =
            leafes_start_index + objects.len() / 4 + if objects.len() % 4 == 0 { 0 } else { 1 };
        for i in 0..objects.len() {
            let node_i = i / 4 + leafes_start_index;
            let leaf_i = i % 4;
            let (aabb_min, aabb_max) = objects[i].calculate_aabb();
            nodes[node_i].aabb_min_x[leaf_i] = aabb_min.0[0];
            nodes[node_i].aabb_min_y[leaf_i] = aabb_min.0[1];
            nodes[node_i].aabb_min_z[leaf_i] = aabb_min.0[2];
            nodes[node_i].aabb_max_x[leaf_i] = aabb_max.0[0];
            nodes[node_i].aabb_max_y[leaf_i] = aabb_max.0[1];
            nodes[node_i].aabb_max_z[leaf_i] = aabb_max.0[2];
            nodes[node_i].value[leaf_i] = Value::Leaf(objects[i].clone());
        }
        sort_by_metric(&mut nodes, leafes_start_index, leafes_end_index);

        // init parent layers
        for layer in (0..(layer_count - 1)).rev() {
            let layer_start = (4usize.pow(layer) - 1) / 3;
            let layer_end = (4usize.pow(layer + 1) - 1) / 3;
            let mut layer_real_end = layer_end;
            'outer: for i in layer_start..layer_end {
                let children = [4 * i + 1, 4 * i + 2, 4 * i + 3, 4 * i + 4];
                match (
                    &nodes[children[0]].value,
                    &nodes[children[1]].value,
                    &nodes[children[2]].value,
                    &nodes[children[3]].value,
                ) {
                    (
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                    ) => {
                        layer_real_end = i;
                        break 'outer;
                    }
                    (
                        _,
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                        [Value::Empty, Value::Empty, Value::Empty, Value::Empty],
                    ) => {
                        swap_tree_rec(&mut nodes, children[0], i);
                        layer_real_end = i + 1;
                        break 'outer;
                    }
                    _ => {
                        for child_i in 0..4 {
                            for j in 0..4 {
                                if !nodes[children[child_i]].value[j].is_empty() {
                                    nodes[i].aabb_min_x[child_i] = nodes[i].aabb_min_x[child_i]
                                        .min(nodes[children[child_i]].aabb_min_x[j]);
                                    nodes[i].aabb_min_y[child_i] = nodes[i].aabb_min_y[child_i]
                                        .min(nodes[children[child_i]].aabb_min_y[j]);
                                    nodes[i].aabb_min_z[child_i] = nodes[i].aabb_min_z[child_i]
                                        .min(nodes[children[child_i]].aabb_min_z[j]);
                                    nodes[i].aabb_max_x[child_i] = nodes[i].aabb_max_x[child_i]
                                        .max(nodes[children[child_i]].aabb_max_x[j]);
                                    nodes[i].aabb_max_y[child_i] = nodes[i].aabb_max_y[child_i]
                                        .max(nodes[children[child_i]].aabb_max_y[j]);
                                    nodes[i].aabb_max_z[child_i] = nodes[i].aabb_max_z[child_i]
                                        .max(nodes[children[child_i]].aabb_max_z[j]);
                                    nodes[i].value[child_i] = Value::Node;
                                } else {
                                    layer_real_end = i + 1;
                                    break 'outer;
                                }
                            }
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
    if from < nodes.len() && to < nodes.len() {
        nodes.swap(from, to);
        // This order is important!
        swap_tree_rec(nodes, from * 4 + 4, to * 4 + 4);
        swap_tree_rec(nodes, from * 4 + 3, to * 4 + 3);
        swap_tree_rec(nodes, from * 4 + 2, to * 4 + 2);
        swap_tree_rec(nodes, from * 4 + 1, to * 4 + 1);
    }
}

fn calc_metric((a_min, a_max): (Vec3, Vec3), (b_min, b_max): (Vec3, Vec3)) -> f64 {
    let min = a_min.min(b_min);
    let max = a_max.max(b_max);
    let v = max - min;
    v.x() * v.y() + v.x() * v.z() + v.y() * v.z()
}

fn sort_by_metric<T: HasAABB + Debug + Clone>(nodes: &mut [Node<T>], from: usize, to: usize) {
    for slot in from..to {
        let mut current_aabb = nodes[slot].get_aabb(0);

        for neighbour in 1..4 {
            let mut min_metric = std::f64::INFINITY;
            let mut min_i = 0;
            let mut min_j = 0;
            for i in slot..to {
                for j in 0..4 {
                    if i == slot && j < neighbour {
                        continue;
                    }
                    if nodes[i].value[j].is_empty() {
                        assert!(i == to - 1);
                        continue;
                    }
                    let candidate_aabb = nodes[i].get_aabb(j);
                    let metric = calc_metric(current_aabb, candidate_aabb);
                    if metric < min_metric {
                        min_metric = metric;
                        min_i = i;
                        min_j = j;
                    }
                }
            }

            if min_metric.is_finite() {
                current_aabb.0 = current_aabb.0.min(nodes[min_i].get_aabb(min_j).0);
                current_aabb.1 = current_aabb.1.max(nodes[min_i].get_aabb(min_j).1);

                swap_tree_rec(nodes, slot * 4 + neighbour + 1, min_i * 4 + min_j + 1);
                if slot == min_i {
                    let node = &mut nodes[slot];
                    node.aabb_min_x.0.swap(neighbour, min_j);
                    node.aabb_min_y.0.swap(neighbour, min_j);
                    node.aabb_min_z.0.swap(neighbour, min_j);
                    node.aabb_max_x.0.swap(neighbour, min_j);
                    node.aabb_max_y.0.swap(neighbour, min_j);
                    node.aabb_max_z.0.swap(neighbour, min_j);
                    node.value.swap(neighbour, min_j);
                } else {
                    let (left, right) = nodes.split_at_mut(min_i);
                    let node_a = &mut left[slot];
                    let node_b = &mut right[0];
                    std::mem::swap(
                        &mut node_a.aabb_min_x[neighbour],
                        &mut node_b.aabb_min_x[min_j],
                    );
                    std::mem::swap(
                        &mut node_a.aabb_min_y[neighbour],
                        &mut node_b.aabb_min_y[min_j],
                    );
                    std::mem::swap(
                        &mut node_a.aabb_min_z[neighbour],
                        &mut node_b.aabb_min_z[min_j],
                    );
                    std::mem::swap(
                        &mut node_a.aabb_max_x[neighbour],
                        &mut node_b.aabb_max_x[min_j],
                    );
                    std::mem::swap(
                        &mut node_a.aabb_max_y[neighbour],
                        &mut node_b.aabb_max_y[min_j],
                    );
                    std::mem::swap(
                        &mut node_a.aabb_max_z[neighbour],
                        &mut node_b.aabb_max_z[min_j],
                    );
                    std::mem::swap(&mut node_a.value[neighbour], &mut node_b.value[min_j]);
                }
            }
        }
    }
}
