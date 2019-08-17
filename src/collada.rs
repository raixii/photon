use crate::math::{AlmostEq, Mat4, Vec3, Vec4};
use crate::scene::{Camera, PointLight, Scene, Triangle, Vertex};
use std::f32::consts::PI;
use std::str::FromStr;
use sxd_document::dom::{ChildOfElement, Document, Element};
use sxd_document::parser;
use sxd_xpath::nodeset::Node;
use sxd_xpath::{Context, Factory, Value};

pub fn read(xml: &str) -> Scene {
    let mut context = Context::new();
    context.set_namespace("c", "http://www.collada.org/2005/11/COLLADASchema");
    let package = parser::parse(xml).unwrap();
    let doc = package.as_document();
    let root = Node::Root(doc.root());

    let scene_instance_url = evaluate_xpath_attribute(
        root,
        "/c:COLLADA/c:scene/c:instance_visual_scene/@url",
        &context,
    );
    let visual_scene = get_by_url(&doc, scene_instance_url, &context);

    let camera_element = evaluate_xpath_element(
        Node::Element(visual_scene),
        "./c:node/c:instance_camera/..",
        &context,
    );

    let camera_url = evaluate_xpath_attribute(
        Node::Element(camera_element),
        "./c:instance_camera/@url",
        &context,
    );
    let camera_specs = get_by_url(&doc, camera_url, &context);

    let camera_transform = get_transform_of_node(camera_element, &context);
    let camera_position = (camera_transform * Vec4([0.0, 0.0, 0.0, 1.0])).xyz();
    let camera_look = (camera_transform * Vec4([0.0, 0.0, -1.0, 0.0]))
        .xyz()
        .normalize();
    let camera_up = (camera_transform * Vec4([0.0, 1.0, 0.0, 0.0]))
        .xyz()
        .normalize();
    let camera_left = (camera_transform * Vec4([-1.0, 0.0, 0.0, 0.0]))
        .xyz()
        .normalize();
    if !(camera_look.dot(camera_up).almost_zero()
        && camera_look.dot(camera_left).almost_zero()
        && camera_left.dot(camera_up).almost_zero())
    {
        panic!("Camera is transformed without keeping the angles.");
    }

    let aspect_ratio: f32 = FromStr::from_str(get_text(evaluate_xpath_element(
        Node::Element(camera_specs),
        "./c:optics/c:technique_common/c:perspective/c:aspect_ratio",
        &context,
    )))
    .unwrap();
    let znear: f32 = FromStr::from_str(get_text(evaluate_xpath_element(
        Node::Element(camera_specs),
        "./c:optics/c:technique_common/c:perspective/c:znear",
        &context,
    )))
    .unwrap();
    let fov_deg: f32 = FromStr::from_str(get_text(evaluate_xpath_element(
        Node::Element(camera_specs),
        "./c:optics/c:technique_common/c:perspective/c:xfov",
        &context,
    )))
    .unwrap();
    let fov = fov_deg / 180.0 * PI;
    // let alpha = (PI - fov) / 2.0;
    let image_plane_half_width = znear * (fov / 2.0).tan(); // * (fov / 2.0).sin() / alpha.sin();
    let image_plane_top_left = camera_position
        + znear * camera_look
        + image_plane_half_width * camera_left
        + (image_plane_half_width / aspect_ratio) * camera_up;
    let camera = Camera {
        position: camera_position,
        top_left_corner: image_plane_top_left,
        plane_width: image_plane_half_width * 2.0,
        plane_height: image_plane_half_width / aspect_ratio * 2.0,
        right_vector: -camera_left,
        down_vector: -camera_up,
    };

    let point_light_nodes = evaluate_xpath_element_all(
        Node::Element(visual_scene),
        "./c:node/c:instance_light/..",
        &context,
    );
    let mut point_lights: Vec<PointLight> = Vec::new();
    for light in point_light_nodes {
        let light_node = {
            let light_url =
                evaluate_xpath_attribute(Node::Element(light), "./c:instance_light/@url", &context);
            get_by_url(&doc, light_url, &context)
        };
        let position = {
            let light_transform = get_transform_of_node(light, &context);
            (light_transform * Vec4([0.0, 0.0, 0.0, 1.0])).xyz()
        };

        let color = {
            let color = get_text(evaluate_xpath_element(
                Node::Element(light_node),
                "./c:technique_common/c:point/c:color",
                &context,
            ));
            let rgb: Vec<f32> = color
                .split_whitespace()
                .map(|c| FromStr::from_str(c).unwrap())
                .collect();
            Vec3([rgb[0], rgb[1], rgb[2]])
        };

        let a: f32 = FromStr::from_str(get_text(evaluate_xpath_element(
            Node::Element(light_node),
            "./c:technique_common/c:point/c:quadratic_attenuation",
            &context,
        )))
        .unwrap();
        let b: f32 = FromStr::from_str(get_text(evaluate_xpath_element(
            Node::Element(light_node),
            "./c:technique_common/c:point/c:linear_attenuation",
            &context,
        )))
        .unwrap();
        let c: f32 = FromStr::from_str(get_text(evaluate_xpath_element(
            Node::Element(light_node),
            "./c:technique_common/c:point/c:constant_attenuation",
            &context,
        )))
        .unwrap();

        point_lights.push(PointLight {
            position,
            color,
            a,
            b,
            c,
        });
    }

    let mut triangles = Vec::new();
    let object_elements = evaluate_xpath_element_all(
        Node::Element(visual_scene),
        "./c:node/c:instance_geometry/..",
        &context,
    );
    for object_element in object_elements {
        let object_transform = get_transform_of_node(object_element, &context);
        let instance_geometry_url = evaluate_xpath_attribute(
            Node::Element(object_element),
            "./c:instance_geometry/@url",
            &context,
        );
        let geometry_element = get_by_url(&doc, instance_geometry_url, &context);
        let vertex_input = evaluate_xpath_element(
            Node::Element(geometry_element),
            "./c:mesh/c:triangles/c:input[@semantic=\"VERTEX\"]",
            &context,
        );
        let normal_input = evaluate_xpath_element(
            Node::Element(geometry_element),
            "./c:mesh/c:triangles/c:input[@semantic=\"NORMAL\"]",
            &context,
        );
        let vertices = get_by_url(
            &doc,
            vertex_input.attribute("source").unwrap().value(),
            &context,
        );
        let position_source_url = evaluate_xpath_attribute(
            Node::Element(vertices),
            "./c:input[@semantic=\"POSITION\"]/@source",
            &context,
        );

        let positions =
            get_vec3s_of_source(get_by_url(&doc, position_source_url, &context), &context);
        let normals = get_vec3s_of_source(
            get_by_url(
                &doc,
                normal_input.attribute("source").unwrap().value(),
                &context,
            ),
            &context,
        );

        let position_offset =
            FromStr::from_str(vertex_input.attribute("offset").unwrap().value()).unwrap();
        let normal_offset =
            FromStr::from_str(normal_input.attribute("offset").unwrap().value()).unwrap();
        let count: usize = FromStr::from_str(evaluate_xpath_attribute(
            Node::Element(geometry_element),
            "./c:mesh/c:triangles/@count",
            &context,
        ))
        .unwrap();

        let indices: Vec<usize> = get_text(evaluate_xpath_element(
            Node::Element(geometry_element),
            "./c:mesh/c:triangles/c:p",
            &context,
        ))
        .split_whitespace()
        .map(|s| FromStr::from_str(s).unwrap())
        .collect();
        let modulo = indices.len() / (count * 3);
        let mut triangle = Triangle {
            a: Vertex {
                normal: Vec3([0.0; 3]),
                position: Vec3([0.0; 3]),
            },
            b: Vertex {
                normal: Vec3([0.0; 3]),
                position: Vec3([0.0; 3]),
            },
            c: Vertex {
                normal: Vec3([0.0; 3]),
                position: Vec3([0.0; 3]),
            },
        };
        for (i, &index) in indices.iter().enumerate() {
            let vertex_index = (i / modulo) % 3;
            let offset = i % modulo;
            if vertex_index == 0 && offset == 0 && i != 0 {
                triangles.push(triangle);
            }

            let vertex = match vertex_index {
                0 => &mut triangle.a,
                1 => &mut triangle.b,
                2 => &mut triangle.c,
                _ => unreachable!(),
            };
            if offset == position_offset {
                vertex.position = (object_transform * positions[index].xyz1()).xyz();
            } else if offset == normal_offset {
                vertex.normal = (object_transform.inv().transpose() * normals[index].xyz0())
                    .xyz()
                    .normalize();
            }
        }
        triangles.push(triangle);
    }

    Scene {
        camera,
        triangles,
        point_lights,
    }
}

