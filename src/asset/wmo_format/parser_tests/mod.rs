pub(super) use super::*;

mod ambient_lights;
mod group_chunks;
mod malformed;
mod material_flags;
mod materials;
mod root_chunks;

pub(super) const SAMPLE_GROUP_FLAGS: u32 = 0x0102_0304;
pub(super) const INTERIOR_GROUP_FLAG: u32 = 0x2000;
pub(super) const BSP_NODE_FLAGS: u16 = 0x0006;
pub(super) const BSP_GROUP_NODE_FLAGS: u16 = 0x0004;
pub(super) const SECOND_UV_FLAG: u32 = 0x0200_0000;
pub(super) const THIRD_UV_FLAG: u32 = 0x4000_0000;
pub(super) const SECOND_COLOR_BLEND_ALPHA_FLAG: u32 = 0x0100_0000;
pub(super) const ROOT_ALL_FLAG_BITS: u16 = 0x000F;
pub(super) const ROOT_RENDER_AND_ALPHA_FIX_FLAG_BITS: u16 = 0x000A;
pub(super) const DOODAD_FLAGS_AND_NAME_OFFSET: u32 = 0x1200002A;
pub(super) const ROOT_DOODAD_FLAGS_AND_NAME_OFFSET: u32 = 0x0100000B;

pub(super) fn append_chunk(data: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    data.extend_from_slice(tag);
    data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    data.extend_from_slice(payload);
}
