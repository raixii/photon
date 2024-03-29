use super::{Import, ImportError};
use crate::math::{AlmostEq, Mat4, Vec2, Vec3, Vec4};
use crate::scene::{
    bsdf_principled, output_material, tex_image, Bsdf, Camera, Graph, Image, Link, LinkType,
    PointLight, Scene, Triangle, Vertex,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Debug;

pub struct Blender<'a> {
    pwd: &'a str,
    string: &'a str,
    w: usize,
    h: usize,
}

impl<'a> Blender<'a> {
    pub fn new(pwd: &'a str, string: &'a str, w: usize, h: usize) -> Blender<'a> {
        Blender { pwd, string, w, h }
    }

    fn resolve_path(&self, path: &'a str) -> String {
        if path.starts_with("//") {
            format!("{}/{}", self.pwd, &path[2..])
        } else {
            path.to_owned()
        }
    }
}

#[derive(Deserialize, Debug)]
struct BlenderJson {
    objects: BTreeMap<String, BlenderObject>,
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
    Mesh(BlenderMesh),
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
    nodes: BTreeMap<String, BlenderNode>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum BlenderNode {
    #[serde(rename = "OUTPUT_MATERIAL")]
    OutputMaterial(BlenderOutputMaterial),
    #[serde(rename = "BSDF_PRINCIPLED")]
    BsdfPrincipled(BlenderBsdfPrincipled),
    #[serde(rename = "TEX_IMAGE")]
    TexImage(BlenderTexImage),
}

impl BlenderNode {
    pub fn map_output(&self, socket: &str) -> Result<usize, ImportError> {
        use BlenderNode::*;
        match (self, socket) {
            (BsdfPrincipled(_), "bsdf") => Ok(bsdf_principled::outputs::BSDF),
            (TexImage(_), "color") => Ok(tex_image::outputs::COLOR),
            (TexImage(_), "alpha") => Ok(tex_image::outputs::ALPHA),
            _ => Err(ImportError::from(format!("Unknown output socket {}", socket))),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum BlenderSocket<T: Debug> {
    #[serde(rename = "VALUE")]
    Value(BlenderValue<T>),
    #[serde(rename = "LINK")]
    Link(BlenderLink),
}

impl<T: Debug + Clone> BlenderSocket<T> {
    fn to_link<To: LinkType, Mapper: (FnOnce(&T) -> To)>(
        &self,
        nodes: &BTreeMap<&str, (usize, &BlenderNode)>,
        mapper: Mapper,
    ) -> Result<Link<To>, ImportError> {
        match self {
            BlenderSocket::Value(v) => Ok(Link::Constant(mapper(&v.value))),
            BlenderSocket::Link(BlenderLink { from_node, from_socket }) => {
                let (index, blender_node) = nodes
                    .get(from_node.as_str())
                    .ok_or_else(|| format!("Node not found {}", from_node))?;
                Ok(Link::Node(*index, blender_node.map_output(from_socket)?))
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct BlenderLink {
    from_node: String,
    from_socket: String,
}

#[derive(Deserialize, Debug)]
struct BlenderValue<T: Debug> {
    value: T,
}

#[derive(Deserialize, Debug)]
struct BlenderOutputMaterial {
    in_surface: BlenderSocket<Option<()>>,
    in_volume: BlenderSocket<Option<()>>,
    in_displacement: BlenderSocket<(f64, f64, f64)>,
}

#[derive(Deserialize, Debug)]
struct BlenderBsdfPrincipled {
    in_base_color: BlenderSocket<(f64, f64, f64, f64)>,
    in_subsurface: BlenderSocket<f64>,
    in_subsurface_radius: BlenderSocket<(f64, f64, f64)>,
    in_subsurface_color: BlenderSocket<(f64, f64, f64, f64)>,
    in_metallic: BlenderSocket<f64>,
    in_specular: BlenderSocket<f64>,
    in_specular_tint: BlenderSocket<f64>,
    in_roughness: BlenderSocket<f64>,
    in_anisotropic: BlenderSocket<f64>,
    in_anisotropic_rotation: BlenderSocket<f64>,
    in_sheen: BlenderSocket<f64>,
    in_sheen_tint: BlenderSocket<f64>,
    in_clearcoat: BlenderSocket<f64>,
    in_clearcoat_roughness: BlenderSocket<f64>,
    in_ior: BlenderSocket<f64>,
    in_transmission: BlenderSocket<f64>,
    in_transmission_roughness: BlenderSocket<f64>,
    in_emission: BlenderSocket<(f64, f64, f64, f64)>,
    in_alpha: BlenderSocket<f64>,
    in_normal: BlenderSocket<(f64, f64, f64)>,
    in_clearcoat_normal: BlenderSocket<(f64, f64, f64)>,
    in_tangent: BlenderSocket<(f64, f64, f64)>,
    out_bsdf: BlenderSocket<Option<()>>,
}

#[derive(Deserialize, Debug)]
struct BlenderTexImage {
    in_vector: BlenderSocket<(f64, f64, f64)>,
    out_color: BlenderSocket<(f64, f64, f64, f64)>,
    out_alpha: BlenderSocket<f64>,
    interpolation: String,
    projection: String,
    extension: String,
    source: String,
    filepath: String,
    colorspace: String,
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
        let mut scene_images = vec![];

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
                        radius: light.radius,
                        a: light.attenuation.0,
                        b: light.attenuation.1,
                        c: light.attenuation.2,
                    });
                }
                BlenderObjectData::Mesh(mesh) => {
                    let matrix = to_mat4(mesh.matrix);
                    let nmatrix = matrix.inv().transpose();
                    let mut triangle = (
                        Vertex {
                            position: Vec3([0.0; 3]),
                            normal: Vec3([0.0; 3]),
                            tex_coord: Vec2([0.0; 2]),
                        },
                        Vertex {
                            position: Vec3([0.0; 3]),
                            normal: Vec3([0.0; 3]),
                            tex_coord: Vec2([0.0; 2]),
                        },
                        Vertex {
                            position: Vec3([0.0; 3]),
                            normal: Vec3([0.0; 3]),
                            tex_coord: Vec2([0.0; 2]),
                        },
                    );
                    let mut i = 0;
                    for t in mesh.triangles {
                        let vertex = match i {
                            0 => &mut triangle.0,
                            1 => &mut triangle.1,
                            2 => &mut triangle.2,
                            _ => unreachable!(),
                        };
                        vertex.position = (matrix * to_vec3(t.p).xyz1()).xyz();
                        vertex.normal = (nmatrix * to_vec3(t.n).xyz0()).xyz();
                        vertex.tex_coord = to_vec2(t.t);
                        if i == 2 {
                            scene_triangles.push(Triangle::new(
                                triangle.0,
                                triangle.1,
                                triangle.2,
                                scene_materials.len(),
                            ));
                            i = 0;
                        } else {
                            i += 1;
                        }
                    }

                    let mut nodes = BTreeMap::<&str, (usize, &BlenderNode)>::new();
                    let mut output_index = None;
                    for (i, (node_name, node)) in mesh.material.nodes.iter().enumerate() {
                        if let BlenderNode::OutputMaterial(_) = node {
                            if output_index.is_none() {
                                output_index = Some(i);
                            } else {
                                return Err(ImportError::from(format!(
                                    "Duplicate OUTPUT_MATERIAL in material {}",
                                    mesh.material.name
                                )));
                            }
                        }
                        nodes.insert(node_name, (i, node));
                    }
                    let mesh_material_name = mesh.material.name.as_str();
                    let output_index = output_index.ok_or_else(|| {
                        format!("Missing OUTPUT_MATERIAL in material {}", mesh_material_name)
                    })?;

                    let mut node_graph = Graph::new();
                    for node in mesh.material.nodes.values() {
                        node_graph.add_node(match node {
                            BlenderNode::OutputMaterial(node) => Box::new(output_material::Node {
                                surface: node.in_surface.to_link(&nodes, |_| Bsdf {
                                    color: Vec3([1.0, 1.0, 1.0]),
                                    specular: 0.0,
                                    metallic: 0.0,
                                })?,
                            }),
                            BlenderNode::BsdfPrincipled(node) => Box::new(bsdf_principled::Node {
                                base_color: node.in_base_color.to_link(&nodes, |v| to_vec4(*v))?,
                                specular: node.in_specular.to_link(&nodes, |v| *v)?,
                                metallic: node.in_metallic.to_link(&nodes, |v| *v)?,
                            }),
                            BlenderNode::TexImage(node) => {
                                if node.interpolation != "Linear" {
                                    return Err(ImportError::from(
                                        "Textures only support linear interpolation",
                                    ));
                                }
                                if node.projection != "FLAT" {
                                    return Err(ImportError::from(
                                        "Textures only support flat projection",
                                    ));
                                }
                                if node.extension != "REPEAT" {
                                    return Err(ImportError::from(
                                        "Textures only support repeat extension",
                                    ));
                                }
                                if node.source != "FILE" {
                                    return Err(ImportError::from(
                                        "Textures may only come from files",
                                    ));
                                }
                                if node.colorspace != "sRGB" {
                                    return Err(ImportError::from(
                                        "Textures only support sRGB color-space",
                                    ));
                                }

                                let image_path = self.resolve_path(&node.filepath);
                                let image_index = scene_images.len();
                                scene_images.push(Image::from_path(&image_path)?);

                                Box::new(tex_image::Node { image: image_index })
                            }
                        });
                    }

                    scene_materials.push((output_index, node_graph));
                }
            }
        }

        Ok(Scene {
            camera: scene_camera.ok_or("Scene does not have a camera.")?,
            triangles: scene_triangles,
            point_lights: scene_lights,
            materials: scene_materials,
            images: scene_images,
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

fn to_vec2(v: (f64, f64)) -> Vec2 {
    Vec2([v.0, v.1])
}

fn to_vec3(v: (f64, f64, f64)) -> Vec3 {
    Vec3([v.0, v.1, v.2])
}

fn to_vec4(v: (f64, f64, f64, f64)) -> Vec4 {
    Vec4([v.0, v.1, v.2, v.3])
}
