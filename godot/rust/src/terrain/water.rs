//! Authored MH2O surfaces with the original procedural water material.
use game_engine_core::{adt, water_material_data};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader,
        ShaderMaterial, image, mesh,
    },
    prelude::*,
};

#[derive(Default)]
pub(super) struct WaterMaterials {
    material: Option<Gd<ShaderMaterial>>,
}

impl WaterMaterials {
    pub fn build(&mut self, root: &adt::Root) -> Result<Option<Gd<Node3D>>, String> {
        let Some(water) = &root.water else {
            return Ok(None);
        };
        let mut meshes = Vec::new();
        for (index, chunk) in water.chunks.iter().enumerate() {
            let position = root
                .chunk_positions
                .get(index)
                .ok_or("MH2O chunk has no authored position")?;
            for layer in &chunk.layers {
                let geometry = adt::build_water_geometry(*position, layer);
                if !geometry.indices.is_empty() {
                    meshes.push(build_mesh(geometry));
                }
            }
        }
        if meshes.is_empty() {
            return Ok(None);
        }
        let material = self.load_material()?;
        let mut node = Node3D::new_alloc();
        node.set_name("Water");
        for mesh in meshes {
            let mut surface = MeshInstance3D::new_alloc();
            surface.set_mesh(&mesh);
            surface.set_surface_override_material(0, &material);
            surface.set_cast_shadows_setting(
                godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF,
            );
            node.add_child(&surface);
        }
        Ok(Some(node))
    }

    pub fn sample_clock(&mut self, parent: &Gd<Node3D>) -> Result<(), String> {
        let Some(material) = &mut self.material else {
            return Ok(());
        };
        let mut clock = parent
            .get_node_or_null("/root/M2MaterialClock")
            .ok_or("ADT water requires the shared material clock")?;
        let milliseconds = clock
            .call("elapsed_time_ms", &[])
            .try_to::<f64>()
            .map_err(|error| format!("Cannot read water material time: {error}"))?;
        let seconds = ((milliseconds / 1000.0) % 3600.0) as f32;
        material.set_shader_parameter("animation_time", &seconds.to_variant());
        Ok(())
    }

    fn load_material(&mut self) -> Result<Gd<ShaderMaterial>, String> {
        if let Some(material) = &self.material {
            return Ok(material.clone());
        }
        let shader = ResourceLoader::singleton()
            .load("res://shaders/water.gdshader")
            .ok_or("Cannot load ADT water shader")?
            .try_cast::<Shader>()
            .map_err(|_| "ADT water shader resource has wrong type")?;
        let size = water_material_data::WATER_NORMAL_SIZE as i32;
        let bytes =
            PackedByteArray::from(water_material_data::generate_water_normal_rgba().as_slice());
        let image = Image::create_from_data(size, size, false, image::Format::RGBA8, &bytes)
            .ok_or("Cannot create procedural ADT water normals")?;
        let texture = ImageTexture::create_from_image(&image)
            .ok_or("Cannot upload procedural ADT water normals")?;
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        material.set_shader_parameter("normal_map", &texture.to_variant());
        self.material = Some(material.clone());
        Ok(material)
    }
}

fn build_mesh(geometry: adt::WaterGeometry) -> Gd<ArrayMesh> {
    let positions: Vec<_> = geometry
        .positions
        .into_iter()
        .map(Vector3::from_array)
        .collect();
    let normals: Vec<_> = geometry
        .normals
        .into_iter()
        .map(Vector3::from_array)
        .collect();
    let uvs: Vec<_> = geometry.uvs.into_iter().map(Vector2::from_array).collect();
    let colors: Vec<_> = geometry
        .colors
        .into_iter()
        .map(|[r, g, b, a]| Color::from_rgba(r, g, b, a))
        .collect();
    let indices: Vec<_> = geometry
        .indices
        .chunks_exact(3)
        .flat_map(|triangle| [triangle[0] as i32, triangle[2] as i32, triangle[1] as i32])
        .collect();
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &PackedVector3Array::from(positions.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::NORMAL.ord() as usize,
        &PackedVector3Array::from(normals.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::TEX_UV.ord() as usize,
        &PackedVector2Array::from(uvs.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::COLOR.ord() as usize,
        &PackedColorArray::from(colors.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::INDEX.ord() as usize,
        &PackedInt32Array::from(indices.as_slice()).to_variant(),
    );
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    mesh
}
