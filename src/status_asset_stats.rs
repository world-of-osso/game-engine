use bevy::image::Image;
use bevy::mesh::{Indices, Mesh, VertexAttributeValues};
use bevy::prelude::{Asset, Assets, StandardMaterial};

use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AssetStoreStats {
    pub image_assets: usize,
    pub image_asset_cpu_bytes: u64,
    pub mesh_assets: usize,
    pub mesh_asset_est_cpu_bytes: u64,
    pub standard_material_assets: usize,
    pub terrain_material_assets: usize,
    pub water_material_assets: usize,
    pub m2_effect_material_assets: usize,
}

pub fn collect_asset_store_stats(
    images: &Assets<Image>,
    meshes: &Assets<Mesh>,
    standard_materials: &Assets<StandardMaterial>,
    terrain_materials: &Assets<TerrainMaterial>,
    water_materials: &Assets<WaterMaterial>,
    m2_effect_materials: &Assets<M2EffectMaterial>,
) -> AssetStoreStats {
    AssetStoreStats {
        image_assets: asset_count(images),
        image_asset_cpu_bytes: image_bytes(images),
        mesh_assets: asset_count(meshes),
        mesh_asset_est_cpu_bytes: mesh_bytes(meshes),
        standard_material_assets: asset_count(standard_materials),
        terrain_material_assets: asset_count(terrain_materials),
        water_material_assets: asset_count(water_materials),
        m2_effect_material_assets: asset_count(m2_effect_materials),
    }
}

fn asset_count<T: Asset>(assets: &Assets<T>) -> usize {
    assets.iter().count()
}

fn image_bytes(images: &Assets<Image>) -> u64 {
    images
        .iter()
        .map(|(_, image)| image.data.as_ref().map_or(0, |data| data.len() as u64))
        .sum()
}

fn mesh_bytes(meshes: &Assets<Mesh>) -> u64 {
    meshes
        .iter()
        .map(|(_, mesh)| estimate_mesh_cpu_bytes(mesh))
        .sum()
}

fn estimate_mesh_cpu_bytes(mesh: &Mesh) -> u64 {
    mesh.attributes()
        .map(|(_, values)| estimate_vertex_attribute_bytes(values))
        .sum::<u64>()
        + mesh.indices().map_or(0, estimate_indices_bytes)
}

fn estimate_indices_bytes(indices: &Indices) -> u64 {
    match indices {
        Indices::U16(values) => slice_bytes(values.as_slice()),
        Indices::U32(values) => slice_bytes(values.as_slice()),
    }
}

fn estimate_vertex_attribute_bytes(values: &VertexAttributeValues) -> u64 {
    match values {
        VertexAttributeValues::Float32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm8x4(values) => slice_bytes(values.as_slice()),
    }
}

fn slice_bytes<T>(values: &[T]) -> u64 {
    std::mem::size_of_val(values) as u64
}
