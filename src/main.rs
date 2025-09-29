use raylib::prelude::*;
use std::f32::consts::PI;

mod framebuffer;
mod ray_intersect;
mod sphere;
mod cube;
mod camera;
mod light;
mod material;
mod textures;
mod procedural;

use framebuffer::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use cube::Cube;
use camera::Camera;
use light::Light;
use material::{Material, vector3_to_color};
use textures::TextureManager;

const ORIGIN_BIAS: f32 = 1e-4;

fn skybox_color(dir: Vector3, texture_manager: &TextureManager) -> Vector3 {
    let d = dir.normalized();
    
    // Obtener color del skybox si existe la textura
    if let Some(skybox_texture) = texture_manager.get_texture("assets/skybox.jpg") {
        // Convertir dirección 3D a coordenadas UV para el skybox
        let u = 0.5 + d.x.atan2(d.z) / (2.0 * std::f32::consts::PI);
        let v = 0.5 - d.y.asin() / std::f32::consts::PI;
        
        let width = skybox_texture.width() as u32;
        let height = skybox_texture.height() as u32;
        let tx = (u * width as f32) as u32;
        let ty = (v * height as f32) as u32;
        texture_manager.get_pixel_color("assets/skybox.jpg", tx, ty)
    } else {
        // Fallback a cielo procedural estilo Skyblock
        let t = (d.y + 1.0) * 0.5;
        let sky_blue = Vector3::new(0.4, 0.6, 1.0);
        let horizon_white = Vector3::new(0.9, 0.9, 1.0);
        let cloud_white = Vector3::new(1.0, 1.0, 1.0);
        
        if t < 0.3 {
            // Horizonte
            let k = t / 0.3;
            horizon_white * (1.0 - k) + sky_blue * k
        } else if t < 0.7 {
            // Cielo con nubes
            let k = (t - 0.3) / 0.4;
            let cloud_factor = (d.x * 3.0).sin() * (d.z * 2.0).cos() * 0.1;
            let base_color = sky_blue * (1.0 - k) + cloud_white * k;
            base_color + Vector3::new(cloud_factor, cloud_factor, cloud_factor)
        } else {
            // Cielo superior
            sky_blue
        }
    }
}

fn offset_origin(intersect: &Intersect, direction: &Vector3) -> Vector3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(intersect.normal) < 0.0 {
        intersect.point - offset
    } else {
        intersect.point + offset
    }
}

fn reflect(incident: &Vector3, normal: &Vector3) -> Vector3 {
    *incident - *normal * 2.0 * incident.dot(*normal)
}

fn refract(incident: &Vector3, normal: &Vector3, refractive_index: f32) -> Option<Vector3> {
    let mut cosi = incident.dot(*normal).max(-1.0).min(1.0);
    let mut etai = 1.0;
    let mut etat = refractive_index;
    let mut n = *normal;

    if cosi > 0.0 {
        std::mem::swap(&mut etai, &mut etat);
        n = -n;
    } else {
        cosi = -cosi;
    }

    let eta = etai / etat;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);

    if k < 0.0 {
        None
    } else {
        Some(*incident * eta + n * (eta * cosi - k.sqrt()))
    }
}

