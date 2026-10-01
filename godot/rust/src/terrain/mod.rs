//! Native ADT geometry and streamed texture resources. World lighting/readiness are separate.
mod assets;
pub(crate) mod material;
pub(crate) mod objects;
mod probe;
mod scenery;
pub(crate) mod state;
pub(crate) mod streaming;
mod textures;
mod water;
mod wmo_liquid;

use std::fs;

use game_engine_core::adt::{self, Chunk, Geometry};
use godot::{
    classes::{ArrayMesh, MeshInstance3D, Node3D, ProjectSettings, RefCounted, mesh},
    prelude::*,
};

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct WowTerrainLoader {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for WowTerrainLoader {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl WowTerrainLoader {
    /// `(-1, -1)` uses authored chunk positions; otherwise pass tile row and column in 0..64.
    #[func]
    fn load_adt_geometry(&self, path: GString, tile_row: i32, tile_col: i32) -> VarDictionary {
        match read_terrain(&path, tile_row, tile_col) {
            Ok(node) => {
                let mut result = VarDictionary::new();
                result.set("node", &node);
                result
            }
            Err(error) => {
                let mut result = VarDictionary::new();
                result.set("error", error);
                result
            }
        }
    }

    /// Retail liquid material of MH2O `(liquid_type, liquid_object)` from the local DB2 exports
    /// and CASC textures, lit by the authored light at WoW `wow_position` and `minutes`.
    #[func]
    fn load_liquid_material(
        &self,
        liquid_type: i32,
        liquid_object: i32,
        map_id: i32,
        wow_position: Vector3,
        minutes: f32,
    ) -> VarDictionary {
        let mut result = VarDictionary::new();
        let liquid =
            u16::try_from(liquid_type).and_then(|t| Ok((t, u16::try_from(liquid_object)?)));
        let material = liquid
            .map_err(|_| "Liquid type and object must be in 0..65536".to_string())
            .and_then(|liquid| {
                read_liquid_material(liquid, map_id as u32, wow_position.to_array(), minutes)
            });
        match material {
            Ok(material) => result.set("material", &material),
            Err(error) => result.set("error", error),
        }
        result
    }
}

/// `load_liquid_material`: one MH2O liquid material from local DB2/CASC, lit at a map point.
fn read_liquid_material(
    liquid: (u16, u16),
    map_id: u32,
    wow_position: [f32; 3],
    minutes: f32,
) -> Result<Gd<godot::classes::ShaderMaterial>, String> {
    let settings = ProjectSettings::singleton();
    let data_root = std::path::PathBuf::from(settings.globalize_path("res://../data").to_string());
    let native =
        assets::NativeTerrainAssets::new(data_root.clone()).read_liquid_material(liquid)?;
    let sample = crate::lighting::assets::LightingCatalog::read(&data_root)?.sample(
        map_id,
        wow_position,
        minutes,
    )?;
    let fog = sample.fog;
    let light = crate::lighting::TerrainLight::new(sample, fog)?;
    water::WaterMaterials::default().standalone(&native, &light)
}

fn tile_coordinates(row: i32, col: i32) -> Result<Option<(u32, u32)>, String> {
    if row == -1 && col == -1 {
        return Ok(None);
    }
    if !(0..64).contains(&row) || !(0..64).contains(&col) {
        return Err("Tile row and column must both be in 0..64, or both -1".into());
    }
    Ok(Some((row as u32, col as u32)))
}

fn read_terrain(path: &GString, row: i32, col: i32) -> Result<Gd<Node3D>, String> {
    let tile = tile_coordinates(row, col)?;
    let filename = ProjectSettings::singleton()
        .globalize_path(path)
        .to_string();
    let bytes = fs::read(&filename).map_err(|error| format!("Cannot read {path}: {error}"))?;
    let root = adt::parse_root(&bytes).map_err(|error| format!("Cannot parse {path}: {error}"))?;
    Ok(build_terrain(root.chunks, tile))
}

fn build_terrain(chunks: Vec<Chunk>, tile: Option<(u32, u32)>) -> Gd<Node3D> {
    let mut root = Node3D::new_alloc();
    root.set_name("TerrainGeometry");
    for chunk in chunks {
        let geometry = adt::chunk_geometry(&chunk, tile);
        if geometry.indices.is_empty() {
            continue;
        }
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_name(&format!("Chunk{}_{}", chunk.index_x, chunk.index_y));
        instance.set_mesh(&build_mesh(geometry, &chunk.vertex_colors));
        root.add_child(&instance);
    }
    root
}

fn build_mesh(geometry: Geometry, colors: &[[f32; 4]; 145]) -> Gd<ArrayMesh> {
    let vectors = |values: Vec<[f32; 3]>| {
        let values: Vec<_> = values.into_iter().map(Vector3::from_array).collect();
        PackedVector3Array::from(values.as_slice())
    };
    let positions = vectors(geometry.positions);
    let normals = vectors(geometry.normals);
    let uvs: Vec<_> = geometry.uvs.into_iter().map(Vector2::from_array).collect();
    let uvs = PackedVector2Array::from(uvs.as_slice());
    // ArrayMesh packs COLOR into byte/255; the shader restores authored byte/127.
    const MCCV_TO_VERTEX_COLOR: f32 = 127.0 / 255.0;
    let vertex_colors: Vec<_> = colors
        .iter()
        .map(|color| {
            Color::from_rgba(
                color[0] * MCCV_TO_VERTEX_COLOR,
                color[1] * MCCV_TO_VERTEX_COLOR,
                color[2] * MCCV_TO_VERTEX_COLOR,
                color[3],
            )
        })
        .collect();
    let vertex_colors = PackedColorArray::from(vertex_colors.as_slice());
    // Godot ArrayMesh front faces are clockwise; Bevy's authored indices are counterclockwise.
    let indices: Vec<i32> = geometry
        .indices
        .chunks_exact(3)
        .flat_map(|triangle| [triangle[0], triangle[2], triangle[1]])
        .map(|index| index as i32)
        .collect();
    let indices = PackedInt32Array::from(indices.as_slice());
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &positions.to_variant(),
    );
    arrays.set(
        mesh::ArrayType::NORMAL.ord() as usize,
        &normals.to_variant(),
    );
    arrays.set(
        mesh::ArrayType::COLOR.ord() as usize,
        &vertex_colors.to_variant(),
    );
    arrays.set(mesh::ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    arrays.set(mesh::ArrayType::INDEX.ord() as usize, &indices.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    mesh
}
