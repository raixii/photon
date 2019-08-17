use crate::math::Vec3;

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub triangles: Vec<Triangle>,
    pub point_lights: Vec<PointLight>,
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}
