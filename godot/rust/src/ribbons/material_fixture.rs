//! Debug-only raster fixture through the production ribbon material/mesh path.
use super::{
    create_ribbon_material, place_ribbon, spawn_ribbon_mesh, update_texture_transforms,
    upload_geometry,
};
use game_engine_core::{
    m2,
    m2_material::{MaterialTime, MaterialTracks},
    m2_ribbons::{self, RibbonVertex},
};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ProjectSettings, ShaderMaterial,
        image,
    },
    prelude::*,
};

fn read_real_uv_model() -> Result<m2::Model, String> {
    let directory = ProjectSettings::singleton()
        .globalize_path("res://../data/models")
        .to_string();
    let read = |filename: &str| {
        std::fs::read(std::path::Path::new(&directory).join(filename))
            .map_err(|error| format!("Read real ribbon UV fixture {filename}: {error}"))
    };
    m2::parse_model(&read("1865728.m2")?, &read("186572800.skin")?)
}

fn create_uv_fixture_texture() -> Result<Gd<ImageTexture>, String> {
    // Two red pixels above two green pixels; the source track shifts V by -0.5.
    let pixels = [
        255, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
    ];
    let image = Image::create_from_data(
        2,
        2,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(&pixels[..]),
    )
    .ok_or("UV fixture image creation failed")?;
    ImageTexture::create_from_image(&image).ok_or_else(|| "UV fixture texture upload failed".into())
}

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
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn spawn_chaos_bolt_ribbons(&mut self) -> GString {
        match self.create_chaos_bolt_ribbons() {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn sample_real_uv(&mut self, global_ms: u32) -> GString {
        match self.update_real_uv(global_ms) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }
}

impl RibbonMaterialFixture {
    fn create_chaos_bolt_ribbons(&mut self) -> Result<(), String> {
        let base = ProjectSettings::singleton()
            .globalize_path("res://../data")
            .to_string();
        let base = std::path::Path::new(&base);
        let read =
            |name: &str| std::fs::read(base.join("models").join(name)).map_err(|e| e.to_string());
        let model = m2::parse_model(&read("1863946.m2")?, &read("186394600.skin")?)?;
        let root = self.to_gd().upcast::<Node3D>();
        for source in &model.ribbon_emitters {
            place_ribbon(&model, source, &root, &base.join("textures"))?;
        }
        Ok(())
    }

    fn update_real_uv(&mut self, global_ms: u32) -> Result<(), String> {
        let model = read_real_uv_model()?;
        let index = m2_ribbons::texture_transform_index(&model, &model.ribbon_emitters[0], 0)?;
        let mesh = self
            .base()
            .try_get_node_as::<MeshInstance3D>("M2RibbonStrip")
            .ok_or("Real-UV fixture strip missing")?;
        let mut material = mesh
            .get_material_override()
            .and_then(|material| material.try_cast::<ShaderMaterial>().ok())
            .ok_or("Real-UV fixture material missing")?;
        let texture = create_uv_fixture_texture()?;
        material.set_shader_parameter("ribbon_texture", &texture.to_variant());
        let mut materials = [(material, index)];
        update_texture_transforms(
            &mut materials,
            &MaterialTracks::of(&model),
            MaterialTime {
                global_ms,
                ..Default::default()
            },
        );
        Ok(())
    }

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
            uv: [0.75, 0.75],
            color: [color.r, color.g, color.b, color.a],
        })
        .collect();
        let mut geometry = ArrayMesh::new_gd();
        upload_geometry(&mut geometry, &vertices);
        let root = self.to_gd().upcast::<Node3D>();
        let mut mesh = spawn_ribbon_mesh(&root, &geometry, &material);
        mesh.set_visible(true);
        Ok(())
    }
}
