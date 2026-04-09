pub(super) use super::*;
pub(super) use crate::asset::wmo_format::parser::{
    WmoLiquidHeader, WmoLiquidTile, WmoLiquidVertex,
};
pub(super) use bevy::ecs::system::RunSystemOnce;

mod doodads;
mod group_runtime;
mod liquid;
mod root_runtime;

pub(super) fn minimal_mat() -> wmo::WmoMaterialDef {
    wmo::WmoMaterialDef {
        texture_fdid: 0,
        texture_2_fdid: 0,
        texture_3_fdid: 0,
        flags: 0,
        material_flags: wmo::WmoMaterialFlags::default(),
        sidn_color: [0.0; 4],
        diff_color: [0.0; 4],
        ground_type: 0,
        blend_mode: 0,
        shader: 0,
        uv_translation_speed: None,
    }
}

pub(super) fn minimal_group_header() -> wmo::WmoGroupHeader {
    wmo::WmoGroupHeader {
        group_name_offset: 0,
        descriptive_group_name_offset: 0,
        flags: 0,
        group_flags: Default::default(),
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        portal_start: 0,
        portal_count: 0,
        trans_batch_count: 0,
        int_batch_count: 0,
        ext_batch_count: 0,
        batch_type_d: 0,
        fog_ids: [0; 4],
        group_liquid: 0,
        unique_id: 0,
        flags2: 0,
        parent_split_group_index: -1,
        next_split_child_group_index: -1,
    }
}

pub(super) fn minimal_group() -> wmo::WmoGroupData {
    wmo::WmoGroupData {
        header: minimal_group_header(),
        doodad_refs: Vec::new(),
        light_refs: Vec::new(),
        bsp_nodes: Vec::new(),
        bsp_face_refs: Vec::new(),
        liquid: None,
        batches: Vec::new(),
    }
}

pub(super) fn minimal_root() -> wmo::WmoRootData {
    wmo::WmoRootData {
        n_groups: 0,
        flags: Default::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: Vec::new(),
        lights: Vec::new(),
        doodad_sets: Vec::new(),
        group_names: Vec::new(),
        doodad_names: Vec::new(),
        doodad_file_ids: Vec::new(),
        doodad_defs: Vec::new(),
        fogs: Vec::new(),
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: Vec::new(),
        skybox_wow_path: None,
    }
}
