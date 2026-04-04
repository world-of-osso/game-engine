use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::cache_metadata::single_source_cache_is_fresh;
use crate::cache_source_mtime::csv_mtime;
use game_engine::paths;
use rusqlite::{Connection, OpenFlags, params_from_iter};

use crate::sky_lightdata::{LightDataRow, decode_bgr32};

const LIGHT_DATA_CACHE_PATH: &str = "cache/light_data_fallback_v3.sqlite";

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
    if !single_source_cache_is_fresh(&conn, source_path)? {
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
                    sky_band1, sky_band2, sky_smog, fog_color,
                    sun_color, sun_halo_color, cloud_emissive_color,
                    cloud_layer1_ambient_color, cloud_layer2_ambient_color,
                    ocean_close_color, ocean_far_color,
                    river_close_color, river_far_color, horizon_ambient_color,
                    fog_end, fog_start, glow, cloud_density, unk1, unk2
             FROM light_data_rows
             WHERE param_id = ?1
             ORDER BY time",
        )
        .map_err(|err| format!("prepare light_data_rows query: {err}"))?;
    let rows = stmt
        .query_map([param_id], decode_light_data_row)
        .map_err(|err| format!("query light_data_rows: {err}"))?;

    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|err| format!("read light_data_rows row: {err}"))?);
    }
    Ok(values)
}

fn decode_light_data_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LightDataRow> {
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
        sun_color: decode_bgr32(row.get(9)?),
        sun_halo_color: decode_bgr32(row.get(10)?),
        cloud_emissive_color: decode_bgr32(row.get(11)?),
        cloud_layer1_ambient_color: decode_bgr32(row.get(12)?),
        cloud_layer2_ambient_color: decode_bgr32(row.get(13)?),
        ocean_close_color: decode_bgr32(row.get(14)?),
        ocean_far_color: decode_bgr32(row.get(15)?),
        river_close_color: decode_bgr32(row.get(16)?),
        river_far_color: decode_bgr32(row.get(17)?),
        horizon_ambient_color: decode_bgr32(row.get(18)?),
        fog_end: row.get(19)?,
        fog_start: row.get(20)?,
        glow: row.get(21)?,
        cloud_density: row.get(22)?,
        unk1: row.get(23)?,
        unk2: row.get(24)?,
    })
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
    conn.execute_batch(&build_cache_schema_sql())
        .map_err(|err| format!("init light data cache: {err}"))
}

fn build_cache_schema_sql() -> String {
    format!(
        "{}{}{}",
        begin_cache_schema_sql(),
        drop_cache_tables_sql(),
        create_cache_tables_sql()
    )
}

fn begin_cache_schema_sql() -> &'static str {
    "BEGIN;\n"
}

fn drop_cache_tables_sql() -> &'static str {
    "DROP TABLE IF EXISTS metadata;
     DROP TABLE IF EXISTS light_data_rows;
    "
}

fn create_cache_tables_sql() -> String {
    format!(
        "{}\n{}",
        create_metadata_table_sql(),
        create_light_data_rows_table_sql()
    )
}

const fn create_metadata_table_sql() -> &'static str {
    "CREATE TABLE metadata (
         source_path TEXT NOT NULL,
         source_mtime INTEGER NOT NULL
     );"
}

const fn create_light_data_rows_table_sql() -> &'static str {
    "CREATE TABLE light_data_rows (
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
         sun_color INTEGER NOT NULL,
         sun_halo_color INTEGER NOT NULL,
         cloud_emissive_color INTEGER NOT NULL,
         cloud_layer1_ambient_color INTEGER NOT NULL,
         cloud_layer2_ambient_color INTEGER NOT NULL,
         ocean_close_color INTEGER NOT NULL,
         ocean_far_color INTEGER NOT NULL,
         river_close_color INTEGER NOT NULL,
         river_far_color INTEGER NOT NULL,
         horizon_ambient_color INTEGER NOT NULL,
         fog_end REAL NOT NULL,
         fog_start REAL NOT NULL,
         glow REAL NOT NULL,
         cloud_density REAL NOT NULL,
         unk1 REAL NOT NULL,
         unk2 REAL NOT NULL,
         PRIMARY KEY (param_id, time)
     );"
}

fn import_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let mut reader = open_reader(source_path)?;
    let columns = read_import_columns(&mut reader, source_path)?;
    let mut insert = prepare_row_insert(conn)?;
    import_reader_rows(&mut reader, &mut insert, &columns, source_path)?;
    Ok(())
}

fn read_import_columns<R: BufRead>(
    reader: &mut R,
    source_path: &Path,
) -> Result<[usize; 26], String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", source_path.display()))?;
    Ok(super::resolve_csv_fallback_column_indices(
        header.trim_end_matches(['\r', '\n']),
    ))
}

fn prepare_row_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR REPLACE INTO light_data_rows
         (param_id, time, direct_color, ambient_color, sky_top, sky_middle,
          sky_band1, sky_band2, sky_smog, fog_color,
          sun_color, sun_halo_color, cloud_emissive_color,
          cloud_layer1_ambient_color, cloud_layer2_ambient_color,
          ocean_close_color, ocean_far_color,
          river_close_color, river_far_color, horizon_ambient_color,
          fog_end, fog_start, glow, cloud_density, unk1, unk2)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
    )
    .map_err(|err| format!("prepare light_data_rows insert: {err}"))
}

fn import_reader_rows<R: BufRead>(
    reader: &mut R,
    insert: &mut rusqlite::Statement<'_>,
    columns: &[usize; 26],
    source_path: &Path,
) -> Result<(), String> {
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
            insert,
            line.trim_end_matches(['\r', '\n']),
            columns,
            source_path,
        )?;
    }
    Ok(())
}

