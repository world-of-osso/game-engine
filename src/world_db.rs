use std::collections::HashMap;
use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};

fn world_db_path() -> PathBuf {
    if let Some(path) = std::env::var_os("GAME_SERVER_WORLD_DB") {
        PathBuf::from(path)
    } else {
        crate::paths::shared_repo_root()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("game-server")
            .join("data")
            .join("world.db")
    }
}

pub(crate) fn load_chr_race_prefixes() -> Result<HashMap<u8, String>, String> {
    let db_path = world_db_path();
    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", db_path.display()))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, client_prefix
             FROM chr_races
             WHERE id > 0
               AND client_prefix IS NOT NULL
               AND client_prefix != ''",
        )
        .map_err(|err| format!("prepare chr_races query: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            let id: u32 = row.get(0)?;
            let prefix: String = row.get(1)?;
            Ok((id as u8, prefix.trim().to_ascii_lowercase()))
        })
        .map_err(|err| format!("query chr_races: {err}"))?;

    let mut prefixes = HashMap::new();
    for row in rows {
        let (id, prefix) = row.map_err(|err| format!("read chr_races row: {err}"))?;
        if !prefix.is_empty() {
            prefixes.insert(id, prefix);
        }
    }

    if prefixes.is_empty() {
        return Err(format!(
            "chr_races in {} returned no client_prefix rows",
            db_path.display()
        ));
    }

    Ok(prefixes)
}

#[cfg(test)]
mod tests {
    use super::load_chr_race_prefixes;

    #[test]
    fn chr_race_prefixes_load_from_world_db() {
        let prefixes = load_chr_race_prefixes().expect("load chr_races prefixes from world.db");
        assert_eq!(prefixes.get(&1).map(String::as_str), Some("hu"));
    }
}
