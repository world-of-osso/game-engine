use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::asset_product::AssetProduct;
use crate::cache_source_mtime::{csv_mtime, source_key};
use crate::cache_sqlite::rebuild_unless_fresh;
use crate::creature_display_data::CreatureDisplay;
use crate::sqlite_util::is_missing_table_error;
use rusqlite::Connection;

#[derive(Clone, Copy)]
struct CreatureModelData {
    source_product: AssetProduct,
    fdid: u32,
    scale_milli: u32,
    uses_display_geosets: bool,
}

struct ModelDataColumns {
    id: usize,
    fdid: usize,
    scale: Option<usize>,
    geosets: usize,
}

struct DisplayInfoColumns {
    id: usize,
    model: usize,
    scale: usize,
    pet_instance_scale: usize,
    tex_var: [usize; 4],
}

/// Builds `<data_dir>/cache/creature_display.sqlite` (display rows and per-model preferred
/// skins) from the pinned Retail DB2 export plus the Forever overlay, reusing it
/// when its recorded sources are unchanged.
pub fn import_creature_display_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let retail = data_dir.join(crate::player_model_data::RETAIL_DB2_DIR);
    let di = retail.join("CreatureDisplayInfo.csv");
    let md = retail.join("CreatureModelData.csv");
    if !di.exists() || !md.exists() {
        return Err(format!(
            "Creature display CSVs not found: {} / {}",
            di.display(),
            md.display()
        ));
    }

    let cache_path = data_dir.join("cache/creature_display.sqlite");

    let forever = data_dir.join(crate::player_model_data::FOREVER_DB2_DIR);
    let mut source_paths = vec![di.clone(), md.clone()];
    if forever.exists() {
        source_paths.extend([
            forever.join("CreatureDisplayInfo.csv"),
            forever.join("CreatureModelData.csv"),
        ]);
    }
    rebuild_unless_fresh(
        &cache_path,
        |conn| cache_is_fresh(conn, data_dir, &source_paths),
        |conn| {
            rebuild_cache(conn, &di, &md)?;
            import_forever_display_rows(conn, &forever, &di)?;
            build_preferred_skins(conn)?;
            record_source_files(conn, data_dir, &source_paths)
        },
    )?;
    Ok(cache_path)
}

fn import_forever_display_rows(
    conn: &Connection,
    forever: &Path,
    retail_display_info: &Path,
) -> Result<(), String> {
    if !forever.exists() {
        return Ok(());
    }
    let models = parse_model_data(
        &forever.join("CreatureModelData.csv"),
        AssetProduct::Forever,
    )?;
    let mut retail_ids = HashSet::new();
    crate::csv_util::read_numeric_rows(retail_display_info, ["ID"], |[id]| {
        retail_ids.insert(id as u32);
    })?;
    import_display_rows(
        conn,
        &forever.join("CreatureDisplayInfo.csv"),
        &models,
        &retail_ids,
    )
}

