use super::bvh::{Bvh, BvhChild, BvhNode};
use crate::math::{AlmostEq, Plane, Vec2, Vec3};
use crate::scene::Geometry;
use std::arch::x86_64::*;
use std::f64::{INFINITY, NEG_INFINITY};

pub struct RayShootResult {
    pub geometry: Geometry,
    pub position: Vec3,
    pub normal: Vec3,
    pub lambda: f64,
    pub tex_coord: Vec2,
}

pub struct RayTracer<'a> {
    bvh: &'a Bvh<Geometry>,
    todo_stack: Vec<BvhNode<'a, Geometry>>,
}

impl<'a> RayTracer<'a> {
    pub fn new(bvh: &Bvh<Geometry>) -> RayTracer {
        RayTracer { bvh, todo_stack: Vec::with_capacity(1024) }
    }

    pub fn trace_ray(
        &mut self,
        ray_origin: Vec3,
        ray: Vec3,
        min_dist: f64,
        mut max_dist: f64,
    ) -> Option<RayShootResult> {
        let mut result: Option<RayShootResult> = None;

        let ray_origin_x = unsafe { _mm256_broadcast_sd(&ray_origin.0[0]) };
        let ray_origin_y = unsafe { _mm256_broadcast_sd(&ray_origin.0[1]) };
        let ray_origin_z = unsafe { _mm256_broadcast_sd(&ray_origin.0[2]) };
        let ray_x = unsafe { _mm256_broadcast_sd(&(1.0 / ray.0[0])) };
        let ray_y = unsafe { _mm256_broadcast_sd(&(1.0 / ray.0[1])) };
        let ray_z = unsafe { _mm256_broadcast_sd(&(1.0 / ray.0[2])) };

        self.todo_stack.clear();
        self.todo_stack.push(self.bvh.root());
        while let Some(bvh) = self.todo_stack.pop() {
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
                let pred = _mm256_or_si256(
                    lambda_check,
                    _mm256_or_si256(lambda_min_check, lambda_max_check),
                );

                let mut result = std::mem::uninitialized();
                _mm256_store_si256(&mut result, pred);
                std::mem::transmute::<__m256i, [u64; 4]>(result)
            };

            for (i, hit) in hits.iter().enumerate() {
                if *hit == 0 {
                    match bvh.value(i) {
                        BvhChild::Empty => {}
                        BvhChild::Subtree(sub_bvh) => {
                            self.todo_stack.push(sub_bvh);
                        }
                        BvhChild::Value(Geometry::Triangle(triangle)) => {
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

                            let normal = triangle.a().normal * alpha
                                + triangle.b().normal * beta
                                + triangle.c().normal * gamma;
                            if normal.dot(ray) > 0.0 {
                                continue;
                            }
                            let normal = normal.normalize();

                            let tex_coord = triangle.a().tex_coord * alpha
                                + triangle.b().tex_coord * beta
                                + triangle.c().tex_coord * gamma;

                            result = Some(RayShootResult {
                                geometry: Geometry::Triangle(*triangle),
                                position: intersection,
                                normal,
                                lambda,
                                tex_coord,
                            });
                            max_dist = lambda;
                        }
                        BvhChild::Value(Geometry::PointLight(pl)) => {
                            // sphere:
                            //     (x-x0)² + (y-y0)² + (z-z0)² = r²
                            //     dot([x-x0, y-y0, z-z0], [x-x0, y-y0, z-z0]) = r²
                            //     dot([x, y, z], [x-x0, y-y0, z-z0]) - dot([x0, y0, z0], [x-x0, y-y0, z-z0]) = r²
                            //     dot([x, y, z], [x, y, z]) - 2 * dot([x, y, z], [x0, y0, z0]) + dot([x0, y0, z0], [x0, y0, z0]) = r²
                            //
                            // ray: ray_origin + lambda * ray
                            //     ray_origin = [xo,yo,zo]
                            //     ray = [xr,yr,zr]
                            //     pl.position = [x0,y0,z0]
                            //     (xo-lambda*xr-x0)² + (yo-lambda*yr-x0)² + (zo-lambda*zr-x0)² = r²
                            //     (xo-x0)² - 2*(xo-x0)*lambda*xr - lambda²*xr² + ... + ... = r²
                            //     lambda² * (xr² + yr² + zr²) + lambda * 2 * ((xo-x0)*xr + (yo-y0)*yr + (zo-z0)*zr) - r² + (xo-x0)² + (yo-y0)² + (zo-z0)² = 0
                            let a = ray.dot(ray);
                            let b = 2.0 * (ray_origin - pl.position).dot(ray);
                            let c = -pl.radius * pl.radius + (ray_origin - pl.position).sqlen();
                            // (-b +/- sqrt(b²-4ac)) / 2a
                            let lambda1 = (-b + (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);
                            let lambda2 = (-b - (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);
                            let lambda = lambda1.min(lambda2);

                            if lambda <= max_dist && lambda >= min_dist {
                                let position = ray_origin + lambda * ray;
                                result = Some(RayShootResult {
                                    geometry: Geometry::PointLight(*pl),
                                    position,
                                    normal: (position - pl.position).normalize(),
                                    lambda,
                                    tex_coord: Vec2([0.0, 0.0]),
                                });
                                max_dist = lambda;
                            }
                        }
                    }
                }
            }
        }

        result
    }
}
