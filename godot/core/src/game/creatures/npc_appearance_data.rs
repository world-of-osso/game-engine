use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredNpcAppearance {
    pub race: u8,
    pub sex: u8,
    pub class: u8,
    pub baked_texture_fdid: Option<u32>,
    pub choice_ids: Vec<u32>,
    pub geosets: Vec<(u16, u16)>,
}

pub fn query_authored_npc_appearance(
    connection: &Connection,
    display_id: u32,
) -> Result<Option<AuthoredNpcAppearance>, String> {
    let context =
        |error| format!("query authored NPC appearance for display {display_id}: {error}");
    let required = connection
        .query_row(
            "SELECT requires_appearance FROM display_coverage WHERE display_id = ?1",
            [display_id],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(context)?
        .ok_or_else(|| {
            format!("NPC display {display_id} is outside imported appearance coverage")
        })?;
    if !required {
        return Ok(None);
    }
    let appearance = query_appearance(connection, display_id).map_err(context)?;
    appearance
        .map(Some)
        .ok_or_else(|| format!("NPC display {display_id} has no required appearance profile"))
}

fn query_appearance(
    connection: &Connection,
    display_id: u32,
) -> rusqlite::Result<Option<AuthoredNpcAppearance>> {
    let appearance = connection
        .query_row(
            "SELECT race, sex, class, baked_texture_fdid FROM appearances WHERE display_id = ?1",
            [display_id],
            |row| {
                let baked_texture_fdid: u32 = row.get(3)?;
                Ok(AuthoredNpcAppearance {
                    race: row.get(0)?,
                    sex: row.get(1)?,
                    class: row.get(2)?,
                    baked_texture_fdid: (baked_texture_fdid != 0).then_some(baked_texture_fdid),
                    choice_ids: Vec::new(),
                    geosets: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut appearance) = appearance else {
        return Ok(None);
    };
    appearance.choice_ids = query_choice_ids(connection, display_id)?;
    appearance.geosets = query_geosets(connection, display_id)?;
    Ok(Some(appearance))
}

fn query_choice_ids(connection: &Connection, display_id: u32) -> rusqlite::Result<Vec<u32>> {
    let mut statement = connection
        .prepare("SELECT choice_id FROM choices WHERE display_id = ?1 ORDER BY choice_id")?;
    statement
        .query_map([display_id], |row| row.get(0))?
        .collect()
}

fn query_geosets(connection: &Connection, display_id: u32) -> rusqlite::Result<Vec<(u16, u16)>> {
    let mut statement = connection.prepare(
        "SELECT geoset_index, geoset_value FROM geosets
         WHERE display_id = ?1 ORDER BY geoset_index, geoset_value",
    )?;
    statement
        .query_map([display_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}