fn cache_is_fresh(
    conn: &Connection,
    data_dir: &Path,
    source_paths: &[PathBuf],
) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT path, mtime FROM source_files") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files query: {err}")),
    };
    // A cache with the old three-slot schema must be rebuilt even at unchanged mtimes.
    if conn
        .prepare("SELECT d.skin_fdid_3, d.source_product, d.pet_instance_scale_milli, d.uses_display_geosets, p.skin_fdid_3 FROM creature_displays d, preferred_skins p LIMIT 0")
        .is_err()
    {
        return Ok(false);
    }
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("query source_files: {err}"))?;
    let mut recorded = HashMap::new();
    for row in rows {
        let (path, mtime) = row.map_err(|err| format!("read source_files row: {err}"))?;
        recorded.insert(path, mtime);
    }
    if recorded.len() != source_paths.len() {
        return Ok(false);
    }
    for path in source_paths {
        let key = source_key(data_dir, path)?;
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rebuild_cache(
    conn: &Connection,
    display_info_path: &Path,
    model_data_path: &Path,
) -> Result<(), String> {
    init_cache_schema(conn)?;
    let model_data = parse_model_data(model_data_path, AssetProduct::Retail)?;
    import_display_rows(conn, display_info_path, &model_data, &HashSet::new())?;
    Ok(())
}

fn build_preferred_skins(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "INSERT INTO preferred_skins (model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3)
         SELECT model_fdid,
                skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3
         FROM (
             SELECT model_fdid,
                    skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3,
                    display_id,
                    ROW_NUMBER() OVER (
                        PARTITION BY model_fdid
                        ORDER BY
                            (CASE WHEN skin_fdid_0 != 0 THEN 1 ELSE 0 END
                           + CASE WHEN skin_fdid_1 != 0 THEN 1 ELSE 0 END
                           + CASE WHEN skin_fdid_2 != 0 THEN 1 ELSE 0 END
                           + CASE WHEN skin_fdid_3 != 0 THEN 1 ELSE 0 END) DESC,
                            display_id ASC
                    ) AS rn
             FROM creature_displays
             WHERE skin_fdid_0 != 0 OR skin_fdid_1 != 0 OR skin_fdid_2 != 0 OR skin_fdid_3 != 0
         )
         WHERE rn = 1;
         COMMIT;",
    )
    .map_err(|err| format!("build preferred_skins: {err}"))
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS creature_displays;
         DROP TABLE IF EXISTS preferred_skins;
         CREATE TABLE source_files (
             path TEXT PRIMARY KEY,
             mtime INTEGER NOT NULL
         );
         CREATE TABLE creature_displays (
             display_id INTEGER PRIMARY KEY,
             model_fdid INTEGER NOT NULL,
             skin_fdid_0 INTEGER NOT NULL,
             skin_fdid_1 INTEGER NOT NULL,
             skin_fdid_2 INTEGER NOT NULL,
             skin_fdid_3 INTEGER NOT NULL,
             scale_milli INTEGER NOT NULL,
             pet_instance_scale_milli INTEGER NOT NULL,
             uses_display_geosets INTEGER NOT NULL,
             source_product TEXT NOT NULL CHECK(source_product IN ('wow', 'wow_classic_beta'))
         );
         CREATE INDEX idx_creature_displays_model_fdid
             ON creature_displays(model_fdid);
         CREATE TABLE preferred_skins (
             model_fdid INTEGER PRIMARY KEY,
             skin_fdid_0 INTEGER NOT NULL,
             skin_fdid_1 INTEGER NOT NULL,
             skin_fdid_2 INTEGER NOT NULL,
             skin_fdid_3 INTEGER NOT NULL
         );",
    )
    .map_err(|err| format!("init creature display cache: {err}"))
}

fn record_source_files(
    conn: &Connection,
    data_dir: &Path,
    source_paths: &[PathBuf],
) -> Result<(), String> {
    let mut insert = conn
        .prepare("INSERT OR REPLACE INTO source_files (path, mtime) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare source_files insert: {err}"))?;
    for path in source_paths {
        insert
            .execute((source_key(data_dir, path)?, csv_mtime(path)?))
            .map_err(|err| format!("insert source file {}: {err}", path.display()))?;
    }
    Ok(())
}

fn import_display_rows(
    conn: &Connection,
    display_info_path: &Path,
    model_data: &HashMap<u32, CreatureModelData>,
    excluded_ids: &HashSet<u32>,
) -> Result<(), String> {
    let mut reader = open_reader(display_info_path)?;
    let cols = read_display_columns(&mut reader, display_info_path)?;
    let mut insert = prepare_display_insert(conn)?;
    let mut line = String::new();
    let mut line_number = 1;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", display_info_path.display()))?
            == 0
        {
            break;
        }
        line_number += 1;
        insert_display_row(
            &mut insert,
            line.trim_end_matches(['\r', '\n']),
            &cols,
            model_data,
            excluded_ids,
        )
        .map_err(|error| format!("{}:{line_number}: {error}", display_info_path.display()))?;
    }
    Ok(())
}

fn read_display_columns(
    reader: &mut BufReader<std::fs::File>,
    display_info_path: &Path,
) -> Result<DisplayInfoColumns, String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", display_info_path.display()))?;
    find_display_info_columns(header.trim_end_matches(['\r', '\n']))
        .ok_or_else(|| format!("{} missing required columns", display_info_path.display()))
}

fn prepare_display_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR IGNORE INTO creature_displays
         (display_id, model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3, scale_milli, source_product, pet_instance_scale_milli, uses_display_geosets)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .map_err(|err| format!("prepare creature_displays insert: {err}"))
}

