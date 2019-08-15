use super::math::{Vec3};

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub triangles: Vec<Triangle>,
    pub point_lights: Vec<PointLight>
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub top_left_corner: Vec3,
    pub pixel_delta_x: Vec3,
    pub pixel_delta_y: Vec3,
    pub width: usize,
    pub height: usize,
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

#[derive(Debug, Copy, Clone)]
pub struct Triangle {
    pub a: Vertex,
    pub b: Vertex,
    pub c: Vertex,   
}

#[derive(Debug, Copy, Clone)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}
