//! WMO root and raw group data (WoW local coordinates, no Bevy mesh).
use std::collections::BTreeMap;

use crate::asset::wmo_format::parser;
use glam::{Mat3, Quat, Vec3};

pub use crate::asset::wmo_format::mesh_data::{WmoBatchType, WmoMeshBatch};
pub use parser::{
    RawGroupData, WmoDoodadDef, WmoDoodadName, WmoDoodadSet, WmoGroupHeader, WmoRootData,
};

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

impl Group {
    pub fn batches(&self, root: Option<&WmoRootData>) -> Vec<WmoMeshBatch> {
        crate::asset::wmo_format::mesh_data::build_group_batches(&self.header, &self.geometry, root)
    }
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

/// Where a WMO doodad's M2 comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum WmoDoodadModel {
    FileId(u32),
    /// A MODN path, for roots without MODI.
    Path(String),
}

/// One MODD doodad a WMO placement draws, in WMO-local engine axes.
#[derive(Clone, Debug, PartialEq)]
pub struct WmoDoodad {
    /// MODD index.
    pub index: u16,
    /// Groups whose MODR references the doodad; it is drawn while one of them is.
    pub groups: Vec<u16>,
    pub model: WmoDoodadModel,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    /// MODD colour as RGBA.
    pub color: [f32; 4],
}

/// The MODD doodads a WMO placement draws, each once in MODD order: those the groups
/// (group index, MODR) reference that lie in doodad set 0 (`$DefaultGlobal`, always active) or
/// the placement's MODF `doodad_set` (WebWowViewerCpp `wmoObject.cpp` `getDoodad`,
/// `setLoadingParam`). A root without MODS places every referenced doodad.
pub fn placed_doodads<'a>(
    root: &WmoRootData,
    group_refs: impl IntoIterator<Item = (u16, &'a [u16])>,
    doodad_set: u16,
) -> Vec<WmoDoodad> {
    let mut referenced: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
    for (group, refs) in group_refs {
        for &index in refs {
            let groups = referenced.entry(index).or_default();
            if !groups.contains(&group) {
                groups.push(group);
            }
        }
    }
    referenced
        .into_iter()
        .filter(|&(index, _)| in_active_set(root, doodad_set, index))
        .filter_map(|(index, groups)| {
            let def = root.doodad_defs.get(index as usize)?;
            Some(WmoDoodad {
                index,
                groups,
                model: doodad_model(root, def.name_offset)?,
                translation: Vec3::from_array(parser::wmo_local_to_bevy(
                    def.position[0],
                    def.position[1],
                    def.position[2],
                )),
                rotation: wmo_local_rotation(def.rotation),
                scale: def.scale,
                color: def.color,
            })
        })
        .collect()
}

fn in_active_set(root: &WmoRootData, doodad_set: u16, index: u16) -> bool {
    if root.doodad_sets.is_empty() {
        return true;
    }
    let contains = |set: Option<&WmoDoodadSet>| {
        set.is_some_and(|set| {
            (set.start_doodad..set.start_doodad.saturating_add(set.n_doodads))
                .contains(&u32::from(index))
        })
    };
    contains(root.doodad_sets.first()) || contains(root.doodad_sets.get(doodad_set as usize))
}

/// With MODI, MODD `name_offset` indexes it (WebWowViewerCpp
/// `doodadFileDataIds[doodadDef->name_offset]`); otherwise it is a MODN byte offset.
fn doodad_model(root: &WmoRootData, name_offset: u32) -> Option<WmoDoodadModel> {
    if !root.doodad_file_ids.is_empty() {
        return root
            .doodad_file_ids
            .get(name_offset as usize)
            .copied()
            .filter(|&fdid| fdid != 0)
            .map(WmoDoodadModel::FileId);
    }
    root.doodad_names
        .iter()
        .find(|name| name.offset == name_offset)
        .map(|name| WmoDoodadModel::Path(name.name.clone()))
}

/// MODD quaternion (x, y, z, w) in WoW-local axes, conjugated into the engine basis
/// that `wmo_local_to_bevy` maps (x, y, z) to (x, z, -y).
fn wmo_local_rotation([x, y, z, w]: [f32; 4]) -> Quat {
    let wow = Quat::from_xyzw(x, y, z, w);
    let wow = if wow.length_squared() > f32::EPSILON {
        wow.normalize()
    } else {
        Quat::IDENTITY
    };
    let basis = Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y);
    Quat::from_mat3(&(basis * Mat3::from_quat(wow) * basis.transpose())).normalize()
}
