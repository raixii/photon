use super::{Import, ImportError};
use crate::math::{AlmostEq, Mat4, Vec3, Vec4};
use crate::scene::{Camera, Material, PointLight, Scene, Triangle, Vertex};
use serde::Deserialize;
use std::collections::HashMap;

pub struct Blender<'a> {
    string: &'a str,
    w: usize,
    h: usize,
}

impl<'a> Blender<'a> {
    pub fn new(string: &str, w: usize, h: usize) -> Blender {
        Blender { string, w, h }
    }
}

#[derive(Deserialize, Debug)]
struct BlenderJson {
    objects: HashMap<String, BlenderObject>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
struct BlenderObject {
    name: String,
    #[serde(flatten)]
    object: BlenderObjectData,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum BlenderObjectData {
    #[serde(rename = "MESH")]
    Mesh(Box<BlenderMesh>),
    #[serde(rename = "LIGHT")]
    Light(BlenderLight),
    #[serde(rename = "CAMERA")]
    Camera(BlenderCamera),
}

#[derive(Deserialize, Debug)]
struct BlenderMesh {
    triangles: Vec<BlenderTriangle>,
    material: BlenderMaterial,
    matrix: BlenderMat4,
}

#[derive(Deserialize, Debug)]
struct BlenderLight {
    color: (f64, f64, f64),
    power: f64,
    specular: f64,
    radius: f64,
    attenuation: (f64, f64, f64),
    matrix: BlenderMat4,
}

#[derive(Deserialize, Debug)]
struct BlenderCamera {
    matrix: BlenderMat4,
    xfov: f64,
    yfov: f64,
    znear: f64,
    zfar: f64,
}

#[derive(Deserialize, Debug)]
struct BlenderTriangle {
    p: (f64, f64, f64),
    n: (f64, f64, f64),
    t: (f64, f64),
}

#[derive(Deserialize, Debug)]
struct BlenderMaterial {
    name: String,
    base_color: (f64, f64, f64, f64),
    subsurface: f64,
    subsurface_radius: (f64, f64, f64),
    subsurface_color: (f64, f64, f64, f64),
    metallic: f64,
    specular: f64,
    specular_tint: f64,
    roughness: f64,
    anisotropic: f64,
    anisotropic_rotation: f64,
    sheen: f64,
    sheen_tint: f64,
    clearcoat: f64,
    clearcoat_roughness: f64,
    ior: f64,
    transmission: f64,
    transmission_roughness: f64,
    emission: (f64, f64, f64, f64),
    alpha: f64,
    normal: (f64, f64, f64),
    clearcoat_normal: (f64, f64, f64),
    tangent: (f64, f64, f64),
}

type BlenderMat4 =
    ((f64, f64, f64, f64), (f64, f64, f64, f64), (f64, f64, f64, f64), (f64, f64, f64, f64));

impl<'a> Import for Blender<'a> {
    fn import(&self) -> Result<Scene, ImportError> {
        let json: BlenderJson = serde_json::from_str(self.string).map_err(|e| format!("{}", e))?;

        let mut scene_camera = None;
        let mut scene_lights = vec![];
        let mut scene_triangles = vec![];
        let mut scene_materials = vec![];

        for (_, object) in json.objects {
            match object.object {
                BlenderObjectData::Camera(camera) => {
                    let camera_transform = to_mat4(camera.matrix);
                    let camera_position = (camera_transform * Vec4([0.0, 0.0, 0.0, 1.0])).xyz();
                    let camera_look =
                        (camera_transform * Vec4([0.0, 0.0, -1.0, 0.0])).xyz().normalize();
                    let camera_up =
                        (camera_transform * Vec4([0.0, 1.0, 0.0, 0.0])).xyz().normalize();
                    let camera_left =
                        (camera_transform * Vec4([-1.0, 0.0, 0.0, 0.0])).xyz().normalize();
                    if !(camera_look.dot(camera_up).almost_zero()
                        && camera_look.dot(camera_left).almost_zero()
                        && camera_left.dot(camera_up).almost_zero())
                    {
                        panic!("Camera is transformed without keeping the angles.");
                    }
                    let image_plane_half_width = camera.znear * (camera.xfov / 2.0).tan();
                    let image_plane_half_height =
                        image_plane_half_width / (self.w as f64 / self.h as f64);
                    let image_plane_top_left = camera_position
                        + camera.znear * camera_look
                        + image_plane_half_width * camera_left
                        + image_plane_half_height * camera_up;
                    scene_camera = Some(Camera {
                        position: camera_position,
                        top_left_corner: image_plane_top_left,
                        plane_width: image_plane_half_width * 2.0,
                        plane_height: image_plane_half_height * 2.0,
                        right_vector: -camera_left,
                        down_vector: -camera_up,
                    });
                }
                BlenderObjectData::Light(light) => {
                    let position = (to_mat4(light.matrix) * Vec4([0.0, 0.0, 0.0, 1.0])).xyz();
                    scene_lights.push(PointLight {
                        position,
                        color: to_vec3(light.color) * light.power,
                        a: light.attenuation.0,
                        b: light.attenuation.1,
                        c: light.attenuation.2,
                    });
                }
                BlenderObjectData::Mesh(mesh) => {
                    let matrix = to_mat4(mesh.matrix);
                    let nmatrix = matrix.inv().transpose();
                    let mut triangle = Triangle::new(
                        Vertex { position: Vec3([0.0; 3]), normal: Vec3([0.0; 3]) },
                        Vertex { position: Vec3([0.0; 3]), normal: Vec3([0.0; 3]) },
                        Vertex { position: Vec3([0.0; 3]), normal: Vec3([0.0; 3]) },
                        scene_materials.len(),
                    );
                    let mut i = 0;
                    for t in mesh.triangles {
                        let vertex = match i {
                            0 => &mut triangle.a,
                            1 => &mut triangle.b,
                            2 => &mut triangle.c,
                            _ => unreachable!(),
                        };
                        vertex.position = (matrix * to_vec3(t.p).xyz1()).xyz();
                        vertex.normal = (nmatrix * to_vec3(t.n).xyz0()).xyz();
                        if i == 2 {
                            scene_triangles.push(triangle);
                            i = 0;
                        } else {
                            i += 1;
                        }
                    }

                    scene_materials.push(Material {
                        color: to_vec4(mesh.material.base_color).xyz(),
                        specular: mesh.material.specular * 0.08,
                        metallic: mesh.material.metallic,
                    });
                }
            }
        }

        Ok(Scene {
            camera: scene_camera.ok_or("Scene does not have a camera.")?,
            triangles: scene_triangles,
            point_lights: scene_lights,
            triangles_bvh: None,
            materials: scene_materials,
        })
    }
}

fn to_mat4(mat: BlenderMat4) -> Mat4 {
    Mat4([
        [(mat.0).0, (mat.1).0, (mat.2).0, (mat.3).0],
        [(mat.0).1, (mat.1).1, (mat.2).1, (mat.3).1],
        [(mat.0).2, (mat.1).2, (mat.2).2, (mat.3).2],
        [(mat.0).3, (mat.1).3, (mat.2).3, (mat.3).3],
    ])
}

fn to_vec3(v: (f64, f64, f64)) -> Vec3 {
    Vec3([v.0, v.1, v.2])
}

fn to_vec4(v: (f64, f64, f64, f64)) -> Vec4 {
    Vec4([v.0, v.1, v.2, v.3])
}
