use noise::{Fbm, NoiseFn, Perlin};
use raylib::prelude::Vector3;
use crate::sphere::Sphere;
use crate::material::Material;

pub fn generate_terrain(
    width: i32,
    depth: i32,
    materials: &[Material],
) -> Vec<Sphere> {
    let mut spheres = Vec::new();
    let seed = rand::random::<u32>();
    let noise_fn: Fbm<Perlin> = Fbm::new(seed);

    for x in 0..width {
        for z in 0..depth {
            let (nx, nz) = (x as f64 / width as f64, z as f64 / depth as f64);
            let height = noise_fn.get([nx * 2.0, nz * 2.0]);
            let y = (height * 5.0).round() as i32;

            let material = if y > 2 {
                materials[0].clone() // bricks
            } else if y > 0 {
                materials[1].clone() // rubber
            } else {
                materials[2].clone() // ivory
            };

            spheres.push(Sphere {
                center: Vector3::new(x as f32, y as f32, z as f32),
                radius: 1.0,
                material,
            });
        }
    }

    spheres
}