fn insert_row(
    insert: &mut rusqlite::Statement<'_>,
    line: &str,
    columns: &[usize; 26],
    source_path: &Path,
) -> Result<(), String> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() <= columns.iter().copied().max().unwrap_or(0) {
        return Ok(());
    }
    let (param_id, time) = parse_row_identity(&fields, columns, line, source_path)?;
    let values = build_insert_row_values(&fields, columns, param_id, time);
    insert
        .execute(params_from_iter(values))
        .map_err(|err| format!("insert light_data_rows row {param_id}:{time}: {err}"))?;
    Ok(())
}

fn parse_row_identity(
    fields: &[&str],
    columns: &[usize; 26],
    line: &str,
    source_path: &Path,
) -> Result<(u32, f32), String> {
    let param_id = fields[columns[0]].parse::<u32>().map_err(|err| {
        format!(
            "parse {} param id row `{line}`: {err}",
            source_path.display()
        )
    })?;
    let time = fields[columns[1]]
        .parse::<f32>()
        .map_err(|err| format!("parse {} time row `{line}`: {err}", source_path.display()))?;
    Ok((param_id, time))
}

fn build_insert_row_values<'a>(
    fields: &'a [&'a str],
    columns: &[usize; 26],
    param_id: u32,
    time: f32,
) -> Vec<rusqlite::types::Value> {
    let p = |i: usize| -> u32 {
        fields
            .get(columns[i])
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    let pf = |i: usize| -> f32 {
        fields
            .get(columns[i])
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0)
    };
    let mut values = build_insert_identity_and_color_values(&p, param_id, time);
    values.extend(build_insert_fog_and_aux_values(&pf));
    values
}

fn build_insert_identity_and_color_values(
    p: &impl Fn(usize) -> u32,
    param_id: u32,
    time: f32,
) -> Vec<rusqlite::types::Value> {
    vec![
        param_id.into(),
        time.into(),
        p(2).into(),
        p(3).into(),
        p(4).into(),
        p(5).into(),
        p(6).into(),
        p(7).into(),
        p(8).into(),
        p(9).into(),
        p(10).into(),
        p(11).into(),
        p(12).into(),
        p(13).into(),
        p(14).into(),
        p(15).into(),
        p(16).into(),
        p(17).into(),
        p(18).into(),
        p(19).into(),
    ]
}

fn build_insert_fog_and_aux_values(pf: &impl Fn(usize) -> f32) -> [rusqlite::types::Value; 6] {
    [
        pf(20).into(),
        (pf(20) * pf(21)).into(),
        pf(22).into(),
        pf(23).into(),
        pf(24).into(),
        pf(25).into(),
    ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_light_data_csv_fallback_round_trips_cache() {
        let dir = game_engine::test_harness::temp_test_dir("light-data-cache");
        let csv_path = dir.join("LightData.csv");
        std::fs::write(
            &csv_path,
            "ID,LightParamID,Time,DirectColor,AmbientColor,SkyTopColor,SkyMiddleColor,SkyBand1Color,SkyBand2Color,SkySmogColor,SkyFogColor,SunColor,CloudSunColor,CloudEmissiveColor,CloudLayer1AmbientColor,CloudLayer2AmbientColor,OceanCloseColor,OceanFarColor,RiverCloseColor,RiverFarColor,ShadowOpacity,FogEnd,FogScaler,FogDensity,FogHeight,FogHeightScaler,FogHeightDensity,FogZScalar,MainFogStartDist,MainFogEndDist,SunFogAngle,CloudDensity,ColorGradingFileDataID,DarkerColorGradingFileDataID,HorizonAmbientColor,GroundAmbientColor,EndFogColor,EndFogColorDistance,FogStartOffset,SunFogColor,SunFogStrength,FogHeightColor,EndFogHeightColor,Field_10_0_0_44649_042,Field_12_0_0_63854_043,FogHeightCoefficients_0,FogHeightCoefficients_1,FogHeightCoefficients_2,FogHeightCoefficients_3,MainFogCoefficients_0,MainFogCoefficients_1,MainFogCoefficients_2,MainFogCoefficients_3,HeightDensityFogCoeff_0,HeightDensityFogCoeff_1,HeightDensityFogCoeff_2,HeightDensityFogCoeff_3\n1,77,100,255,65280,16711680,255,255,255,255,255,111,222,333,444,555,666,777,888,999,0,1000,0.25,0,0,0,0,0,0,0,0,0.5,0,0,1234,0,0,0,0,0,1.5,0,0,2.5,3.5,0,0,0,0,0,0,0,0,0,0,0,0\n2,77,200,1,2,3,4,5,6,7,8,12,23,34,45,56,67,78,89,90,0,2000,0.5,0,0,0,0,0,0,0,0,0.75,0,0,2345,0,0,0,0,0,2.0,0,0,4.5,5.5,0,0,0,0,0,0,0,0,0,0,0,0\n3,88,300,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,0,3000,0.75,0,0,0,0,0,0,0,0,1.0,0,0,3456,0,0,0,0,0,2.5,0,0,6.5,7.5,0,0,0,0,0,0,0,0,0,0,0,0\n",
        )
        .unwrap();

        let rows = load_light_data_csv_fallback(&csv_path, 77).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, 100.0);
        assert_eq!(rows[1].time, 200.0);
        let direct = rows[0].direct_color.to_linear();
        assert!((direct.red - 1.0).abs() < 0.01);
        let sun = rows[0].sun_color.to_linear();
        assert!(sun.red > 0.0 || sun.green > 0.0 || sun.blue > 0.0);
        assert_eq!(rows[0].fog_end, 1000.0);
        assert_eq!(rows[0].fog_start, 250.0);
        assert_eq!(rows[0].glow, 1.5);

        let _ = std::fs::remove_dir_all(dir);
    }
}
