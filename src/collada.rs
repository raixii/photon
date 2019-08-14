use sxd_document::parser;
use sxd_xpath::{Value, Factory, Context};
use sxd_xpath::nodeset::Node;
use sxd_document::dom::{Document, Element, ChildOfElement};
use std::str::FromStr;
use super::scene::Scene;
use super::math::Mat4;

pub fn read(xml: &str) -> Scene {
    let mut context = Context::new();
    context.set_namespace("c", "http://www.collada.org/2005/11/COLLADASchema");

    let package = parser::parse(xml).unwrap();
    let doc = package.as_document();
    let root = Node::Root(doc.root());

    let scene_instance_url = evaluate_xpath_attribute(root, "/c:COLLADA/c:scene/c:instance_visual_scene/@url", &context);
    let visual_scene = get_by_url(&doc, scene_instance_url, &context);

    let camera_element = evaluate_xpath_element(Node::Element(visual_scene), "./c:node/c:instance_camera/..", &context);
    let camera_transform = get_transform_of_node(camera_element, &context);

    println!("{:?}", camera_transform);
    unimplemented!()
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

fn evaluate_xpath_element_text<'a>(node: Node<'a>, xpath: &str, context: &'a Context) -> &'a str {
    if let ChildOfElement::Text(text) = evaluate_xpath_element(node, xpath, context).children()[0] {
        text.text()
    } else {
        panic!("First child is not a text node.")
    }
}

fn get_by_url<'a>(document: &'a Document, url: &str, context: &'a Context) -> Element<'a> {
    if url.chars().nth(0).unwrap() == '#' {
        evaluate_xpath_element(Node::Root(document.root()), &format!("//*[@id=\"{}\"]", &url[1..]), context)
    } else {
        panic!("Unknown URL.")
    }
}

fn get_transform_of_node(node: Element, context: &Context) -> Mat4 {
    let matrix_str = evaluate_xpath_element_text(Node::Element(node), "./c:matrix[@sid=\"transform\"]", context);
    let f: Vec<_> = matrix_str.split_whitespace().map(|s| FromStr::from_str(s).unwrap()).collect();
    Mat4([
        [f[0], f[4], f[8],  f[12]],
        [f[1], f[5], f[9],  f[13]],
        [f[2], f[6], f[10], f[14]],
        [f[3], f[7], f[11], f[15]],
    ])
}
