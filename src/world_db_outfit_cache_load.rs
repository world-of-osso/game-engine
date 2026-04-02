use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

use crate::outfit_data::{DisplayInfoResolved, DisplayMaterialTextures};

pub(super) fn load_cached_char_start_outfits(
    data_dir: &Path,
) -> Result<super::StarterOutfits, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT race_id, class_id, sex_id, item_id
             FROM starter_outfits
             ORDER BY race_id, class_id, sex_id, item_order",
        )
        .map_err(|err| format!("prepare starter_outfits lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u8>(0)?,
                row.get::<_, u8>(1)?,
                row.get::<_, u8>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })
        .map_err(|err| format!("query starter_outfits: {err}"))?;
    let mut outfits = HashMap::new();
    for row in rows {
        let (race, class, sex, item_id) =
            row.map_err(|err| format!("read starter_outfits row: {err}"))?;
        outfits
            .entry((race, class, sex))
            .or_insert_with(Vec::new)
            .push(item_id);
    }
    Ok(outfits)
}

pub(super) fn load_cached_item_modified_appearance(
    data_dir: &Path,
) -> Result<HashMap<u32, u32>, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare("SELECT item_id, appearance_id FROM item_modified_appearance_map")
        .map_err(|err| format!("prepare item_modified_appearance_map lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query item_modified_appearance_map: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (item_id, appearance_id) =
            row.map_err(|err| format!("read item_modified_appearance_map row: {err}"))?;
        map.insert(item_id, appearance_id);
    }
    Ok(map)
}

pub(super) fn load_cached_item_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare("SELECT appearance_id, display_info_id FROM item_appearance_map")
        .map_err(|err| format!("prepare item_appearance_map lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query item_appearance_map: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (appearance_id, display_info_id) =
            row.map_err(|err| format!("read item_appearance_map row: {err}"))?;
        map.insert(appearance_id, display_info_id);
    }
    Ok(map)
}

pub(super) fn load_material_to_texture_map(
    conn: &Connection,
) -> Result<HashMap<u32, u32>, String> {
    let mut stmt = conn
        .prepare("SELECT material_resource_id, texture_fdid FROM material_to_texture")
        .map_err(|err| format!("prepare material_to_texture lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query material_to_texture: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (material_resource_id, texture_fdid) =
            row.map_err(|err| format!("read material_to_texture row: {err}"))?;
        map.insert(material_resource_id, texture_fdid);
    }
    Ok(map)
}

pub(super) fn load_cached_display_resources(
    data_dir: &Path,
) -> Result<super::CachedDisplayResources, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let display_info = load_display_info_map(&conn)?;
    let material_to_texture = load_material_to_texture_map(&conn)?;
    let direct = load_display_materials_map(&conn)?;
    let model_to_fdids = load_model_to_fdids_map(&conn)?;
    Ok(super::CachedDisplayResources {
        display_info,
        material_to_texture,
        display_materials: DisplayMaterialTextures { direct },
        model_to_fdids,
    })
}

fn load_display_info_map(conn: &Connection) -> Result<HashMap<u32, DisplayInfoResolved>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, model_res_0, model_res_1, model_mat_res_0, model_mat_res_1,
                    geoset_group_0, geoset_group_1, geoset_group_2, helmet_vis_0, helmet_vis_1
             FROM display_info",
        )
        .map_err(|err| format!("prepare display_info lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, i16>(5)?,
                row.get::<_, i16>(6)?,
                row.get::<_, i16>(7)?,
                row.get::<_, u32>(8)?,
                row.get::<_, u32>(9)?,
            ))
        })
        .map_err(|err| format!("query display_info: {err}"))?;
    let mut display_info = HashMap::new();
    for row in rows {
        let (id, mr0, mr1, mm0, mm1, gg0, gg1, gg2, hv0, hv1) =
            row.map_err(|err| format!("read display_info row: {err}"))?;
        display_info.insert(
            id,
            build_display_info_row([mr0, mr1], [mm0, mm1], [gg0, gg1, gg2], [hv0, hv1]),
        );
    }
    Ok(display_info)
}

fn build_display_info_row(
    model_resources: [u32; 2],
    model_material_resources: [u32; 2],
    geoset_groups: [i16; 3],
    helmet_vis_ids: [u32; 2],
) -> DisplayInfoResolved {
    let collect = |values: [u32; 2]| values.into_iter().filter(|v| *v != 0).collect::<Vec<_>>();
    DisplayInfoResolved {
        item_textures: Vec::new(),
        geoset_overrides: Vec::new(),
        model_resource_ids: collect(model_resources),
        model_material_resource_ids: collect(model_material_resources),
        model_resource_columns: model_resources,
        model_material_resource_columns: model_material_resources,
        helmet_geoset_vis_ids: collect(helmet_vis_ids),
        geoset_groups: [geoset_groups[0], geoset_groups[1], geoset_groups[2], 0, 0, 0],
    }
}

fn load_display_materials_map(conn: &Connection) -> Result<HashMap<u32, Vec<(u8, u32)>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT display_info_id, component_section, texture_fdid
             FROM display_material_textures
             ORDER BY display_info_id, component_section, texture_fdid",
        )
        .map_err(|err| format!("prepare display_material_textures lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u8>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(|err| format!("query display_material_textures: {err}"))?;
    let mut direct = HashMap::new();
    for row in rows {
        let (display_info_id, component_section, texture_fdid) =
            row.map_err(|err| format!("read display_material_textures row: {err}"))?;
        direct
            .entry(display_info_id)
            .or_insert_with(Vec::new)
            .push((component_section, texture_fdid));
    }
    Ok(direct)
}

fn load_model_to_fdids_map(conn: &Connection) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT model_resource_id, file_data_id
             FROM model_to_fdid
             ORDER BY model_resource_id, model_order",
        )
        .map_err(|err| format!("prepare model_to_fdid lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query model_to_fdid: {err}"))?;
    let mut model_to_fdids = HashMap::new();
    for row in rows {
        let (model_resource_id, file_data_id) =
            row.map_err(|err| format!("read model_to_fdid row: {err}"))?;
        model_to_fdids
            .entry(model_resource_id)
            .or_insert_with(Vec::new)
            .push(file_data_id);
    }
    Ok(model_to_fdids)
}
