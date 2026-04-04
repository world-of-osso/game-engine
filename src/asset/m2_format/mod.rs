//! Pure M2 format parsing namespace.
//!
//! This module exposes the parser-only M2 submodules under `asset::m2_format::*`
//! and keeps the low-level binary decoding helpers here, with no Bevy deps.

pub mod m2_anim;
pub mod m2_attach;
pub mod m2_bone_names;
pub mod m2_light;
pub mod m2_particle;
pub mod parser;
pub(crate) use crate::asset::read_bytes::{
    FIXED16_SCALE, fixed16_to_f32, read_f32, read_i16, read_u16, read_u32, read_vec3,
    unorm16_to_f32,
};
pub use parser::ensure_primary_skin_path;
#[cfg(test)]
pub(crate) use parser::parse_skin_full;
pub(crate) use parser::{
    M2Chunks, M2Material, M2Submesh, M2TextureUnit, M2Vertex, SkinData, TextureTables,
    load_anim_data, load_skin_data, parse_chunks, parse_materials, parse_texture_lookup,
    parse_texture_types, parse_texture_unit_lookup, parse_transparency_lookup, parse_txid,
    parse_uv_animation_lookup, parse_vertices, resolve_indices,
};

pub(crate) const MD20_VERSION_OFFSET: usize = 0x04;
pub(crate) const MD20_GLOBAL_SEQUENCES_COUNT_OFFSET: usize = 0x14;
pub(crate) const MD20_SEQUENCES_COUNT_OFFSET: usize = 0x1C;
pub(crate) const MD20_BONES_COUNT_OFFSET: usize = 0x2C;
pub(crate) const MD20_VERTICES_COUNT_OFFSET: usize = 0x3C;
pub(crate) const MD20_COLORS_COUNT_OFFSET: usize = 0x48;
pub(crate) const MD20_TEXTURES_COUNT_OFFSET: usize = 0x50;
pub(crate) const MD20_TRANSPARENCY_COUNT_OFFSET: usize = 0x58;
pub(crate) const MD20_TEXTURE_WEIGHTS_COUNT_OFFSET: usize = 0x60;
pub(crate) const MD20_MATERIALS_COUNT_OFFSET: usize = 0x70;
pub(crate) const MD20_TEXTURE_LOOKUP_COUNT_OFFSET: usize = 0x80;
pub(crate) const MD20_TEXTURE_UNIT_LOOKUP_COUNT_OFFSET: usize = 0x88;
pub(crate) const MD20_ATTACHMENTS_COUNT_OFFSET: usize = 0xD8;
pub(crate) const MD20_ATTACHMENT_LOOKUP_COUNT_OFFSET: usize = 0xE0;
pub(crate) const MD20_PARTICLE_EMITTERS_COUNT_OFFSET: usize = 0x128;