fn insert_display_row(
    insert: &mut rusqlite::Statement<'_>,
    line: &str,
    cols: &DisplayInfoColumns,
    model_data: &HashMap<u32, CreatureModelData>,
    excluded_ids: &HashSet<u32>,
) -> Result<(), String> {
    let Some((display_id, entry)) = parse_display_entry(line, cols, model_data)? else {
        return Ok(());
    };
    if excluded_ids.contains(&display_id) {
        return Ok(());
    }
    insert
        .execute((
            display_id,
            entry.model_fdid,
            entry.skin_fdids[0],
            entry.skin_fdids[1],
            entry.skin_fdids[2],
            entry.skin_fdids[3],
            entry.scale_milli,
            entry.source_product.as_str(),
            entry.pet_instance_scale_milli,
            entry.uses_display_geosets,
        ))
        .map_err(|err| format!("insert creature display row {display_id}: {err}"))?;
    Ok(())
}

fn parse_model_data(
    path: &Path,
    source_product: AssetProduct,
) -> Result<HashMap<u32, CreatureModelData>, String> {
    let mut map = HashMap::new();
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = crate::csv_util::parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let columns = ModelDataColumns {
        id: header_index(&headers, "ID", path)?,
        fdid: header_index(&headers, "FileDataID", path)?,
        scale: Some(header_index(&headers, "ModelScale", path)?),
        geosets: header_index(&headers, "CreatureGeosetDataID", path)?,
    };

    let mut line = String::new();
    let mut line_number = 1;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        line_number += 1;
        insert_model_entry(
            &mut map,
            line.trim_end_matches(['\r', '\n']),
            &columns,
            source_product,
        )
        .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
    }
    Ok(map)
}

fn insert_model_entry(
    map: &mut HashMap<u32, CreatureModelData>,
    line: &str,
    columns: &ModelDataColumns,
    source_product: AssetProduct,
) -> Result<(), String> {
    let cols = crate::csv_util::parse_csv_line(line);
    let id = crate::csv_util::parse_field::<u32>(&cols, columns.id)?;
    let fdid = crate::csv_util::parse_field::<u32>(&cols, columns.fdid)?;
    let geoset_data_id = crate::csv_util::parse_field::<u32>(&cols, columns.geosets)?;
    let scale = columns
        .scale
        .and_then(|index| cols.get(index))
        .map(String::as_str);
    let scale_milli = parse_scale_milli(scale)?;
    if fdid == 0 {
        return Ok(());
    }

    map.insert(
        id,
        CreatureModelData {
            source_product,
            fdid,
            scale_milli,
            uses_display_geosets: geoset_data_id != 0,
        },
    );
    Ok(())
}

fn find_display_info_columns(header: &str) -> Option<DisplayInfoColumns> {
    let headers = crate::csv_util::parse_csv_line(header);
    let find = |name: &str| headers.iter().position(|h| h.trim() == name);
    Some(DisplayInfoColumns {
        id: find("ID")?,
        model: find("ModelID")?,
        scale: find("CreatureModelScale")?,
        pet_instance_scale: find("PetInstanceScale")?,
        tex_var: [
            find("TextureVariationFileDataID_0")?,
            find("TextureVariationFileDataID_1")?,
            find("TextureVariationFileDataID_2")?,
            find("TextureVariationFileDataID_3")?,
        ],
    })
}

fn parse_display_entry(
    line: &str,
    cols: &DisplayInfoColumns,
    model_data: &HashMap<u32, CreatureModelData>,
) -> Result<Option<(u32, CreatureDisplay)>, String> {
    let values = crate::csv_util::parse_csv_line(line);
    let display_id = crate::csv_util::parse_field::<u32>(&values, cols.id)?;
    let model_id = crate::csv_util::parse_field::<u32>(&values, cols.model)?;
    let skin_fdids = [
        crate::csv_util::parse_field(&values, cols.tex_var[0])?,
        crate::csv_util::parse_field(&values, cols.tex_var[1])?,
        crate::csv_util::parse_field(&values, cols.tex_var[2])?,
        crate::csv_util::parse_field(&values, cols.tex_var[3])?,
    ];
    let display_scale_milli = parse_scale_milli(values.get(cols.scale).map(String::as_str))?;
    let pet_instance_scale_milli =
        parse_scale_milli(values.get(cols.pet_instance_scale).map(String::as_str))?;
    // Models without a FileDataID have no visual; malformed cells are not this case.
    let Some(model) = model_data.get(&model_id) else {
        return Ok(None);
    };
    Ok(Some((
        display_id,
        CreatureDisplay {
            source_product: model.source_product,
            model_fdid: model.fdid,
            uses_display_geosets: model.uses_display_geosets,
            skin_fdids,
            scale_milli: combine_scale_milli(display_scale_milli, model.scale_milli),
            pet_instance_scale_milli,
        },
    )))
}

