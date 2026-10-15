//! Bevy-free creature display catalog row and SQLite lookup.

use rusqlite::{Connection, OptionalExtension};

/// Per-display creature data: M2 model FDID and four DB2 texture variations.
/// The first three bind M2 types 11/12/13; the fourth binds type 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDisplay {
    pub source_product: crate::asset_product::AssetProduct,
    pub model_fdid: u32,
    /// CreatureDisplayInfo.CreatureModelAlpha: 0 transparent, 255 opaque.
    pub model_alpha: u8,
    /// CreatureModelData.CreatureGeosetDataID declares display-selected creature variants.
    pub uses_display_geosets: bool,
    pub skin_fdids: [u32; 4],
    /// CreatureDisplayInfo.CreatureModelScale × CreatureModelData.ModelScale.
    pub scale_milli: u32,
    /// Additional CreatureDisplayInfo.PetInstanceScale, applied only to battle instances.
    pub pet_instance_scale_milli: u32,
}

/// Read one imported creature display without hiding catalog errors.
pub fn query_display(
    conn: &Connection,
    display_id: u32,
) -> rusqlite::Result<Option<CreatureDisplay>> {
    let mut stmt = conn.prepare(
        "SELECT model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3, scale_milli, source_product, pet_instance_scale_milli, uses_display_geosets, model_alpha
         FROM creature_displays WHERE display_id = ?1",
    )?;
    stmt.query_row([display_id], |row| {
        Ok(CreatureDisplay {
            model_fdid: row.get(0)?,
            model_alpha: row.get(9)?,
            uses_display_geosets: row.get(8)?,
            skin_fdids: [row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?],
            scale_milli: row.get(5)?,
            source_product: row.get(6)?,
            pet_instance_scale_milli: row.get(7)?,
        })
    })
    .optional()
}

/// CDI geoset indices are zero-based; M2 variant groups start at100.
/// Declared selection disables unselected ordinary groups1–8; other groups stay
/// unchanged unless the display explicitly selects a variant of that group.
pub fn display_geoset_visible(mesh_part: u16, base_visible: bool, geosets: &[(u16, u16)]) -> bool {
    if !base_visible || mesh_part == 0 {
        return base_visible;
    }
    let group = u32::from(mesh_part) / 100;
    if let Some(&(index, value)) = geosets
        .iter()
        .find(|(index, _)| u32::from(*index) + 1 == group)
    {
        return u32::from(mesh_part) == (u32::from(index) + 1) * 100 + u32::from(value);
    }
    mesh_part >= 900
}

/// The skins a model shows by default: of the displays using `model_fdid`, the one
/// with the most populated texture slots, lowest display ID for ties (import's
/// `preferred_skins` table).
pub fn query_preferred_skins(
    conn: &Connection,
    model_fdid: u32,
) -> rusqlite::Result<Option<[u32; 4]>> {
    conn.prepare(
        "SELECT skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3
         FROM preferred_skins WHERE model_fdid = ?1",
    )?
    .query_row([model_fdid], |row| {
        Ok([row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?])
    })
    .optional()
}
