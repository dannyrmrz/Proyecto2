use raylib::prelude::Vector3;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::material::Material;
use std::f32::consts::PI;

pub struct Sphere {
    pub center: Vector3,
    pub radius: f32,
    pub material: Material,
}

impl Sphere {
    fn get_uv(&self, point: &Vector3) -> (f32, f32) {
        let normalized = (*point - self.center) / self.radius;
        let u = 0.5 + normalized.x.atan2(normalized.z) / (2.0 * PI);
        let v = 0.5 - normalized.y.asin() / PI;
        (u, v)
    }
}

impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect {
        let oc = *ray_origin - self.center;

        let a = ray_direction.dot(*ray_direction);
        let b = 2.0 * oc.dot(*ray_direction);
        let c = oc.dot(oc) - self.radius * self.radius;

        let discriminant = b * b - 4.0 * a * c;

        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                let point = *ray_origin + *ray_direction * t;
                let normal = (point - self.center).normalized();
                let distance = t;
                let (u, v) = self.get_uv(&point);

                return Intersect::new(point, normal, distance, self.material.clone(), u, v);
            }
        }

        Intersect::empty()
    }
}
