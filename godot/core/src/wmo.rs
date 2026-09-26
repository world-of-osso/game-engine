//! WMO root and raw group data (WoW local coordinates, no Bevy mesh).
use crate::asset::wmo_format::parser;

pub use parser::{RawGroupData, WmoGroupHeader, WmoRootData};

pub fn parse_root(bytes: &[u8]) -> Result<WmoRootData, String> {
    if !crate::asset::adt::ChunkIter::new(bytes).any(|chunk| matches!(chunk, Ok((b"DHOM", _)))) {
        return Err("WMO root missing MOHD chunk".into());
    }
    parser::load_wmo_root(bytes)
}

pub struct Group {
    pub header: WmoGroupHeader,
    pub geometry: RawGroupData,
}

pub fn parse_group(bytes: &[u8]) -> Result<Group, String> {
    let payload = parser::find_mogp(bytes)?;
    if payload.len() < parser::MOGP_HEADER_SIZE {
        return Err(format!("MOGP payload too small: {} bytes", payload.len()));
    }
    let header = parser::parse_mogp_header(payload)?;
    let geometry = parser::parse_group_subchunks(&payload[parser::MOGP_HEADER_SIZE..])?;
    Ok(Group { header, geometry })
}
