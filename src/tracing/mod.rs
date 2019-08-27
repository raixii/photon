use crate::math::{Vec3, Vec4};
use crate::scene::{Geometry, Scene};
use bvh::Bvh;
use crossbeam_channel::Sender;
use rand::SeedableRng;
use rendering::render_subpixel;
use std::cmp::Ordering;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, Arc};
use std::thread;
use std::time::Instant;

mod bvh;
mod raytracer;
mod rendering;

pub fn main(
    scene: Arc<Scene>,
    antialiasing: u32,
    w: usize,
    h: usize,
    thread_count: usize,
    seed: u128,
    want_quit: Arc<AtomicBool>,
    pixel_sender: Sender<(usize, usize, Vec4)>,
) {
    let start_time = Instant::now();
    let geometry = {
        let mut geometry = vec![];
        for triangle in &scene.triangles {
            geometry.push(Geometry::Triangle(*triangle));
        }
        for point_light in &scene.point_lights {
            geometry.push(Geometry::PointLight(*point_light));
        }
        geometry
    };
    let bvh = Arc::new(Bvh::new(&geometry));
    eprintln!("Building BVH: {} ms", (Instant::now() - start_time).as_millis());

    let (render_sender, render_receiver) = crossbeam_channel::unbounded();
    {
        let mut positions = vec![];
        for x in 0..w {
            for y in 0..h {
                for xaa in 0..2usize.pow(antialiasing) {
                    for yaa in 0..2usize.pow(antialiasing) {
                        positions.push(((x << antialiasing) + xaa, (y << antialiasing) + yaa));
                    }
                }
            }
        }
        positions.sort_by(|a, b| {
            let a_zeros = a.0.trailing_zeros().min(a.1.trailing_zeros());
            let b_zeros = b.0.trailing_zeros().min(b.1.trailing_zeros());
            if a_zeros > b_zeros {
                Ordering::Less
            } else if a_zeros < b_zeros {
                Ordering::Greater
            } else if a.0 < b.0 {
                Ordering::Less
            } else if a.0 > b.0 {
                Ordering::Greater
            } else if a.1 < b.1 {
                Ordering::Less
            } else if a.1 > b.1 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        assert_eq!(positions.len(), w * h * 4usize.pow(antialiasing));
        for p in positions {
            render_sender.send(p).unwrap();
        }
    }

    let start_time = Instant::now();
    let mut worker_threads = Vec::with_capacity(thread_count);
    for t in 0..thread_count {
        let scene = Arc::clone(&scene);
        let bvh = Arc::clone(&bvh);
        let want_quit = Arc::clone(&want_quit);
        let render_receiver = render_receiver.clone();
        let pixel_sender = pixel_sender.clone();
        let worker_thread = thread::Builder::new()
            .name(format!("Worker {}", t + 1))
            .spawn(move || {
                let mut rng = rand_pcg::Pcg32::from_seed(
                    seed.overflowing_mul(t as u128 + 123).0.to_be_bytes(),
                );
                let mut ray_tracer = raytracer::RayTracer::new(&bvh);

                while let Ok((my_x, my_y)) = render_receiver.try_recv() {
                    if want_quit.load(atomic::Ordering::Relaxed) {
                        break;
                    }

                    let (render_x, render_y) = if antialiasing == 0 {
                        // Use pixel center
                        (my_x as f64 + 0.5, my_y as f64 + 0.5)
                    } else {
                        // Use RGSS around the second-to-last (!!!) subpixel center

                        // First find the subpixel center
                        // pixel_left + subpixel_index * subpixel_size + subpixel_size / 2
                        // Hint: For x = 1 and aa = 1 this leads to 0.75.
                        //       For x = 0 and aa = 1 this leads to 0.25.
                        //       For x = 0 and aa = 2 this leads to 0.125.
                        //       For x = 1 and aa = 2 this leads to 0.25.
                        let subpixel_size = 1.0 / f64::from(1 << antialiasing);
                        let rgss_center_x = (my_x >> antialiasing) as f64
                            + (my_x & ((1 << antialiasing) - 1)) as f64 * subpixel_size
                            + subpixel_size / 2.0;
                        let rgss_center_y = (my_y >> antialiasing) as f64
                            + (my_y & ((1 << antialiasing) - 1)) as f64 * subpixel_size
                            + subpixel_size / 2.0;

                        // Pick one offset for each of the four remaining subpixels. Note that these
                        // offsets are relative to the subpixel center, *not* relative to the
                        // second-to-last subpixel center.
                        let (rgss_offset_x, rgss_offset_y) = [
                            (-1.0 / 8.0, 1.0 / 8.0),  // x % 2 == 0 && y % 2 == 0  =>  top-left
                            (-1.0 / 8.0, -1.0 / 8.0), // x % 2 == 1 && y % 2 == 0  =>  top-right
                            (1.0 / 8.0, 1.0 / 8.0),   // x % 2 == 0 && y % 2 == 1  =>  bottom-left
                            (1.0 / 8.0, -1.0 / 8.0),  // x % 2 == 1 && y % 2 == 1  =>  bottom-right
                        ][(my_x % 2) + 2 * (my_y % 2)];

                        // Divide the offsets to the correct subpixel size
                        let rgss_offset_x = rgss_offset_x / f64::from(1 << (antialiasing - 1));
                        let rgss_offset_y = rgss_offset_y / f64::from(1 << (antialiasing - 1));

                        (rgss_center_x + rgss_offset_x, rgss_center_y + rgss_offset_y)
                    };

                    let color = render_subpixel(
                        &scene,
                        &mut rng,
                        render_x,
                        render_y,
                        w as f64,
                        h as f64,
                        &mut ray_tracer,
                    );
                    let color = color.unwrap_or(Vec3([0.0, 0.0, 0.0])).xyz1();

                    pixel_sender.send((my_x >> antialiasing, my_y >> antialiasing, color)).unwrap();
                }
            })
            .unwrap();
        worker_threads.push(worker_thread);
    }
    for worker_thread in worker_threads {
        worker_thread.join().unwrap();
    }
    eprintln!("Raytracing: {} ms", (Instant::now() - start_time).as_millis());
}
