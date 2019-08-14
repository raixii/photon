use super::math::{Vec3};

#[derive(Debug)]
pub struct Scene {
    camera: Camera,
    triangles: Vec<Triangle>,
    point_lights: Vec<PointLight>
}

#[derive(Debug)]
pub struct Camera {
    position: Vec3,
    top_left_corner: Vec3,
    pixel_delta_x: Vec3,
    pixel_delta_y: Vec3,
    width: usize,
    height: usize,
}

#[derive(Debug)]
pub struct PointLight {
    position: Vec3,
    color: Vec3,
    // Light attenuation ax² + bx + c
    a: f32,
    b: f32,
    c: f32,
}

#[derive(Debug)]
pub struct Triangle {
    a: Vertex,
    b: Vertex,
    c: Vertex,   
}

#[derive(Debug)]
pub struct Vertex {
    position: Vec3,
    normal: Vec3,
}
