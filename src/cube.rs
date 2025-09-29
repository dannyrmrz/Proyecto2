use raylib::prelude::Vector3;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::material::Material;

pub struct Cube {
    pub center: Vector3,
    pub size: f32,
    pub material: Material,
}

impl Cube {
    fn get_uv(&self, point: &Vector3, normal: &Vector3) -> (f32, f32) {
        let half_size = self.size * 0.5;
        let local_point = *point - self.center;
        
        // Determine which face we hit based on the normal
        let abs_normal = Vector3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        
        if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
            // Hit X face
            let u = (local_point.z + half_size) / self.size;
            let v = (local_point.y + half_size) / self.size;
            (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
        } else if abs_normal.y > abs_normal.z {
            // Hit Y face
            let u = (local_point.x + half_size) / self.size;
            let v = (local_point.z + half_size) / self.size;
            (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
        } else {
            // Hit Z face
            let u = (local_point.x + half_size) / self.size;
            let v = (local_point.y + half_size) / self.size;
            (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
        }
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect {
        let half_size = self.size * 0.5;
        let min = self.center - Vector3::new(half_size, half_size, half_size);
        let max = self.center + Vector3::new(half_size, half_size, half_size);
        
        let mut t_min = (min - *ray_origin) / *ray_direction;
        let mut t_max = (max - *ray_origin) / *ray_direction;
        
        // Swap if necessary
        if t_min.x > t_max.x {
            std::mem::swap(&mut t_min.x, &mut t_max.x);
        }
        if t_min.y > t_max.y {
            std::mem::swap(&mut t_min.y, &mut t_max.y);
        }
        if t_min.z > t_max.z {
            std::mem::swap(&mut t_min.z, &mut t_max.z);
        }
        
        let t_enter = t_min.x.max(t_min.y).max(t_min.z);
        let t_exit = t_max.x.min(t_max.y).min(t_max.z);
        
        if t_enter < t_exit && t_exit > 0.0 {
            let t = if t_enter > 0.0 { t_enter } else { t_exit };
            let point = *ray_origin + *ray_direction * t;
            
            // Calculate normal
            let local_point = point - self.center;
            let abs_local = Vector3::new(local_point.x.abs(), local_point.y.abs(), local_point.z.abs());
            let normal = if abs_local.x > abs_local.y && abs_local.x > abs_local.z {
                Vector3::new(local_point.x.signum(), 0.0, 0.0)
            } else if abs_local.y > abs_local.z {
                Vector3::new(0.0, local_point.y.signum(), 0.0)
            } else {
                Vector3::new(0.0, 0.0, local_point.z.signum())
            };
            
            let (u, v) = self.get_uv(&point, &normal);
            
            return Intersect::new(point, normal, t, self.material.clone(), u, v);
        }
        
        Intersect::empty()
    }
}
