use crate::bvh;
use crate::math::{HasAABB, Vec3};

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub triangles: Vec<Triangle>,
    pub point_lights: Vec<PointLight>,
    pub triangles_bvh: Option<bvh::Node<Triangle>>,
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub top_left_corner: Vec3,
    pub plane_width: f32,
    pub plane_height: f32,
    pub right_vector: Vec3,
    pub down_vector: Vec3,
}

#[derive(Debug)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    // Light attenuation ax² + bx + c
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Triangle {
    pub a: Vertex,
    pub b: Vertex,
    pub c: Vertex,
}

impl HasAABB for Triangle {
    fn calculate_aabb(&self) -> (Vec3, Vec3) {
        let min = self.a.position.min(self.b.position).min(self.c.position);
        let max = self.a.position.max(self.b.position).max(self.c.position);
        (min, max)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}
