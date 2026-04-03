use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use game_engine::paths;
use rusqlite::{Connection, OpenFlags};

use crate::sky_lightdata::{LightDataRow, decode_bgr32};

const LIGHT_DATA_CACHE_PATH: &str = "cache/light_data_fallback.sqlite";

pub(crate) fn load_light_data_csv_fallback(
    path: &Path,
    param_id: u32,
) -> Result<Vec<LightDataRow>, String> {
    let cache_path = ensure_light_data_cache(path)?;
    load_rows_from_sqlite(&cache_path, param_id)
}

pub(crate) fn load_light_data_csv_fallback_uncached(
    path: &Path,
    param_id: u32,
) -> Result<Vec<LightDataRow>, String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let columns = super::resolve_csv_fallback_column_indices(header.trim_end_matches(['\r', '\n']));
    let mut rows = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        if let Some(row) = super::parse_csv_fallback_light_row(
            line.trim_end_matches(['\r', '\n']),
            &columns,
            param_id,
        ) {
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(rows)
}

fn ensure_light_data_cache(source_path: &Path) -> Result<PathBuf, String> {
    let cache_path = paths::shared_data_path(LIGHT_DATA_CACHE_PATH);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }

    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    if !cache_is_fresh(&conn, source_path)? {
        rebuild_cache(&conn, source_path)?;
    }
    Ok(cache_path)
}

fn load_rows_from_sqlite(cache_path: &Path, param_id: u32) -> Result<Vec<LightDataRow>, String> {
    let conn = Connection::open_with_flags(
        cache_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    let mut stmt = conn
        .prepare(
            "SELECT time, direct_color, ambient_color, sky_top, sky_middle,
                    sky_band1, sky_band2, sky_smog, fog_color
             FROM light_data_rows
             WHERE param_id = ?1
             ORDER BY time",
        )
        .map_err(|err| format!("prepare light_data_rows query: {err}"))?;
    let rows = stmt
        .query_map([param_id], |row| {
            Ok(LightDataRow {
                time: row.get(0)?,
                direct_color: decode_bgr32(row.get(1)?),
                ambient_color: decode_bgr32(row.get(2)?),
                sky_top: decode_bgr32(row.get(3)?),
                sky_middle: decode_bgr32(row.get(4)?),
                sky_band1: decode_bgr32(row.get(5)?),
                sky_band2: decode_bgr32(row.get(6)?),
                sky_smog: decode_bgr32(row.get(7)?),
                fog_color: decode_bgr32(row.get(8)?),
            })
        })
        .map_err(|err| format!("query light_data_rows: {err}"))?;

    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|err| format!("read light_data_rows row: {err}"))?);
    }
    Ok(values)
}

fn cache_is_fresh(conn: &Connection, source_path: &Path) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source_path, source_mtime FROM metadata LIMIT 1") {
        Ok(stmt) => stmt,
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare metadata query: {err}")),
    };
    let row = stmt.query_row([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    });
    let (recorded_path, recorded_mtime) = match row {
        Ok(row) => row,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(err) => return Err(format!("query metadata: {err}")),
    };
    Ok(recorded_path == source_path.to_string_lossy() && recorded_mtime == csv_mtime(source_path)?)
}

fn rebuild_cache(conn: &Connection, source_path: &Path) -> Result<(), String> {
    init_cache_schema(conn)?;
    import_rows(conn, source_path)?;
    record_metadata(conn, source_path)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit light data cache: {err}"))?;
    Ok(())
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS metadata;
         DROP TABLE IF EXISTS light_data_rows;
         CREATE TABLE metadata (
             source_path TEXT NOT NULL,
             source_mtime INTEGER NOT NULL
         );
         CREATE TABLE light_data_rows (
             param_id INTEGER NOT NULL,
             time REAL NOT NULL,
             direct_color INTEGER NOT NULL,
             ambient_color INTEGER NOT NULL,
             sky_top INTEGER NOT NULL,
             sky_middle INTEGER NOT NULL,
             sky_band1 INTEGER NOT NULL,
             sky_band2 INTEGER NOT NULL,
             sky_smog INTEGER NOT NULL,
             fog_color INTEGER NOT NULL,
             PRIMARY KEY (param_id, time)
         );",
    )
    .map_err(|err| format!("init light data cache: {err}"))
}

fn import_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let mut reader = open_reader(source_path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", source_path.display()))?;
    let columns = super::resolve_csv_fallback_column_indices(header.trim_end_matches(['\r', '\n']));
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO light_data_rows
             (param_id, time, direct_color, ambient_color, sky_top, sky_middle,
              sky_band1, sky_band2, sky_smog, fog_color)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .map_err(|err| format!("prepare light_data_rows insert: {err}"))?;
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", source_path.display()))?
            == 0
        {
            break;
        }
        insert_row(
            &mut insert,
            line.trim_end_matches(['\r', '\n']),
            &columns,
            source_path,
        )?;
    }
    Ok(())
}

fn insert_row(
    insert: &mut rusqlite::Statement<'_>,
    line: &str,
    columns: &[usize; 10],
    source_path: &Path,
) -> Result<(), String> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() <= columns.iter().copied().max().unwrap_or(0) {
        return Ok(());
    }
    let p = |i: usize| -> u32 {
        fields
            .get(columns[i])
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    let param_id = fields[columns[0]].parse::<u32>().map_err(|err| {
        format!(
            "parse {} param id row `{line}`: {err}",
            source_path.display()
        )
    })?;
    let time = fields[columns[1]]
        .parse::<f32>()
        .map_err(|err| format!("parse {} time row `{line}`: {err}", source_path.display()))?;
    insert
        .execute((
            param_id,
            time,
            p(2),
            p(3),
            p(4),
            p(5),
            p(6),
            p(7),
            p(8),
            p(9),
        ))
        .map_err(|err| format!("insert light_data_rows row {param_id}:{time}: {err}"))?;
    Ok(())
}

fn record_metadata(conn: &Connection, source_path: &Path) -> Result<(), String> {
    conn.execute(
        "INSERT INTO metadata (source_path, source_mtime) VALUES (?1, ?2)",
        (
            source_path.to_string_lossy().to_string(),
            csv_mtime(source_path)?,
        ),
    )
    .map_err(|err| format!("insert light data metadata: {err}"))?;
    Ok(())
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn csv_mtime(path: &Path) -> Result<i64, String> {
    let modified = std::fs::metadata(path)
        .map_err(|err| format!("stat {}: {err}", path.display()))?
        .modified()
        .map_err(|err| format!("mtime {}: {err}", path.display()))?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(label: &str) -> PathBuf {
        let unique = format!(
            "game-engine-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn load_light_data_csv_fallback_round_trips_cache() {
        let dir = temp_test_dir("light-data-cache");
        let csv_path = dir.join("LightData.csv");
        std::fs::write(
            &csv_path,
            "ID,LightParamID,Time,DirectColor,AmbientColor,SkyTopColor,SkyMiddleColor,SkyBand1Color,SkyBand2Color,SkySmogColor,SkyFogColor\n1,77,100,255,65280,16711680,255,255,255,255,255\n2,77,200,1,2,3,4,5,6,7,8\n3,88,300,9,10,11,12,13,14,15,16\n",
        )
        .unwrap();

        let rows = load_light_data_csv_fallback(&csv_path, 77).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, 100.0);
        assert_eq!(rows[1].time, 200.0);
        let direct = rows[0].direct_color.to_linear();
        assert!((direct.red - 1.0).abs() < 0.01);

        let _ = std::fs::remove_dir_all(dir);
    }
}
