use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::outfit_data::DisplayInfoResolved;

pub(super) fn load_cached_display_info(
    data_dir: &Path,
    display_info_id: u32,
) -> Result<Option<DisplayInfoResolved>, String> {
    let conn = open_outfit_conn(data_dir)?;
    let Some(mut display) = query_display_info_row(&conn, display_info_id)? else {
        return Ok(None);
    };
    display.item_materials = query_display_materials(&conn, display_info_id)?;
    Ok(Some(display))
}

/// The material's texture files in TextureFileData order.
pub(super) fn load_cached_material_texture_fdids(
    data_dir: &Path,
    material_resource_id: u32,
) -> Result<Vec<u32>, String> {
    let conn = open_outfit_conn(data_dir)?;
    query_material_texture_fdids(&conn, material_resource_id)
}

fn query_material_texture_fdids(
    conn: &Connection,
    material_resource_id: u32,
) -> Result<Vec<u32>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT texture_fdid FROM material_textures
             WHERE material_resource_id = ?1 ORDER BY file_order",
        )
        .map_err(|err| format!("prepare material_textures lookup: {err}"))?;
    let rows = stmt
        .query_map([material_resource_id], |row| row.get::<_, u32>(0))
        .map_err(|err| format!("query material_textures rows: {err}"))?;
    rows.collect::<Result<_, _>>()
        .map_err(|err| format!("read material_textures row: {err}"))
}

pub(super) fn load_cached_model_fdids(
    data_dir: &Path,
    model_resource_id: u32,
) -> Result<Vec<u32>, String> {
    let conn = open_outfit_conn(data_dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT file_data_id
             FROM model_to_fdid
             WHERE model_resource_id = ?1
             ORDER BY model_order",
        )
        .map_err(|err| format!("prepare model_to_fdid single lookup: {err}"))?;
    let rows = stmt
        .query_map([model_resource_id], |row| row.get::<_, u32>(0))
        .map_err(|err| format!("query model_to_fdid single rows: {err}"))?;
    let mut fdids = Vec::new();
    for row in rows {
        fdids.push(row.map_err(|err| format!("read model_to_fdid single row: {err}"))?);
    }
    Ok(fdids)
}

pub(super) fn resolve_cached_skin_fdids_for_model_fdid(
    data_dir: &Path,
    model_fdid: u32,
) -> Result<Option<[u32; 3]>, String> {
    let conn = open_outfit_conn(data_dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT di.id, di.model_mat_res_0, di.model_mat_res_1
             FROM display_info di
             JOIN model_to_fdid m
               ON m.model_resource_id = di.model_res_0
               OR m.model_resource_id = di.model_res_1
             WHERE m.file_data_id = ?1
             ORDER BY di.id",
        )
        .map_err(|err| format!("prepare model skin lookup by fdid: {err}"))?;
    let rows = stmt
        .query_map([model_fdid], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                [row.get::<_, u32>(1)?, row.get::<_, u32>(2)?],
            ))
        })
        .map_err(|err| format!("query model skin lookup by fdid: {err}"))?;
    select_best_skin_fdids(&conn, rows)
}

pub(super) fn resolve_cached_skin_fdids_for_model_name(
    data_dir: &Path,
    model_name: &str,
) -> Result<Option<[u32; 3]>, String> {
    let conn = open_outfit_conn(data_dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT di.id, di.model_mat_res_0, di.model_mat_res_1, m.file_data_id
             FROM display_info di
             JOIN model_to_fdid m
               ON m.model_resource_id = di.model_res_0
               OR m.model_resource_id = di.model_res_1
             ORDER BY di.id, m.model_order",
        )
        .map_err(|err| format!("prepare model skin lookup by name: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                [row.get::<_, u32>(1)?, row.get::<_, u32>(2)?],
                row.get::<_, u32>(3)?,
            ))
        })
        .map_err(|err| format!("query model skin lookup by name: {err}"))?;
    let matching_fdids = crate::outfit_listfile::find_fdids_by_name(data_dir, model_name)?;
    select_best_skin_fdids_by_name(&conn, rows, &matching_fdids)
}

fn select_best_skin_fdids_by_name(
    conn: &Connection,
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<(u32, [u32; 2], u32)>,
    >,
    matching_fdids: &HashSet<u32>,
) -> Result<Option<[u32; 3]>, String> {
    let mut material_cache = HashMap::new();
    let mut best: Option<(usize, u32, [u32; 3])> = None;
    for row in rows {
        let (display_info_id, material_ids, model_fdid) =
            row.map_err(|err| format!("read model skin lookup by name row: {err}"))?;
        if !matching_fdids.contains(&model_fdid) {
            continue;
        }
        let skin_fdids = resolve_skin_fdids(conn, material_ids, &mut material_cache)?;
        update_best_skin_candidate(&mut best, display_info_id, skin_fdids);
    }
    Ok(best.map(|(_, _, skin_fdids)| skin_fdids))
}

fn open_outfit_conn(data_dir: &Path) -> Result<Connection, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    super::open_read_only(&cache_path)
}