fn cast_shadow(
    intersect: &Intersect,
    light: &Light,
    objects: &[Cube],
) -> f32 {
    let light_dir = (light.position - intersect.point).normalized();
    let light_distance = (light.position - intersect.point).length();

    let shadow_ray_origin = offset_origin(intersect, &light_dir);

    for object in objects {
        let shadow_intersect = object.ray_intersect(&shadow_ray_origin, &light_dir);
        if shadow_intersect.is_intersecting && shadow_intersect.distance < light_distance {
            return 1.0;
        }
    }

    0.0
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[Cube],
    light: &Light,
    texture_manager: &TextureManager,
    depth: u32,
) -> Vector3 {
    if depth > 3 {
        return skybox_color(*ray_direction, texture_manager);
    }

    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;

    for object in objects {
        let i = object.ray_intersect(ray_origin, ray_direction);
        if i.is_intersecting && i.distance < zbuffer {
            zbuffer = i.distance;
            intersect = i;
        }
    }

    if !intersect.is_intersecting {
        return skybox_color(*ray_direction, texture_manager);
    }

    let light_dir = (light.position - intersect.point).normalized();
    let view_dir = (*ray_origin - intersect.point).normalized();

    let mut normal = intersect.normal;
    if let Some(normal_map_path) = &intersect.material.normal_map_id {
        let texture = texture_manager.get_texture(normal_map_path).unwrap();
        let width = texture.width() as u32;
        let height = texture.height() as u32;
        let tx = (intersect.u * width as f32) as u32;
        let ty = (intersect.v * height as f32) as u32;

        if let Some(tex_normal) = texture_manager.get_normal_from_map(normal_map_path, tx, ty) {
            let tangent = Vector3::new(normal.y, -normal.x, 0.0).normalized();
            let bitangent = normal.cross(tangent);
            
            let transformed_normal_x = tex_normal.x * tangent.x + tex_normal.y * bitangent.x + tex_normal.z * normal.x;
            let transformed_normal_y = tex_normal.x * tangent.y + tex_normal.y * bitangent.y + tex_normal.z * normal.y;
            let transformed_normal_z = tex_normal.x * tangent.z + tex_normal.y * bitangent.z + tex_normal.z * normal.z;

            normal = Vector3::new(transformed_normal_x, transformed_normal_y, transformed_normal_z).normalized();
        }
    }

    let reflect_dir = reflect(&-light_dir, &normal).normalized();

    let shadow_intensity = cast_shadow(&intersect, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    let diffuse_color = if let Some(texture_path) = &intersect.material.texture_id {
        let texture = texture_manager.get_texture(texture_path).unwrap();
        let width = texture.width() as u32;
        let height = texture.height() as u32;
        let tx = (intersect.u * width as f32) as u32;
        let ty = (intersect.v * height as f32) as u32;
        let color = texture_manager.get_pixel_color(texture_path, tx, ty);
        color
    } else {
        intersect.material.diffuse
    };

    let diffuse_intensity = normal.dot(light_dir).max(0.0) * light_intensity;
    let diffuse = diffuse_color * diffuse_intensity;

    let specular_intensity = view_dir.dot(reflect_dir).max(0.0).powf(intersect.material.specular) * light_intensity;
    let light_color_v3 = Vector3::new(light.color.r as f32 / 255.0, light.color.g as f32 / 255.0, light.color.b as f32 / 255.0);
    let specular = light_color_v3 * specular_intensity;

    let albedo = intersect.material.albedo;
    let phong_color = diffuse * albedo[0] + specular * albedo[1] + intersect.material.emissive;

    let reflectivity = intersect.material.albedo[2];
    let reflect_color = if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &normal).normalized();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, depth + 1)
    } else {
        Vector3::zero()
    };

    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        if let Some(refract_dir) = refract(ray_direction, &normal, intersect.material.refractive_index) {
            let refract_origin = offset_origin(&intersect, &refract_dir);
            cast_ray(&refract_origin, &refract_dir, objects, light, texture_manager, depth + 1)
        } else {
            let reflect_dir = reflect(ray_direction, &normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            cast_ray(&reflect_origin, &reflect_dir, objects, light, texture_manager, depth + 1)
        }
    } else {
        Vector3::zero()
    };

    phong_color * (1.0 - reflectivity - transparency) + reflect_color * reflectivity + refract_color * transparency
}

pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Cube],
    camera: &Camera,
    light: &Light,
    texture_manager: &TextureManager,
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            let screen_x = screen_x * aspect_ratio * perspective_scale;
            let screen_y = screen_y * perspective_scale;

            let ray_direction = Vector3::new(screen_x, screen_y, -1.0).normalized();
            
            let rotated_direction = camera.basis_change(&ray_direction);

            let pixel_color_v3 = cast_ray(&camera.eye, &rotated_direction, objects, light, texture_manager, 0);
            let pixel_color = vector3_to_color(pixel_color_v3);

            framebuffer.set_current_color(pixel_color);
            framebuffer.set_pixel(x, y);
        }
    }
}

