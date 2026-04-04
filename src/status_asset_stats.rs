use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::Mesh;
use bevy::prelude::{Asset, Assets, StandardMaterial};

use crate::m2_effect_material::M2EffectMaterial;
use crate::mesh_asset_stats::{estimate_indices_bytes, estimate_vertex_attribute_bytes};
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
    if !mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD) {
        return 0;
    }

    mesh.attributes()
        .map(|(_, values)| estimate_vertex_attribute_bytes(values))
        .sum::<u64>()
        + mesh.indices().map_or(0, estimate_indices_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::PrimitiveTopology;

    #[test]
    fn render_world_only_mesh_reports_zero_cpu_bytes() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32, 0.0, 0.0]; 3]);

        assert_eq!(estimate_mesh_cpu_bytes(&mesh), 0);
    }
}
