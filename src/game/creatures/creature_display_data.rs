//! Bevy-free creature display catalog row and SQLite lookup.

use rusqlite::{Connection, OptionalExtension};

/// Per-display creature data: M2 model FDID and up to 3 skin texture FDIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDisplay {
    pub model_fdid: u32,
    pub skin_fdids: [u32; 3],
    pub scale_milli: u32,
}

/// Read one imported creature display without hiding catalog errors.
pub fn query_display(
    conn: &Connection,
    display_id: u32,
) -> rusqlite::Result<Option<CreatureDisplay>> {
    let mut stmt = conn.prepare(
        "SELECT model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, scale_milli
         FROM creature_displays WHERE display_id = ?1",
    )?;
    stmt.query_row([display_id], |row| {
        Ok(CreatureDisplay {
            model_fdid: row.get(0)?,
            skin_fdids: [row.get(1)?, row.get(2)?, row.get(3)?],
            scale_milli: row.get(4)?,
        })
    })
    .optional()
}

/// The skins a model shows by default: of the displays using `model_fdid`, the one
/// with the most populated texture slots, lowest display ID for ties (import's
/// `preferred_skins` table).
pub fn query_preferred_skins(
    conn: &Connection,
    model_fdid: u32,
) -> rusqlite::Result<Option<[u32; 3]>> {
    conn.prepare(
        "SELECT skin_fdid_0, skin_fdid_1, skin_fdid_2
         FROM preferred_skins WHERE model_fdid = ?1",
    )?
    .query_row([model_fdid], |row| {
        Ok([row.get(0)?, row.get(1)?, row.get(2)?])
    })
    .optional()
}