fn query_display_info_row(
    conn: &Connection,
    display_info_id: u32,
) -> Result<Option<DisplayInfoResolved>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT model_res_0, model_res_1, model_mat_res_0, model_mat_res_1,
                    geoset_group_0, geoset_group_1, geoset_group_2, geoset_group_3,
                    geoset_group_4, geoset_group_5, helmet_vis_0, helmet_vis_1
             FROM display_info
             WHERE id = ?1",
        )
        .map_err(|err| format!("prepare display_info single lookup: {err}"))?;
    stmt.query_row([display_info_id], |row| {
        Ok(build_display_info_row(
            [row.get::<_, u32>(0)?, row.get::<_, u32>(1)?],
            [row.get::<_, u32>(2)?, row.get::<_, u32>(3)?],
            [
                row.get::<_, i16>(4)?,
                row.get::<_, i16>(5)?,
                row.get::<_, i16>(6)?,
                row.get::<_, i16>(7)?,
                row.get::<_, i16>(8)?,
                row.get::<_, i16>(9)?,
            ],
            [row.get::<_, u32>(10)?, row.get::<_, u32>(11)?],
        ))
    })
    .optional()
    .map_err(|err| format!("query display_info single row: {err}"))
}

fn query_display_materials(
    conn: &Connection,
    display_info_id: u32,
) -> Result<Vec<(u8, u32)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT component_section, material_resource_id
             FROM display_materials
             WHERE display_info_id = ?1
             ORDER BY component_section, material_resource_id",
        )
        .map_err(|err| format!("prepare display_materials single lookup: {err}"))?;
    let rows = stmt
        .query_map([display_info_id], |row| {
            Ok((row.get::<_, u8>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|err| format!("query display_materials single rows: {err}"))?;
    let mut materials = Vec::new();
    for row in rows {
        materials.push(row.map_err(|err| format!("read display_materials row: {err}"))?);
    }
    Ok(materials)
}

fn build_display_info_row(
    model_resources: [u32; 2],
    model_material_resources: [u32; 2],
    geoset_groups: [i16; 6],
    helmet_vis_ids: [u32; 2],
) -> DisplayInfoResolved {
    let collect = |values: [u32; 2]| values.into_iter().filter(|v| *v != 0).collect::<Vec<_>>();
    DisplayInfoResolved {
        item_materials: Vec::new(),
        geoset_overrides: Vec::new(),
        model_resource_ids: collect(model_resources),
        model_material_resource_ids: collect(model_material_resources),
        model_resource_columns: model_resources,
        model_material_resource_columns: model_material_resources,
        helmet_geoset_vis_ids: collect(helmet_vis_ids),
        geoset_groups,
    }
}

fn select_best_skin_fdids(
    conn: &Connection,
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<(u32, [u32; 2])>,
    >,
) -> Result<Option<[u32; 3]>, String> {
    let mut material_cache = HashMap::new();
    let mut best: Option<(usize, u32, [u32; 3])> = None;
    for row in rows {
        let (display_info_id, material_ids) =
            row.map_err(|err| format!("read model skin lookup row: {err}"))?;
        let skin_fdids = resolve_skin_fdids(conn, material_ids, &mut material_cache)?;
        update_best_skin_candidate(&mut best, display_info_id, skin_fdids);
    }
    Ok(best.map(|(_, _, skin_fdids)| skin_fdids))
}

fn update_best_skin_candidate(
    best: &mut Option<(usize, u32, [u32; 3])>,
    display_info_id: u32,
    skin_fdids: [u32; 3],
) {
    let filled = count_filled_skin_fdids(skin_fdids);
    if filled == 0 || !is_better_skin_candidate(best, filled, display_info_id) {
        return;
    }
    *best = Some((filled, display_info_id, skin_fdids));
}

fn count_filled_skin_fdids(skin_fdids: [u32; 3]) -> usize {
    skin_fdids.iter().filter(|&&fdid| fdid != 0).count()
}

fn is_better_skin_candidate(
    best: &Option<(usize, u32, [u32; 3])>,
    filled: usize,
    display_info_id: u32,
) -> bool {
    match best {
        None => true,
        Some((best_filled, best_display_id, _)) => {
            filled > *best_filled || (filled == *best_filled && display_info_id < *best_display_id)
        }
    }
}

fn resolve_skin_fdids(
    conn: &Connection,
    material_ids: [u32; 2],
    cache: &mut HashMap<u32, u32>,
) -> Result<[u32; 3], String> {
    let mut skin_fdids = [0; 3];
    for (idx, material_id) in material_ids.into_iter().enumerate() {
        if material_id == 0 {
            continue;
        }
        if let Some(&fdid) = cache.get(&material_id) {
            skin_fdids[idx] = fdid;
            continue;
        }
        // Model-path lookups have no wearer: the material's first file.
        let fdid = query_material_texture_fdids(conn, material_id)?
            .first()
            .copied()
            .unwrap_or(0);
        cache.insert(material_id, fdid);
        skin_fdids[idx] = fdid;
    }
    Ok(skin_fdids)
}
