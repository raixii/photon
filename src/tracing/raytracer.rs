use crate::bvh;
use crate::math::{AlmostEq, Vec3};
use crate::scene::{Camera, Scene, Triangle};

pub fn raytrace(scene: &Scene, x: f32, y: f32, width: f32, height: f32) -> Option<Vec3> {
    let ray = calc_ray(&scene.camera, x, y, width, height);
    let triangle = nearest_triangle(
        scene.triangles_bvh.as_ref().unwrap(),
        scene.camera.position,
        ray,
        1.0,
    );
    if let Some((triangle, _)) = triangle {
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

fn nearest_triangle(
    bvh: &bvh::Node<Triangle>,
    ray_origin: Vec3,
    ray: Vec3,
    min_dist: f32,
) -> Option<(&Triangle, f32)> {
    // These two equations describe all lambda for which the ray is inside an AABB:
    //     aabb_min <= ray_origin + lambda * ray
    //     ray_origin + lambda * ray <= aabb_max
    // This can be rearranged to (rax > 0)
    //     (aabb_min.x - ray_origin.x) / ray.x <= lambda
    //     (aabb_min.y - ray_origin.y) / ray.y <= lambda
    //     (aabb_min.z - ray_origin.z) / ray.z <= lambda
    //     lambda <= (aabb_max.x - ray_origin.x) / ray.x
    //     lambda <= (aabb_max.y - ray_origin.y) / ray.y
    //     lambda <= (aabb_max.y - ray_origin.y) / ray.y
    // (rax < 0)
    //     (aabb_min.x - ray_origin.x) / ray.x >= lambda
    //     (aabb_min.y - ray_origin.y) / ray.y >= lambda
    //     (aabb_min.z - ray_origin.z) / ray.z >= lambda
    //     lambda >= (aabb_max.x - ray_origin.x) / ray.x
    //     lambda >= (aabb_max.y - ray_origin.y) / ray.y
    //     lambda >= (aabb_max.y - ray_origin.y) / ray.y
    // (ray = 0)
    //     aabb_min.x - ray_origin.x <= 0
    //     aabb_min.y - ray_origin.y <= 0
    //     aabb_min.z - ray_origin.z <= 0
    //     aabb_max.x - ray_origin.x >= 0
    //     aabb_max.y - ray_origin.y >= 0
    //     aabb_max.z - ray_origin.z >= 0
    let mut lambda_min = std::f32::NEG_INFINITY;
    let mut lambda_max = std::f32::INFINITY;
    for i in 0..3 {
        if ray.0[i] > 0.0 {
            lambda_min = lambda_min.max((bvh.aabb_min().0[i] - ray_origin.0[i]) / ray.0[i]);
            lambda_max = lambda_max.min((bvh.aabb_max().0[i] - ray_origin.0[i]) / ray.0[i]);
        } else if ray.0[i] < 0.0 {
            lambda_max = lambda_max.min((bvh.aabb_min().0[i] - ray_origin.0[i]) / ray.0[i]);
            lambda_min = lambda_min.max((bvh.aabb_max().0[i] - ray_origin.0[i]) / ray.0[i]);
        } else {
            // We ignore false positive here
        }
    }
    if lambda_max < lambda_min {
        return None;
    }

    match bvh.value() {
        bvh::Value::Node(a, b) => {
            match (
                nearest_triangle(&a, ray_origin, ray, min_dist),
                nearest_triangle(&b, ray_origin, ray, min_dist),
            ) {
                (None, None) => None,
                (None, Some(b)) => Some(b),
                (Some(a), None) => Some(a),
                (Some((a, lambda_a)), Some((b, lambda_b))) => {
                    if lambda_a < lambda_b {
                        Some((a, lambda_a))
                    } else {
                        Some((b, lambda_b))
                    }
                }
            }
        }
        bvh::Value::Leaf(triangle) => {
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

            // Ray equation:  ray_origin + lambda * ray

            // Plug the ray equation(s) into the plane equation:
            //     dot([a, b, c], ray_origin + lambda * ray) = d
            //     dot([a, b, c], ray_origin) + lambda * dot([a, b, c], ray) = d
            //     lambda = (d - dot([a, b, c], ray_origin)) / dot([a, b, c], ray)
            let lambda = (d - Vec3([a, b, c]).dot(ray_origin)) / Vec3([a, b, c]).dot(ray);
            if !lambda.is_finite() || lambda < min_dist {
                return None;
            }
            let intersection = ray_origin + lambda * ray;

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
                return None;
            }

            Some((triangle, lambda))
        }
    }
}
