use super::nodes::{output_material, Bsdf, Graph, Link};
use crate::math::{HasAABB, Plane, Vec2, Vec3};

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub triangles: Vec<Triangle>,
    pub point_lights: Vec<PointLight>,
    pub materials: Vec<(usize, Graph)>,
}

impl Scene {
    pub fn evaluate_material(&self, triangle: &Triangle, tex_coord: Vec2) -> Bsdf {
        let (output_index, material) = &self.materials[triangle.material];
        let mut ctx = material.new_context(tex_coord);
        ctx.evaluate_link(Link::Node(*output_index, output_material::outputs::SURFACE))
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub radius: f64,
    // Light attenuation ax² + bx + c
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl HasAABB for PointLight {
    fn calculate_aabb(&self) -> (Vec3, Vec3) {
        let min = self.position - Vec3([self.radius; 3]);
        let max = self.position + Vec3([self.radius; 3]);
        (min, max)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Triangle {
    a: Vertex,
    b: Vertex,
    c: Vertex,
    material: usize,
    plane: Plane,
}

impl Triangle {
    pub fn new(ta: Vertex, tb: Vertex, tc: Vertex, material: usize) -> Triangle {
        // (a, b, c) is the normal vector of the triangle's plane:  n = (t[1]-t[0]) x (t[2]-t[0])
        // Triangle plane:  ax + by + cz = d
        //     (a, b, c) = n.xyz
        //     d = dot(t[0], n.xyz)
        let (pa, pb, pc, pd) = {
            let n = (tb.position - ta.position).cross(tc.position - ta.position);
            let d = ta.position.dot(n);
            (n.x(), n.y(), n.z(), d)
        };
        Triangle { a: ta, b: tb, c: tc, material, plane: Plane { a: pa, b: pb, c: pc, d: pd } }
    }

    pub fn a(&self) -> &Vertex {
        &self.a
    }

    pub fn b(&self) -> &Vertex {
        &self.b
    }

    pub fn c(&self) -> &Vertex {
        &self.c
    }

    pub fn plane(&self) -> &Plane {
        &self.plane
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
pub enum Geometry {
    Triangle(Triangle),
    PointLight(PointLight),
}

impl HasAABB for Geometry {
    fn calculate_aabb(&self) -> (Vec3, Vec3) {
        match self {
            Geometry::Triangle(t) => t.calculate_aabb(),
            Geometry::PointLight(pl) => pl.calculate_aabb(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}
