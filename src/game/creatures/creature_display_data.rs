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