fn evaluate_xpath_attribute<'a>(node: Node<'a>, xpath: &str, context: &'a Context) -> &'a str {
    let xpath = Factory::new().build(xpath).unwrap().unwrap();
    if let Value::Nodeset(attribute_nodes) = xpath.evaluate(context, node).unwrap() {
        if let Node::Attribute(attribute) = attribute_nodes.document_order_first().unwrap() {
            attribute.value()
        } else {
            panic!("First node in result is not an attribute node.")
        }
    } else {
        panic!("XPath expression does not return a nodeset.")
    }
}

fn evaluate_xpath_element_all<'a>(
    node: Node<'a>,
    xpath: &str,
    context: &'a Context,
) -> Vec<Element<'a>> {
    let xpath = Factory::new().build(xpath).unwrap().unwrap();
    if let Value::Nodeset(nodes) = xpath.evaluate(&context, node).unwrap() {
        nodes
            .iter()
            .map(|n| {
                if let Node::Element(element) = n {
                    element
                } else {
                    panic!("Node is not an element node")
                }
            })
            .collect()
    } else {
        panic!("XPath expression does not return a nodeset")
    }
}

fn evaluate_xpath_element<'a>(node: Node<'a>, xpath: &str, context: &'a Context) -> Element<'a> {
    let xpath = Factory::new().build(xpath).unwrap().unwrap();
    if let Value::Nodeset(element_nodes) = xpath.evaluate(context, node).unwrap() {
        if let Node::Element(element) = element_nodes.document_order_first().unwrap() {
            element
        } else {
            panic!("First node in result is not an element node.")
        }
    } else {
        panic!("XPath expression does not return a nodeset.")
    }
}