fn main() {
    let window_width = 1300;
    let window_height = 900;
 
    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raytracer Example")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    let mut texture_manager = TextureManager::new();
    // Cargar texturas para la isla Skyblock
    texture_manager.load_texture(&mut window, &thread, "assets/wood.jpg");
    texture_manager.load_texture(&mut window, &thread, "assets/leaves.jpg");
    texture_manager.load_texture(&mut window, &thread, "assets/water.jpg");
    texture_manager.load_texture(&mut window, &thread, "assets/stone.jpg");
    texture_manager.load_texture(&mut window, &thread, "assets/dirt.jpg");
    texture_manager.load_texture(&mut window, &thread, "assets/glass.jpg");
    // Cargar skybox si existe, sino usar fallback procedural
    if std::path::Path::new("assets/skybox.jpg").exists() {
        texture_manager.load_texture(&mut window, &thread, "assets/skybox.jpg");
    }
    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    // Material 1: Madera (wood.jpg) - Tronco del árbol
    let wood = Material::new(
        Vector3::new(0.6, 0.4, 0.2), // Color marrón
        5.0, // Specular bajo
        [0.8, 0.1, 0.0, 0.0], // Albedo: difuso alto, specular bajo, sin reflexión ni transparencia
        0.0, // Sin refracción
        Some("assets/wood.jpg".to_string()),
        None,
        Vector3::zero(),
    );

    // Material 2: Hojas (leaves.jpg) - Copa del árbol
    let leaves = Material::new(
        Vector3::new(0.2, 0.6, 0.2), // Color verde
        3.0, // Specular muy bajo
        [0.9, 0.05, 0.0, 0.0], // Albedo: difuso muy alto, specular muy bajo
        0.0, // Sin refracción
        Some("assets/leaves.jpg".to_string()),
        None,
        Vector3::zero(),
    );

    // Material 3: Agua (water.jpg) - CON REFLEXIÓN
    let water = Material::new(
        Vector3::new(0.2, 0.4, 0.8), // Color azul agua
        50.0, // Specular medio
        [0.2, 0.1, 0.7, 0.0], // Albedo: difuso bajo, specular bajo, reflexión alta
        0.0, // Sin refracción
        Some("assets/water.jpg".to_string()),
        None,
        Vector3::zero(),
    );

    // Material 4: Piedra (stone.jpg)
    let stone = Material::new(
        Vector3::new(0.5, 0.5, 0.5), // Color gris
        10.0, // Specular medio
        [0.9, 0.05, 0.0, 0.0], // Albedo: difuso muy alto, specular muy bajo
        0.0, // Sin refracción
        Some("assets/stone.jpg".to_string()),
        None,
        Vector3::zero(),
    );

    // Material 5: Tierra (dirt.jpg)
    let dirt = Material::new(
        Vector3::new(0.4, 0.3, 0.2), // Color marrón oscuro
        2.0, // Specular muy bajo
        [0.9, 0.05, 0.0, 0.0], // Albedo: difuso muy alto, specular muy bajo
        0.0, // Sin refracción
        Some("assets/dirt.jpg".to_string()),
        None,
        Vector3::zero(),
    );

    // Material 6: Cristal (glass.jpg) - CON REFRACCIÓN
    let glass = Material::new(
        Vector3::new(0.6, 0.7, 0.8), // Color azul claro
        125.0, // Specular muy alto
        [0.0, 0.1, 0.1, 0.8], // Albedo: sin difuso, specular bajo, reflexión baja, transparencia alta
        1.5, // Índice de refracción del vidrio
        Some("assets/glass.jpg".to_string()),
        None,
        Vector3::zero(),
    );

    // Material para la luz
    let light_material = Material::new(
        Vector3::new(1.0, 1.0, 1.0),
        10.0,
        [1.0, 0.0, 0.0, 0.0],
        0.0,
        None,
        None,
        Vector3::new(1.0, 1.0, 1.0) * 2.0,
    );

    // Crear isla Skyblock con cubos texturizados
    let objects = vec![
        // Base de la isla - bloques de tierra
        Cube { center: Vector3::new(0.0, -1.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(1.0, -1.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(-1.0, -1.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(0.0, -1.5, 1.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(0.0, -1.5, -1.0), size: 1.0, material: dirt.clone() },
        
        // Capa superior de la isla - bloques de tierra
        Cube { center: Vector3::new(0.0, -0.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(1.0, -0.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(-1.0, -0.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(0.0, -0.5, 1.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(0.0, -0.5, -1.0), size: 1.0, material: dirt.clone() },
        
        // Superficie de la isla - bloques de tierra
        Cube { center: Vector3::new(0.0, 0.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(1.0, 0.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(-1.0, 0.5, 0.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(0.0, 0.5, 1.0), size: 1.0, material: dirt.clone() },
        Cube { center: Vector3::new(0.0, 0.5, -1.0), size: 1.0, material: dirt.clone() },
        
        // Árbol - Tronco de madera
        Cube { center: Vector3::new(0.0, 1.5, 0.0), size: 1.0, material: wood.clone() },
        Cube { center: Vector3::new(0.0, 2.5, 0.0), size: 1.0, material: wood.clone() },
        
        // Árbol - Hojas
        Cube { center: Vector3::new(0.0, 3.5, 0.0), size: 1.0, material: leaves.clone() },
        Cube { center: Vector3::new(1.0, 3.5, 0.0), size: 1.0, material: leaves.clone() },
        Cube { center: Vector3::new(-1.0, 3.5, 0.0), size: 1.0, material: leaves.clone() },
        Cube { center: Vector3::new(0.0, 3.5, 1.0), size: 1.0, material: leaves.clone() },
        Cube { center: Vector3::new(0.0, 3.5, -1.0), size: 1.0, material: leaves.clone() },
        Cube { center: Vector3::new(0.0, 4.5, 0.0), size: 1.0, material: leaves.clone() },
        
        // Cofre de madera
        Cube { center: Vector3::new(1.5, 1.0, 1.5), size: 1.0, material: wood.clone() },
        
        // Bloques de piedra
        Cube { center: Vector3::new(-1.5, 1.0, -1.5), size: 1.0, material: stone.clone() },
        Cube { center: Vector3::new(-1.5, 2.0, -1.5), size: 1.0, material: stone.clone() },
        
        // Bloques de cristal
        Cube { center: Vector3::new(2.0, 1.0, -1.0), size: 1.0, material: glass.clone() },
        Cube { center: Vector3::new(2.0, 2.0, -1.0), size: 1.0, material: glass.clone() },
        Cube { center: Vector3::new(2.0, 1.0, 0.0), size: 1.0, material: glass.clone() },
        
        // Bloques de agua reflectante
        Cube { center: Vector3::new(-2.0, 1.0, 1.0), size: 1.0, material: water.clone() },
        Cube { center: Vector3::new(-2.0, 1.0, 0.0), size: 1.0, material: water.clone() },
        
        // Luz (sol)
        Cube { center: Vector3::new(0.0, 6.0, 0.0), size: 0.5, material: light_material.clone() },
    ];

    let mut camera = Camera::new(
        Vector3::new(0.0, 2.0, 8.0), // Cámara más alejada y elevada
        Vector3::new(0.0, 1.0, 0.0), // Mirando hacia el centro de la isla
        Vector3::new(0.0, 1.0, 0.0),
    );
    let rotation_speed = PI / 200.0; // Movimiento más suave
    let zoom_speed = 0.05; // Zoom más suave

    let light = Light::new(
        Vector3::new(1.0, -1.0, 5.0),
        Color::new(255, 255, 255, 255),
        1.5,
    );

    while !window.window_should_close() {
        let mut camera_moved = false;
        
        // Movimiento más suave con detección de teclas presionadas
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            camera.orbit(rotation_speed, 0.0);
            camera_moved = true;
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            camera.orbit(-rotation_speed, 0.0);
            camera_moved = true;
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            camera.orbit(0.0, -rotation_speed);
            camera_moved = true;
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            camera.orbit(0.0, rotation_speed);
            camera_moved = true;
        }
        if window.is_key_down(KeyboardKey::KEY_W) {
            camera.zoom(zoom_speed);
            camera_moved = true;
        }
        if window.is_key_down(KeyboardKey::KEY_S) {
            camera.zoom(-zoom_speed);
            camera_moved = true;
        }

        // Solo renderizar si la cámara se movió
        if camera_moved {
            render(&mut framebuffer, &objects, &camera, &light, &texture_manager);
        }
        
        framebuffer.swap_buffers(&mut window, &thread);
    }
}
