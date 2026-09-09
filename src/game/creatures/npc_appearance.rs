use rusqlite::{Connection, OpenFlags, OptionalExtension};

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
    query_appearance(connection, display_id)
        .map_err(|error| format!("query authored NPC appearance for display {display_id}: {error}"))
}

pub fn load_authored_npc_appearance(
    display_id: u32,
) -> Result<Option<AuthoredNpcAppearance>, String> {
    let path = game_engine::paths::resolve_data_path("cache/npc_appearance.sqlite");
    let connection =
        Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| {
            format!(
                "open authored NPC appearance cache {} for display {display_id}: {error}",
                path.display()
            )
        })?;
    query_authored_npc_appearance(&connection, display_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE appearances (
                display_id INTEGER PRIMARY KEY, race INTEGER, sex INTEGER,
                class INTEGER, baked_texture_fdid INTEGER
            );
            CREATE TABLE choices (display_id INTEGER, choice_id INTEGER);
            CREATE TABLE geosets (
                display_id INTEGER, geoset_index INTEGER, geoset_value INTEGER
            );
            INSERT INTO appearances VALUES (19177, 1, 0, 1, 1050256);
            INSERT INTO appearances VALUES (19178, 1, 1, 8, 0);
            INSERT INTO choices VALUES (19177, 9001), (19178, 65537), (19177, 300);
            INSERT INTO geosets VALUES (19177, 21, 2), (19178, 4, 3), (19177, 7, 1);",
            )
            .unwrap();
        connection
    }

    #[test]
    fn reads_distinct_appearances_with_full_choice_ids_and_sorted_geosets() {
        let connection = fixture();
        assert_eq!(
            query_authored_npc_appearance(&connection, 19177).unwrap(),
            Some(AuthoredNpcAppearance {
                race: 1,
                sex: 0,
                class: 1,
                baked_texture_fdid: Some(1050256),
                choice_ids: vec![300, 9001],
                geosets: vec![(7, 1), (21, 2)],
            }),
        );
        assert_eq!(
            query_authored_npc_appearance(&connection, 19178).unwrap(),
            Some(AuthoredNpcAppearance {
                race: 1,
                sex: 1,
                class: 8,
                baked_texture_fdid: None,
                choice_ids: vec![65537],
                geosets: vec![(4, 3)],
            }),
        );
    }

    #[test]
    fn missing_display_is_none() {
        assert_eq!(
            query_authored_npc_appearance(&fixture(), 999).unwrap(),
            None
        );
    }

    #[test]
    fn appearance_without_choices_or_geosets_retains_empty_lists() {
        let connection = fixture();
        connection
            .execute_batch("INSERT INTO appearances VALUES (42, 2, 0, 3, 4294967295);")
            .unwrap();
        let appearance = query_authored_npc_appearance(&connection, 42)
            .unwrap()
            .unwrap();
        assert_eq!(appearance.baked_texture_fdid, Some(u32::MAX));
        assert!(appearance.choice_ids.is_empty());
        assert!(appearance.geosets.is_empty());
    }

    #[test]
    fn missing_schema_returns_error_with_display_context() {
        let connection = Connection::open_in_memory().unwrap();
        let error = query_authored_npc_appearance(&connection, 19177).unwrap_err();
        assert!(error.contains("19177"), "{error}");
        for table in ["choices", "geosets"] {
            let connection = fixture();
            connection
                .execute_batch(&format!("DROP TABLE {table};"))
                .unwrap();
            let error = query_authored_npc_appearance(&connection, 19177).unwrap_err();
            assert!(error.contains("19177"), "{error}");
        }
    }

    #[test]
    fn invalid_numeric_values_return_errors_without_partial_appearances() {
        for update in [
            "UPDATE appearances SET race = -1 WHERE display_id = 19177",
            "UPDATE appearances SET sex = 256 WHERE display_id = 19177",
            "UPDATE appearances SET class = 256 WHERE display_id = 19177",
            "UPDATE appearances SET baked_texture_fdid = -1 WHERE display_id = 19177",
            "UPDATE appearances SET baked_texture_fdid = 4294967296 WHERE display_id = 19177",
            "UPDATE choices SET choice_id = -1 WHERE display_id = 19177",
            "UPDATE choices SET choice_id = 4294967296 WHERE display_id = 19177",
            "UPDATE geosets SET geoset_index = 65536 WHERE display_id = 19177",
            "UPDATE geosets SET geoset_value = -1 WHERE display_id = 19177",
            "UPDATE appearances SET race = 'invalid' WHERE display_id = 19177",
        ] {
            let connection = fixture();
            connection.execute_batch(update).unwrap();
            let error = query_authored_npc_appearance(&connection, 19177).unwrap_err();
            assert!(error.contains("19177"), "{update}: {error}");
        }
    }
}
