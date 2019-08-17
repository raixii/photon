use crate::math::{AlmostEq, Vec3};
use crate::scene::{Camera, Scene, Triangle};

pub fn raytrace(scene: &Scene, x: f32, y: f32, width: f32, height: f32) -> Option<Vec3> {
    let ray = calc_ray(&scene.camera, x, y, width, height);
    let triangle = nearest_triangle(&scene.triangles, scene.camera.position, ray);
    if let Some(triangle) = triangle {
        let i = scene.triangles.iter().position(|t| t == triangle).unwrap();
        Some(Vec3([
            1.0 / (scene.triangles.len() as f32) * (i as f32),
            0.0,
            0.0,
        ]))
    } else {
        None
    }
}

fn calc_ray(camera: &Camera, x: f32, y: f32, width: f32, height: f32) -> Vec3 {
    let point_on_plane = {
        let p_x = camera.plane_width * x / width;
        let p_y = camera.plane_height * y / height;
        let offset_x = camera.plane_width / width / 2.0;
        let offset_y = camera.plane_height / height / 2.0;
        camera.top_left_corner
            + camera.right_vector * (p_x + offset_x)
            + camera.down_vector * (p_y + offset_y)
    };
    point_on_plane - camera.position
}

fn nearest_triangle(triangles: &[Triangle], camera_pos: Vec3, ray: Vec3) -> Option<&Triangle> {
    let mut current_candidate = None;

    for triangle in triangles {
        // (a, b, c) is the normal vector of the triangle's plane:  n = (t[1]-t[0]) x (t[2]-t[0])
        // Triangle plane:  ax + by + cz = d
        //     (a, b, c) = n.xyz
        //     d = dot(t[0], n.xyz)
        let (a, b, c, d) = {
            let n = (triangle.b.position - triangle.a.position)
                .cross(triangle.c.position - triangle.a.position);
            let d = triangle.a.position.dot(n);
            (n.x(), n.y(), n.z(), d)
        };

        // Ray equation:  camera_pos + lambda * ray

        // Plug the ray equation(s) into the plane equation:
        //     dot([a, b, c], camera_pos + lambda * ray) = d
        //     dot([a, b, c], camera_pos) + lambda * dot([a, b, c], ray) = d
        //     lambda = (d - dot([a, b, c], camera_pos)) / dot([a, b, c], ray)
        let lambda = (d - Vec3([a, b, c]).dot(camera_pos)) / Vec3([a, b, c]).dot(ray);
        if !lambda.is_finite() || lambda < 1.0 {
            continue;
        }
        let intersection = camera_pos + lambda * ray;

        // Get the barycentric coordinates
        let area_triangle = Vec3([a, b, c]).len();
        let area_triangle_abi = (triangle.a.position - intersection)
            .cross(triangle.b.position - intersection)
            .len();
        let area_triangle_aci = (triangle.a.position - intersection)
            .cross(triangle.c.position - intersection)
            .len();
        let area_triangle_bci = (triangle.b.position - intersection)
            .cross(triangle.c.position - intersection)
            .len();
        let gamma = area_triangle_abi / area_triangle;
        let beta = area_triangle_aci / area_triangle;
        let alpha = area_triangle_bci / area_triangle;
        if !(alpha + beta + gamma).almost_eq(1.0) {
            continue;
        }

        match current_candidate {
            Some((current_lambda, _)) if current_lambda < lambda => {}
            _ => current_candidate = Some((lambda, triangle)),
        }
    }

    current_candidate.map(|(_, triangle)| triangle)
}