fn parse_scale_milli(value: Option<&str>) -> Result<u32, String> {
    let value = value.ok_or("missing model scale")?;
    let scale: f32 = value
        .parse()
        .map_err(|error| format!("invalid model scale {value:?}: {error}"))?;
    if !scale.is_finite() {
        return Err(format!("non-finite model scale {value:?}"));
    }
    // Preserve the existing authored zero-scale normalization, not parse-error recovery.
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Ok((scale * 1000.0).round() as u32)
}

fn combine_scale_milli(display_scale_milli: u32, model_scale_milli: u32) -> u32 {
    let display = display_scale_milli.max(1);
    let model = model_scale_milli.max(1);
    ((display as u64 * model as u64 + 500) / 1000) as u32
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn header_index(headers: &[String], column: &str, path: &Path) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header.trim() == column)
        .ok_or_else(|| format!("{} missing {column} column", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache_sqlite::open_read_only;

    #[test]
    fn import_ordinary_geoset_gate_and_rebuild_older_catalog() {
        let dir = std::env::temp_dir().join(format!("creature-geoset-gate-{}", std::process::id()));
        let csv = dir.join(crate::player_model_data::RETAIL_DB2_DIR);
        std::fs::create_dir_all(&csv).unwrap();
        std::fs::write(csv.join("CreatureDisplayInfo.csv"),
            "ID,ModelID,CreatureModelScale,PetInstanceScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3\n1105,10253,1,1,0,0,0,0\n4213,7369,1.2,1,0,0,0,0\n").unwrap();
        std::fs::write(
            csv.join("CreatureModelData.csv"),
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n10253,1661349,1,343\n7369,974343,1,0\n",
        )
        .unwrap();
        let path = import_creature_display_cache(&dir).unwrap();
        let conn = Connection::open(&path).unwrap();
        let buzzard = crate::creature_display_data::query_display(&conn, 1105)
            .unwrap()
            .unwrap();
        assert!(buzzard.uses_display_geosets);
        let cobrahn = crate::creature_display_data::query_display(&conn, 4213)
            .unwrap()
            .unwrap();
        assert!(!cobrahn.uses_display_geosets);
        assert_eq!(cobrahn.scale_milli, 1200);
        conn.execute(
            "ALTER TABLE creature_displays DROP COLUMN uses_display_geosets",
            [],
        )
        .unwrap();
        drop(conn);
        import_creature_display_cache(&dir).unwrap();
        let conn = open_read_only(&path).unwrap();
        assert!(
            crate::creature_display_data::query_display(&conn, 1105)
                .unwrap()
                .unwrap()
                .uses_display_geosets
        );
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_pet_instance_scale_preserves_world_scale_and_rebuilds_old_cache() {
        let dir = std::env::temp_dir().join(format!("creature-pet-scale-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(crate::player_model_data::RETAIL_DB2_DIR)).unwrap();
        std::fs::write(dir.join(crate::player_model_data::RETAIL_DB2_DIR).join("CreatureDisplayInfo.csv"),
            "ID,ModelID,CreatureModelScale,PetInstanceScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3\n62217,7,0.25,0.7,11,12,0,0\n328,7,1,1,11,12,0,0\n").unwrap();
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR)
                .join("CreatureModelData.csv"),
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n7,1100485,2,0\n",
        )
        .unwrap();
        let path = import_creature_display_cache(&dir).unwrap();
        let conn = Connection::open(&path).unwrap();
        let scales: (u32, u32) = conn.query_row(
            "SELECT scale_milli, pet_instance_scale_milli FROM creature_displays WHERE display_id = 62217",
            [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(scales, (500, 700));
        conn.execute(
            "ALTER TABLE creature_displays DROP COLUMN pet_instance_scale_milli",
            [],
        )
        .unwrap();
        drop(conn);
        import_creature_display_cache(&dir).unwrap();
        let conn = open_read_only(&path).unwrap();
        let pet_scale: u32 = conn
            .query_row(
                "SELECT pet_instance_scale_milli FROM creature_displays WHERE display_id = 328",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(pet_scale, 1000);
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_lord_cannon_preserves_fourth_creature_variation() {
        let dir =
            std::env::temp_dir().join(format!("creature-fourth-variation-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(crate::player_model_data::RETAIL_DB2_DIR)).unwrap();
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR).join("CreatureDisplayInfo.csv"),
            "ID,ModelID,CreatureModelScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3,PetInstanceScale\n35297,2780,3,4237053,4237050,4237060,4237057,1\n",
        ).unwrap();
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR)
                .join("CreatureModelData.csv"),
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n2780,4236958,1,0\n",
        )
        .unwrap();
        let path = import_creature_display_cache(&dir).unwrap();
        let conn = open_read_only(&path).unwrap();
        let display = crate::creature_display_data::query_display(&conn, 35297)
            .unwrap()
            .unwrap();
        assert_eq!(display.model_fdid, 4236958);
        assert_eq!(display.scale_milli, 3000);
        assert_eq!(
            display.skin_fdids.as_slice(),
            &[4237053, 4237050, 4237060, 4237057]
        );
        let preferred = crate::creature_display_data::query_preferred_skins(&conn, 4236958)
            .unwrap()
            .unwrap();
        assert_eq!(preferred.as_slice(), &[4237053, 4237050, 4237060, 4237057]);
        drop(conn);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "ALTER TABLE creature_displays DROP COLUMN skin_fdid_3;
             ALTER TABLE preferred_skins DROP COLUMN skin_fdid_3;",
        )
        .unwrap();
        drop(conn);
        // Source mtimes are unchanged: schema freshness alone must trigger re-import.
        let path = import_creature_display_cache(&dir).unwrap();
        let conn = open_read_only(&path).unwrap();
        let display = crate::creature_display_data::query_display(&conn, 35297)
            .unwrap()
            .unwrap();
        assert_eq!(
            display.skin_fdids.as_slice(),
            &[4237053, 4237050, 4237060, 4237057]
        );
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn equivalent_data_root_spelling_reuses_the_cache() {
        let dir =
            std::env::temp_dir().join(format!("creature-display-alias-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(crate::player_model_data::RETAIL_DB2_DIR)).unwrap();
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR).join("CreatureDisplayInfo.csv"),
            "ID,ModelID,CreatureModelScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3,PetInstanceScale\n4,7,1,11,12,0,0,1\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR)
                .join("CreatureModelData.csv"),
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n7,9001,1,0\n",
        )
        .unwrap();
        let cache = import_creature_display_cache(&dir).unwrap();
        let conn = Connection::open(&cache).unwrap();
        conn.execute("DELETE FROM preferred_skins", []).unwrap();
        drop(conn);
        let alias = dir.join("..").join(dir.file_name().unwrap());
        import_creature_display_cache(&alias).unwrap();
        let conn = open_read_only(&cache).unwrap();
        assert_eq!(
            crate::creature_display_data::query_preferred_skins(&conn, 9001).unwrap(),
            None,
            "an equivalent data root must not rebuild the cache"
        );
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn forever_npc_displays_keep_retail_collisions_and_product_model_chain() {
        let dir = std::env::temp_dir().join(format!("forever-npc-displays-{}", std::process::id()));
        let forever = dir.join("db2/1.60.1.70205");
        std::fs::create_dir_all(&forever).unwrap();
        std::fs::create_dir_all(dir.join(crate::player_model_data::RETAIL_DB2_DIR)).unwrap();
        let header = "ID,ModelID,CreatureModelScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3,PetInstanceScale\n";
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR)
                .join("CreatureDisplayInfo.csv"),
            format!("{header}10,1,1,11,0,0,0,1\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join(crate::player_model_data::RETAIL_DB2_DIR)
                .join("CreatureModelData.csv"),
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n1,1011653,1.25,0\n",
        )
        .unwrap();
        std::fs::write(
            forever.join("CreatureDisplayInfo.csv"),
            format!("{header}10,1,2,999,0,0,0,1\n136968,1,1,0,0,0,0,1\n139694,1,1,0,0,0,0,1\n"),
        )
        .unwrap();
        std::fs::write(
            forever.join("CreatureModelData.csv"),
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n1,7478494,1,0\n",
        )
        .unwrap();
        let cache = import_creature_display_cache(&dir).unwrap();
        let conn = open_read_only(&cache).unwrap();
        let retail = crate::creature_display_data::query_display(&conn, 10)
            .unwrap()
            .unwrap();
        assert_eq!(retail.model_fdid, 1011653);
        assert_eq!(retail.skin_fdids, [11, 0, 0, 0]);
        assert_eq!(retail.scale_milli, 1250);
        for (id, expected_product) in [
            (10, "wow"),
            (136968, "wow_classic_beta"),
            (139694, "wow_classic_beta"),
        ] {
            let product: String = conn
                .query_row(
                    "SELECT source_product FROM creature_displays WHERE display_id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                product, expected_product,
                "display {id} lost its authored metadata owner"
            );
            let display = crate::creature_display_data::query_display(&conn, id)
                .unwrap()
                .unwrap();
            assert_eq!(display.source_product.as_str(), expected_product);
        }
        for id in [136968, 139694] {
            assert_eq!(
                crate::creature_display_data::query_display(&conn, id)
                    .unwrap()
                    .unwrap()
                    .model_fdid,
                7478494
            );
        }
        drop(conn);
        // Removing an overlay invalidates the old cache rather than retaining its rows.
        std::fs::remove_dir_all(&forever).unwrap();
        let cache = import_creature_display_cache(&dir).unwrap();
        let conn = open_read_only(&cache).unwrap();
        assert_eq!(
            crate::creature_display_data::query_display(&conn, 136968).unwrap(),
            None
        );
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_and_query_creature_display() {
        let dir =
            std::env::temp_dir().join(format!("creature-display-cache-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(crate::player_model_data::RETAIL_DB2_DIR)).unwrap();
        let display_path = dir
            .join(crate::player_model_data::RETAIL_DB2_DIR)
            .join("CreatureDisplayInfo.csv");
        let model_path = dir
            .join(crate::player_model_data::RETAIL_DB2_DIR)
            .join("CreatureModelData.csv");
        std::fs::write(
            &display_path,
            "ID,ModelID,CreatureModelScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3,PetInstanceScale\n4,7,1,11,12,0,0,1\n5,7,1,21,22,23,24,1\n",
        )
        .unwrap();
        std::fs::write(
            &model_path,
            "ID,FileDataID,ModelScale,CreatureGeosetDataID\n7,9001,1.25,0\n",
        )
        .unwrap();

        let cache_path = dir.join("creature_display.sqlite");
        let conn = Connection::open(&cache_path).unwrap();
        rebuild_cache(&conn, &display_path, &model_path).unwrap();
        build_preferred_skins(&conn).unwrap();
        drop(conn);

        let conn = open_read_only(&cache_path).unwrap();

        let entry = crate::creature_display_data::query_display(&conn, 4)
            .unwrap()
            .unwrap();
        assert_eq!(
            entry,
            CreatureDisplay {
                source_product: AssetProduct::Retail,
                model_fdid: 9001,
                uses_display_geosets: false,
                skin_fdids: [11, 12, 0, 0],
                scale_milli: 1250,
                pet_instance_scale_milli: 1000,
            }
        );

        // Preferred skins: display_id=5 has 3 filled slots vs display_id=4 with 2
        let mut stmt = conn
            .prepare(
                "SELECT skin_fdid_0, skin_fdid_1, skin_fdid_2, skin_fdid_3
                 FROM preferred_skins WHERE model_fdid = ?1",
            )
            .unwrap();
        let skins: [u32; 4] = stmt
            .query_row([9001u32], |row| {
                Ok([row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?])
            })
            .unwrap();
        assert_eq!(skins, [21, 22, 23, 24]);

        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod db2_header_tests {
    use super::*;
    #[test]
    fn db2_header_creature_rejects_invalid_canonical_row() {
        let fixture = crate::db2_test_support::ExportFixture::copy("CreatureModelData", Some("ID"));
        let error = parse_model_data(&fixture.0, AssetProduct::Retail)
            .err()
            .expect("invalid model ID must fail");
        assert!(error.contains("CreatureModelData.csv:2:"), "{error}");
    }
    #[test]
    fn db2_header_creature_reads_canonical_models() {
        let fixture = crate::db2_test_support::ExportFixture::copy("CreatureModelData", None);
        let models = parse_model_data(&fixture.0, AssetProduct::Retail).unwrap();
        assert!(!models.is_empty());
    }
}

#[cfg(test)]
mod db2_canonical_tests {
    use super::*;
    #[test]
    fn db2_header_creatures_read_installed_model_display_chain() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let models =
            parse_model_data(&source.join("CreatureModelData.csv"), AssetProduct::Retail).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        init_cache_schema(&conn).unwrap();
        import_display_rows(
            &conn,
            &source.join("CreatureDisplayInfo.csv"),
            &models,
            &HashSet::new(),
        )
        .unwrap();
        let count: usize = conn
            .query_row("SELECT COUNT(*) FROM creature_displays", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(count > 0);
        let expected: u32 = conn
            .query_row(
                "SELECT model_fdid FROM creature_displays WHERE display_id=328",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(expected, 1112735);
    }
}
