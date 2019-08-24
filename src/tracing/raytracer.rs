use crate::bvh::{BvhChild, BvhNode};
use crate::math::{AlmostEq, Mat4, Plane, Vec3, EPS};
use crate::scene::{Camera, Material, Scene, Triangle};
use rand::Rng;
use std::arch::x86_64::*;
use std::f64::{consts::PI, INFINITY, NEG_INFINITY};

pub fn raytrace<R: Rng>(
    scene: &Scene,
    rng: &mut R,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Vec3> {
    let mut todo_cache = Vec::new();
    let colors: Vec<_> = rgss(&scene.camera, x, y, width, height)
        .iter()
        .map(|ray| {
            handle_ray(scene, rng, scene.camera.position, *ray, 1.0, 1024, 0.0, &mut todo_cache)
        })
        .collect();
    Some(colors.iter().fold(Vec3([0.0; 3]), |acc, val| {
        if let Some(val) = val {
            acc + (*val / (colors.len() as f64))
        } else {
            acc
        }
    }))
}

fn handle_ray<'a, R: Rng>(
    scene: &'a Scene,
    rng: &mut R,
    origin: Vec3,
    ray: Vec3,
    lambda_min: f64,
    max_bounces: usize,
    path_length: f64,
    todo_cache: &mut Vec<BvhNode<'a, Triangle>>,
) -> Option<Vec3> {
    assert!(max_bounces != std::usize::MAX);
    let bvh = scene.triangles_bvh.as_ref().unwrap().root();

    if let Some(shoot_result) = shoot_ray(bvh, origin, ray, lambda_min, INFINITY, todo_cache) {
        let (ray, ray_len) = ray.normalize_len();
        let path_length = path_length + ray_len * shoot_result.lambda;
        let n = shoot_result.weighted_normal();
        let p = shoot_result.hit_pos;
        let r = reflect_ray(ray, n);
        let triangle = shoot_result.triangle;
        let material = if max_bounces == 0 {
            anti_bounce_material(scene.material_of_triangle(triangle))
        } else {
            *scene.material_of_triangle(triangle)
        };
        let mut result_color = Vec3([0.0; 3]);

        if material.metallic > EPS {
            if let Some(color) =
                handle_ray(scene, rng, p, r, EPS, max_bounces - 1, path_length, todo_cache)
            {
                result_color += material.color * color * material.metallic;
            }
        }

        let diffuse = 1.0 - material.metallic;
        if diffuse > EPS {
            for point_light in &scene.point_lights {
                let (light_ray, light_dist) = (point_light.position - p).normalize_len();
                let cos_n_light_ray = n.dot(light_ray);
                if cos_n_light_ray <= 0.0 {
                    continue;
                }

                let sample_size = 20;
                for _ in 0..sample_size {
                    // sample from circle
                    let (r, phi) = (
                        rng.sample(rand::distributions::Uniform::new_inclusive(0.0f64, 1.0)).sqrt()
                            * point_light.radius,
                        rng.sample(rand::distributions::Uniform::new(0.0, 2.0 * PI)),
                    );

                    let circle_radius_vec = Vec3([light_ray.0[1], -light_ray.0[0], light_ray.0[2]]);
                    let sample_dest = point_light.position
                        + r * (Mat4::rotation_around_vector(light_ray, phi)
                            * circle_radius_vec.xyz0())
                        .xyz();

                    let light_shoot_result =
                        shoot_ray(bvh, p, sample_dest - p, EPS, 1.0, todo_cache);
                    if light_shoot_result.is_some() {
                        continue;
                    }

                    let path_length = path_length + light_dist;
                    let attenuation = point_light.a * path_length * path_length
                        + point_light.b * path_length
                        + point_light.c;

                    result_color += (material.color * point_light.color)
                        * (cos_n_light_ray * diffuse / attenuation / f64::from(sample_size));
                }
            }
        }

        Some(result_color)
    } else {
        None
    }
}

