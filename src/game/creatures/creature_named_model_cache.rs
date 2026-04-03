use std::collections::HashMap;
use std::path::Path;

use game_engine::paths;
use rusqlite::{Connection, OpenFlags};

pub(crate) fn load_named_model_fdid_cache(
    cache_path: &Path,
) -> Result<HashMap<String, u32>, String> {
    let conn = open_cache(cache_path, true)?;
    let mut stmt = match conn.prepare("SELECT name, fdid FROM named_model_fdids") {
        Ok(stmt) => stmt,
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Ok(HashMap::new());
        }
        Err(err) => return Err(format!("prepare named_model_fdids query: {err}")),
    };
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|err| format!("query named_model_fdids: {err}"))?;
    let mut cache = HashMap::new();
    for row in rows {
        let (name, fdid) = row.map_err(|err| format!("read named_model_fdids row: {err}"))?;
        cache.insert(name, fdid);
    }
    Ok(cache)
}

pub(crate) fn load_named_model_skin_cache(
    cache_path: &Path,
) -> Result<HashMap<String, [u32; 3]>, String> {
    let conn = open_cache(cache_path, true)?;
    let mut stmt = match conn
        .prepare("SELECT name, skin_fdid_0, skin_fdid_1, skin_fdid_2 FROM named_model_skins")
    {
        Ok(stmt) => stmt,
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Ok(HashMap::new());
        }
        Err(err) => return Err(format!("prepare named_model_skins query: {err}")),
    };
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                [
                    row.get::<_, u32>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, u32>(3)?,
                ],
            ))
        })
        .map_err(|err| format!("query named_model_skins: {err}"))?;
    let mut cache = HashMap::new();
    for row in rows {
        let (name, skin_fdids) = row.map_err(|err| format!("read named_model_skins row: {err}"))?;
        cache.insert(name, skin_fdids);
    }
    Ok(cache)
}

pub(crate) fn remember_named_model_fdid(
    cache_path: &Path,
    name: &str,
    fdid: u32,
) -> Result<(), String> {
    let conn = open_cache(cache_path, false)?;
    init_schema(&conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO named_model_fdids (name, fdid) VALUES (?1, ?2)",
        (name.to_ascii_lowercase(), fdid),
    )
    .map_err(|err| format!("insert named_model_fdids row {name}: {err}"))?;
    Ok(())
}

pub(crate) fn remember_named_model_skin(
    cache_path: &Path,
    name: &str,
    skin_fdids: [u32; 3],
) -> Result<(), String> {
    let conn = open_cache(cache_path, false)?;
    init_schema(&conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO named_model_skins (name, skin_fdid_0, skin_fdid_1, skin_fdid_2)
         VALUES (?1, ?2, ?3, ?4)",
        (
            name.to_ascii_lowercase(),
            skin_fdids[0],
            skin_fdids[1],
            skin_fdids[2],
        ),
    )
    .map_err(|err| format!("insert named_model_skins row {name}: {err}"))?;
    Ok(())
}

fn open_cache(cache_path: &Path, read_only: bool) -> Result<Connection, String> {
    let resolved = resolve_cache_path(cache_path);
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    if read_only {
        Connection::open_with_flags(
            &resolved,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|err| format!("open {}: {err}", resolved.display()))
    } else {
        Connection::open(&resolved).map_err(|err| format!("open {}: {err}", resolved.display()))
    }
}

fn resolve_cache_path(path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        paths::shared_data_path(path)
    }
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS named_model_fdids (
             name TEXT PRIMARY KEY,
             fdid INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS named_model_skins (
             name TEXT PRIMARY KEY,
             skin_fdid_0 INTEGER NOT NULL,
             skin_fdid_1 INTEGER NOT NULL,
             skin_fdid_2 INTEGER NOT NULL
         );",
    )
    .map_err(|err| format!("init named-model cache schema: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_model_cache_round_trips_sqlite_entries() {
        let cache_path = std::env::temp_dir().join(format!(
            "game-engine-named-model-cache-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        remember_named_model_fdid(&cache_path, "boar.m2", 1234).unwrap();
        remember_named_model_skin(&cache_path, "boar.m2", [11, 22, 33]).unwrap();

        let fdids = load_named_model_fdid_cache(&cache_path).unwrap();
        let skins = load_named_model_skin_cache(&cache_path).unwrap();

        assert_eq!(fdids.get("boar.m2").copied(), Some(1234));
        assert_eq!(skins.get("boar.m2").copied(), Some([11, 22, 33]));

        let _ = std::fs::remove_file(cache_path);
    }
}
