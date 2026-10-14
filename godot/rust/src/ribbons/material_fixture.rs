//! Debug-only raster fixture through the production ribbon material/mesh path.
use super::{create_ribbon_material, spawn_ribbon_mesh, upload_geometry};
use game_engine_core::{m2, m2_ribbons::RibbonVertex};
use godot::{
    classes::{ArrayMesh, Image, ImageTexture, Node3D, image},
    prelude::*,
};

#[derive(GodotClass)]
#[class(base = Node3D, init)]
struct RibbonMaterialFixture {
    base: Base<Node3D>,
}

#[godot_api]
impl RibbonMaterialFixture {
    #[func]
    fn add_strip(&mut self, blend: i32, priority: i32, color: Color, depth: f32) -> GString {
        match self.spawn_strip(blend, priority, color, depth) {
            Ok(()) => GString::new(),
            Err(error) => error.into(),
        }
    }
}

impl RibbonMaterialFixture {
    fn spawn_strip(
        &mut self,
        blend: i32,
        priority: i32,
        color: Color,
        depth: f32,
    ) -> Result<(), String> {
        let authored = m2::Material {
            flags: 0x1 | 0x2 | 0x4 | 0x10, // unlit, unfogged, two-sided, no depth write
            blend_mode: u16::try_from(blend).map_err(|_| "Invalid fixture blend")?,
        };
        let image = Image::create_from_data(
            1,
            1,
            false,
            image::Format::RGBA8,
            &PackedByteArray::from(&[255, 255, 255, 255][..]),
        )
        .ok_or("Fixture image creation failed")?;
        let texture =
            ImageTexture::create_from_image(&image).ok_or("Fixture texture upload failed")?;
        let priority = i16::try_from(priority).map_err(|_| "Invalid fixture priority")?;
        let material = create_ribbon_material(&authored, &texture, priority)?;
        let vertices: Vec<_> = [
            [-1.0, -1.0],
            [1.0, -1.0],
            [-1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [1.0, 1.0],
        ]
        .iter()
        .map(|&[x, y]| RibbonVertex {
            position: [x, y, depth],
            uv: [0.5, 0.5],
            color: [color.r, color.g, color.b, color.a],
        })
        .collect();
        let mut geometry = ArrayMesh::new_gd();
        upload_geometry(&mut geometry, &vertices);
        let root = self.base().clone();
        let mut mesh = spawn_ribbon_mesh(&root, &geometry, &material);
        mesh.set_visible(true);
        Ok(())
    }
}