fn reflect_ray(ray: Vec3, n: Vec3) -> Vec3 {
    ray - 2.0 * ray.dot(n) * n
}

fn anti_bounce_material(material: &Material) -> Material {
    Material { color: material.color, specular: 0.0, metallic: 0.0 }
}

fn calc_ray(camera: &Camera, x: f64, y: f64, width: f64, height: f64) -> Vec3 {
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

fn rgss(camera: &Camera, x: f64, y: f64, width: f64, height: f64) -> Vec<Vec3> {
    let offests = [(0.125, -0.375), (0.375, 0.125), (-0.125, 0.375), (-0.375, -0.125)];

    offests
        .iter()
        .map(|offset| calc_ray(camera, x + offset.0, y + offset.1, width, height))
        .collect()
}

struct RayShootResult<'a> {
    lambda: f64,
    triangle: &'a Triangle,
    barycentric_coords: Vec3,
    hit_pos: Vec3,
}

impl<'a> RayShootResult<'a> {
    fn weighted_normal(&self) -> Vec3 {
        (self.triangle.a().normal * self.barycentric_coords.x()
            + self.triangle.b().normal * self.barycentric_coords.y()
            + self.triangle.c().normal * self.barycentric_coords.z())
        .normalize()
    }
}

fn shoot_ray<'a>(
    bvh: BvhNode<'a, Triangle>,
    ray_origin: Vec3,
    ray: Vec3,
    min_dist: f64,
    max_dist: f64,
    todo_stack: &mut Vec<BvhNode<'a, Triangle>>,
) -> Option<RayShootResult<'a>> {
    let mut result_lambda = INFINITY;
    let mut result: Option<RayShootResult> = None;

    let ray_origin_x = unsafe { _mm256_broadcast_sd(&ray_origin.0[0]) };
    let ray_origin_y = unsafe { _mm256_broadcast_sd(&ray_origin.0[1]) };
    let ray_origin_z = unsafe { _mm256_broadcast_sd(&ray_origin.0[2]) };
    let ray_x = unsafe { _mm256_broadcast_sd(&(1.0 / ray.0[0])) };
    let ray_y = unsafe { _mm256_broadcast_sd(&(1.0 / ray.0[1])) };
    let ray_z = unsafe { _mm256_broadcast_sd(&(1.0 / ray.0[2])) };

    todo_stack.clear();
    todo_stack.push(bvh);
    while !todo_stack.is_empty() {
        let bvh = todo_stack.pop().unwrap();

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
        let hits = unsafe {
            let mut lambda_min = _mm256_broadcast_sd(&NEG_INFINITY);
            let mut lambda_max = _mm256_broadcast_sd(&INFINITY);

            // X
            let a = _mm256_mul_pd(
                _mm256_sub_pd(_mm256_load_pd(bvh.aabb_min_x().as_ptr()), ray_origin_x),
                ray_x,
            );
            let b = _mm256_mul_pd(
                _mm256_sub_pd(_mm256_load_pd(bvh.aabb_max_x().as_ptr()), ray_origin_x),
                ray_x,
            );
            if ray.0[0] > 0.0 {
                lambda_min = _mm256_max_pd(lambda_min, a);
                lambda_max = _mm256_min_pd(lambda_max, b);
            } else if ray.0[0] < 0.0 {
                lambda_min = _mm256_max_pd(lambda_min, b);
                lambda_max = _mm256_min_pd(lambda_max, a);
            }

            // Y
            let a = _mm256_mul_pd(
                _mm256_sub_pd(_mm256_load_pd(bvh.aabb_min_y().as_ptr()), ray_origin_y),
                ray_y,
            );
            let b = _mm256_mul_pd(
                _mm256_sub_pd(_mm256_load_pd(bvh.aabb_max_y().as_ptr()), ray_origin_y),
                ray_y,
            );
            if ray.0[1] > 0.0 {
                lambda_min = _mm256_max_pd(lambda_min, a);
                lambda_max = _mm256_min_pd(lambda_max, b);
            } else if ray.0[1] < 0.0 {
                lambda_min = _mm256_max_pd(lambda_min, b);
                lambda_max = _mm256_min_pd(lambda_max, a);
            }

            // Z
            let a = _mm256_mul_pd(
                _mm256_sub_pd(_mm256_load_pd(bvh.aabb_min_z().as_ptr()), ray_origin_z),
                ray_z,
            );
            let b = _mm256_mul_pd(
                _mm256_sub_pd(_mm256_load_pd(bvh.aabb_max_z().as_ptr()), ray_origin_z),
                ray_z,
            );
            if ray.0[2] > 0.0 {
                lambda_min = _mm256_max_pd(lambda_min, a);
                lambda_max = _mm256_min_pd(lambda_max, b);
            } else if ray.0[2] < 0.0 {
                lambda_min = _mm256_max_pd(lambda_min, b);
                lambda_max = _mm256_min_pd(lambda_max, a);
            }

            let lambda_check =
                _mm256_castpd_si256(_mm256_cmp_pd(lambda_max, lambda_min, _CMP_LT_OQ));
            let lambda_min_check = _mm256_castpd_si256(_mm256_cmp_pd(
                lambda_min,
                _mm256_broadcast_sd(&max_dist),
                _CMP_GT_OQ,
            ));
            let lambda_max_check = _mm256_castpd_si256(_mm256_cmp_pd(
                lambda_max,
                _mm256_broadcast_sd(&min_dist),
                _CMP_LT_OQ,
            ));
            let pred =
                _mm256_or_si256(lambda_check, _mm256_or_si256(lambda_min_check, lambda_max_check));

            let mut result = std::mem::uninitialized();
            _mm256_store_si256(&mut result, pred);
            std::mem::transmute::<__m256i, [u64; 4]>(result)
        };

        for i in 0..4 {
            if hits[i] == 0 {
                match bvh.value(i) {
                    BvhChild::Empty => {}
                    BvhChild::Subtree(sub_bvh) => {
                        todo_stack.push(sub_bvh);
                    }
                    BvhChild::Value(triangle) => {
                        let Plane { a, b, c, d } = *triangle.plane();
                        // Ray equation:  ray_origin + lambda * ray

                        // Plug the ray equation(s) into the plane equation:
                        //     dot([a, b, c], ray_origin + lambda * ray) = d
                        //     dot([a, b, c], ray_origin) + lambda * dot([a, b, c], ray) = d
                        //     lambda = (d - dot([a, b, c], ray_origin)) / dot([a, b, c], ray)
                        let lambda =
                            (d - Vec3([a, b, c]).dot(ray_origin)) / Vec3([a, b, c]).dot(ray);
                        if !lambda.is_finite() || lambda < min_dist || lambda > max_dist {
                            continue;
                        }
                        let intersection = ray_origin + lambda * ray;

                        // Get the barycentric coordinates
                        let area_triangle = Vec3([a, b, c]).len();
                        let area_triangle_abi = (triangle.a().position - intersection)
                            .cross(triangle.b().position - intersection)
                            .len();
                        let area_triangle_aci = (triangle.a().position - intersection)
                            .cross(triangle.c().position - intersection)
                            .len();
                        let area_triangle_bci = (triangle.b().position - intersection)
                            .cross(triangle.c().position - intersection)
                            .len();
                        let gamma = area_triangle_abi / area_triangle;
                        let beta = area_triangle_aci / area_triangle;
                        let alpha = area_triangle_bci / area_triangle;
                        if !(alpha + beta + gamma).almost_eq(1.0) {
                            continue;
                        }

                        if lambda < result_lambda {
                            result = Some(RayShootResult {
                                lambda,
                                triangle,
                                barycentric_coords: Vec3([alpha, beta, gamma]),
                                hit_pos: intersection,
                            });
                            result_lambda = lambda;
                        }
                    }
                }
            }
        }
    }

    result
}
