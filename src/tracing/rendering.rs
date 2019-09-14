use super::raytracer::{RayShootResult, RayTracer};
use crate::math::{Mat4, Vec2, Vec3, EPS};
use crate::scene::{Bsdf, Camera, Geometry, Scene};
use rand::Rng;
use std::f64::consts::PI;
use std::f64::INFINITY;

pub fn render_subpixel<R: Rng>(
    scene: &Scene,
    rng: &mut R,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    ray_tracer: &mut RayTracer,
) -> Option<Vec3> {
    let ray = calc_ray(&scene.camera, x, y, width, height);
    handle_ray(scene, rng, scene.camera.position, ray, 1.0, 1024, ray_tracer)
}

fn handle_ray<'a, R: Rng>(
    scene: &'a Scene,
    rng: &mut R,
    origin: Vec3,
    ray: Vec3,
    lambda_min: f64,
    max_bounces: usize,
    ray_tracer: &mut RayTracer,
) -> Option<Vec3> {
    assert!(max_bounces != std::usize::MAX);

    if let Some(RayShootResult { geometry, normal: n, position: p, .. }) =
        ray_tracer.trace_ray(origin, ray, lambda_min, INFINITY)
    {
        match geometry {
            Geometry::Triangle(triangle) => {
                let r = reflect_ray(ray.normalize(), n);
                let bsdf = scene.evaluate_material(&triangle, Vec2([0.0, 0.0]));
                let bsdf = if max_bounces == 0 { anti_bounce_material(&bsdf) } else { bsdf };
                let mut result_color = Vec3([0.0; 3]);

                let mut specular = bsdf.specular;
                if specular > EPS || bsdf.metallic > EPS {
                    if let Some(color) =
                        handle_ray(scene, rng, p, r, EPS, max_bounces - 1, ray_tracer)
                    {
                        let cos_n_ray = n.dot(r);
                        specular = (specular + (1.0 - specular) * (1.0 - cos_n_ray).powi(5))
                            * (1.0 - bsdf.metallic);
                        result_color += color * (Vec3([specular; 3]) + bsdf.color * bsdf.metallic);
                    }
                }

                let diffuse = 1.0 - bsdf.metallic - specular;
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
                                rng.sample(rand::distributions::Uniform::new_inclusive(
                                    0.0f64, 1.0,
                                ))
                                .sqrt()
                                    * point_light.radius,
                                rng.sample(rand::distributions::Uniform::new(0.0, 2.0 * PI)),
                            );

                            let circle_radius_vec =
                                Vec3([light_ray.0[1], -light_ray.0[0], light_ray.0[2]]);
                            let sample_dest = point_light.position
                                + r * (Mat4::rotation_around_vector(light_ray, phi)
                                    * circle_radius_vec.xyz0())
                                .xyz();

                            let light_shoot_result =
                                ray_tracer.trace_ray(p, sample_dest - p, EPS, 1.0);
                            if let Some(RayShootResult {
                                geometry: Geometry::Triangle(_), ..
                            }) = light_shoot_result
                            {
                                continue;
                            }

                            let attenuation = 1.0 + light_dist * light_dist;
                            result_color += (bsdf.color * point_light.color)
                                * (cos_n_light_ray * diffuse
                                    / attenuation
                                    / f64::from(sample_size));
                        }
                    }
                }

                Some(result_color)
            }
            Geometry::PointLight(point_light) => Some(point_light.color),
        }
    } else {
        None
    }
}

fn reflect_ray(ray: Vec3, n: Vec3) -> Vec3 {
    ray - 2.0 * ray.dot(n) * n
}

fn anti_bounce_material(bsdf: &Bsdf) -> Bsdf {
    Bsdf { color: bsdf.color, specular: 0.0, metallic: 0.0 }
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
