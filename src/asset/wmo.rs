use std::sync::Arc;

pub use super::wmo_format::mesh_data::WmoBatchType;
use super::wmo_format::mesh_data::{self, WmoMeshBatch};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, MeshVertexAttribute, PrimitiveTopology};
use bevy::render::render_resource::VertexFormat;

pub use super::wmo_format::parser::{
    MOGP_HEADER_SIZE, RawBatch, RawGroupData, WmoDoodadDef, WmoDoodadName, WmoDoodadSet, WmoFog,
    WmoGroupHeader, WmoGroupInfo, WmoLight, WmoLightType, WmoLiquid, WmoMaterialDef,
    WmoMaterialFlags, WmoPortal, WmoPortalRef, WmoRootData, WmoRootFlags, find_mogp, load_wmo_root,
    parse_group_subchunks, parse_mogp_header, wmo_local_to_bevy,
};

const WMO_BLEND_ALPHA_ATTRIBUTE_ID: u64 = 7865676600002740225;
const WMO_THIRD_UV_ATTRIBUTE_ID: u64 = 7865676600002740226;

pub const WMO_BLEND_ALPHA_ATTRIBUTE: MeshVertexAttribute = MeshVertexAttribute::new(
    "WmoBlendAlpha",
    WMO_BLEND_ALPHA_ATTRIBUTE_ID,
    VertexFormat::Float32,
);

pub const WMO_THIRD_UV_ATTRIBUTE: MeshVertexAttribute = MeshVertexAttribute::new(
    "WmoThirdUv",
    WMO_THIRD_UV_ATTRIBUTE_ID,
    VertexFormat::Float32x2,
);

pub struct WmoGroupData {
    pub header: WmoGroupHeader,
    pub doodad_refs: Vec<u16>,
    pub light_refs: Vec<u16>,
    pub liquid: Option<WmoLiquid>,
    pub batches: Vec<WmoGroupBatch>,
    /// Collision faces for player ground (docs/specs/wmo-floor-collision.md).
    pub collision: Arc<shared::ground::WmoGroupCollision>,
}

#[derive(Clone)]
pub struct WmoGroupBatch {
    pub mesh: Mesh,
    pub material_index: u16,
    pub batch_type: WmoBatchType,
    pub uses_second_color_blend_alpha: bool,
    pub uses_second_uv_set: bool,
    pub uses_third_uv_set: bool,
    pub uses_generated_tangents: bool,
    pub has_vertex_color: bool,
}

pub fn load_wmo_group(data: &[u8]) -> Result<WmoGroupData, String> {
    load_wmo_group_with_root(data, None)
}

pub fn load_wmo_group_with_root(
    data: &[u8],
    root: Option<&WmoRootData>,
) -> Result<WmoGroupData, String> {
    let mogp_payload = find_mogp(data)?;
    if mogp_payload.len() < MOGP_HEADER_SIZE {
        return Err(format!(
            "MOGP payload too small: {} bytes",
            mogp_payload.len()
        ));
    }

    let header = parse_mogp_header(mogp_payload)?;
    let sub_chunks = &mogp_payload[MOGP_HEADER_SIZE..];
    let raw = parse_group_subchunks(sub_chunks)?;
    let collision = Arc::new(shared::ground::WmoGroupCollision::parse(data)?);
    build_group_batches(header, raw, root, collision)
}

fn build_group_batches(
    header: WmoGroupHeader,
    raw: RawGroupData,
    root: Option<&WmoRootData>,
    collision: Arc<shared::ground::WmoGroupCollision>,
) -> Result<WmoGroupData, String> {
    let batches = mesh_data::build_group_batches(&header, &raw, root)
        .into_iter()
        .map(build_bevy_batch)
        .collect();
    Ok(WmoGroupData {
        header,
        doodad_refs: raw.doodad_refs,
        light_refs: raw.light_refs,
        liquid: raw.liquid,
        batches,
        collision,
    })
}

fn build_bevy_batch(batch: WmoMeshBatch) -> WmoGroupBatch {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, batch.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, batch.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, batch.uvs);
    if let Some(second_uvs) = batch.second_uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, second_uvs);
    }
    if let Some(third_uvs) = batch.third_uvs {
        mesh.insert_attribute(WMO_THIRD_UV_ATTRIBUTE, third_uvs);
    }
    if let Some(alphas) = batch.second_color_blend_alphas {
        mesh.insert_attribute(WMO_BLEND_ALPHA_ATTRIBUTE, alphas);
    }
    if let Some(colors) = batch.colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh.insert_indices(Indices::U32(batch.indices));
    if batch.uses_generated_tangents && !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) {
        let _ = mesh.generate_tangents();
    }
    WmoGroupBatch {
        mesh,
        material_index: batch.material_index,
        batch_type: batch.batch_type,
        uses_second_color_blend_alpha: batch.uses_second_color_blend_alpha,
        uses_second_uv_set: batch.uses_second_uv_set,
        uses_third_uv_set: batch.uses_third_uv_set,
        uses_generated_tangents: batch.uses_generated_tangents,
        has_vertex_color: batch.has_vertex_color,
    }
}

#[cfg(test)]
#[path = "wmo_tests.rs"]
mod tests;