fn get_text(element: Element) -> &str {
    if let ChildOfElement::Text(text) = element.children()[0] {
        text.text()
    } else {
        panic!("First child is not a text node.")
    }
}

fn get_by_url<'a>(document: &'a Document, url: &str, context: &'a Context) -> Element<'a> {
    if url.chars().nth(0).unwrap() == '#' {
        evaluate_xpath_element(
            Node::Root(document.root()),
            &format!("//*[@id=\"{}\"]", &url[1..]),
            context,
        )
    } else {
        panic!("Unknown URL.")
    }
}

fn get_transform_of_node(node: Element, context: &Context) -> Mat4 {
    let matrix_str = get_text(evaluate_xpath_element(
        Node::Element(node),
        "./c:matrix[@sid=\"transform\"]",
        context,
    ));
    let f: Vec<_> = matrix_str
        .split_whitespace()
        .map(|s| FromStr::from_str(s).unwrap())
        .collect();
    Mat4([
        [f[0], f[4], f[8], f[12]],
        [f[1], f[5], f[9], f[13]],
        [f[2], f[6], f[10], f[14]],
        [f[3], f[7], f[11], f[15]],
    ])
}

fn get_vec3s_of_source(node: Element, context: &Context) -> Vec<Vec3> {
    let document = node.document();
    let float_array_url = evaluate_xpath_attribute(
        Node::Element(node),
        "./c:technique_common/c:accessor/@source",
        context,
    );
    let float_array_str = get_text(get_by_url(&document, float_array_url, context));
    let mut v = Vec3([0.0; 3]);
    let mut at = 0;
    let mut result = Vec::new();
    for f in float_array_str.split_whitespace() {
        v.0[at] = FromStr::from_str(f).unwrap();
        at += 1;
        if at == 3 {
            at = 0;
            result.push(v);
        }
    }
    result
}
