use crate::bvh::Bvh;
use crate::math::{HasAABB, Vec3};

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub triangles: Vec<Triangle>,
    pub point_lights: Vec<PointLight>,
    pub triangles_bvh: Option<Bvh<Triangle>>,
    pub materials: Vec<Material>,
}

impl Scene {
    pub fn material_of_triangle(&self, triangle: &Triangle) -> &Material {
        &self.materials[triangle.material]
    }
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub top_left_corner: Vec3,
    pub plane_width: f64,
    pub plane_height: f64,
    pub right_vector: Vec3,
    pub down_vector: Vec3,
}

#[derive(Debug)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    // Light attenuation ax² + bx + c
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Triangle {
    pub a: Vertex,
    pub b: Vertex,
    pub c: Vertex,
    material: usize,
}

impl Triangle {
    pub fn new(a: Vertex, b: Vertex, c: Vertex, material: usize) -> Triangle {
        Triangle { a, b, c, material }
    }
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Material {
    pub emission: Vec3,
    pub color: Vec3,
    pub specular: f64,
    //
    // roughness - kommt evtl mit Blender 2.81 aus dem collada
    // ad later for transparency
    // alpha: f64,
    // refraction: f64
}
