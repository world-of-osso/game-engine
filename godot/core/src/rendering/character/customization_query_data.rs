use std::collections::HashMap;

use rusqlite::Connection;

use crate::customization_data::{
    RaceModels, RawCategory, RawChoice, RawChrModel, RawData, RawElement, RawGeoset, RawMaterial,
    RawOption, RawSkinnedModel,
};

type HairGeosetKey = (u32, u16, u16);

/// Cache schema; the version is in the file name because checkouts and the two
/// clients share `data/cache`, and one name made each rebuild over the other's.
pub(crate) const CACHE_SCHEMA_VERSION: u32 = 4;

pub fn customization_cache_file() -> String {
    format!("customization-v{CACHE_SCHEMA_VERSION}.sqlite")
}

pub(crate) fn query_customization_raw_data(
    conn: &Connection,
    race_models: RaceModels,
) -> Result<RawData, String> {
    Ok(RawData {
        chr_models: load_chr_models(conn)?,
        options: load_options(conn)?,
        categories: load_categories(conn)?,
        choices: load_choices(conn)?,
        elements: load_elements(conn)?,
        materials: load_materials(conn)?,
        geosets: load_geosets(conn)?,
        skinned_models: load_skinned_models(conn)?,
        hair_geosets: load_hair_geosets(conn)?,
        texture_fdids: load_texture_fdids(conn)?,
        race_models,
    })
}

fn load_chr_models(conn: &Connection) -> Result<Vec<RawChrModel>, String> {
    let mut chr_models_stmt = conn
        .prepare("SELECT id, layout_id, customize_scale, camera_distance_offset FROM chr_models ORDER BY id")
        .map_err(|err| format!("prepare chr_models lookup: {err}"))?;
    chr_models_stmt
        .query_map([], |row| {
            Ok(RawChrModel {
                id: row.get(0)?,
                layout_id: row.get(1)?,
                customize_scale: row.get(2)?,
                camera_distance_offset: row.get(3)?,
            })
        })
        .map_err(|err| format!("query chr_models: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read chr_models row: {err}"))
}

fn load_options(conn: &Connection) -> Result<Vec<RawOption>, String> {
    let mut options_stmt = conn
        .prepare("SELECT id, name, chr_model_id, category_id, order_index, ui_type, requirement_id FROM options ORDER BY id")
        .map_err(|err| format!("prepare options lookup: {err}"))?;
    options_stmt
        .query_map([], |row| {
            Ok(RawOption {
                id: row.get(0)?,
                name: row.get(1)?,
                chr_model_id: row.get(2)?,
                category_id: row.get(3)?,
                order_index: row.get(4)?,
                ui_type: row.get(5)?,
                requirement_id: row.get(6)?,
            })
        })
        .map_err(|err| format!("query options: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read options row: {err}"))
}

fn load_categories(conn: &Connection) -> Result<HashMap<u32, RawCategory>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, order_index, icon, selected_icon FROM categories")
        .map_err(|err| format!("prepare categories lookup: {err}"))?;
    stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            RawCategory {
                name: row.get(1)?,
                order_index: row.get(2)?,
                icon: row.get(3)?,
                selected_icon: row.get(4)?,
            },
        ))
    })
    .map_err(|err| format!("query categories: {err}"))?
    .collect::<Result<HashMap<_, _>, _>>()
    .map_err(|err| format!("read categories row: {err}"))
}

fn load_choices(conn: &Connection) -> Result<Vec<RawChoice>, String> {
    let mut choices_stmt = conn
        .prepare("SELECT id, option_id, name, requirement_id, order_index, visibility_requirement_id, swatch_color_0, swatch_color_1 FROM choices ORDER BY id")
        .map_err(|err| format!("prepare choices lookup: {err}"))?;
    choices_stmt
        .query_map([], |row| {
            Ok(RawChoice {
                id: row.get(0)?,
                option_id: row.get(1)?,
                name: row.get(2)?,
                requirement_id: row.get(3)?,
                order_index: row.get(4)?,
                visibility_requirement_id: row.get(5)?,
                swatch_colors: [row.get(6)?, row.get(7)?],
            })
        })
        .map_err(|err| format!("query choices: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read choices row: {err}"))
}

fn load_elements(conn: &Connection) -> Result<Vec<RawElement>, String> {
    let mut elements_stmt = conn
        .prepare("SELECT choice_id, related_choice_id, geoset_id, material_id, skinned_model_id, has_unsupported_effects FROM elements")
        .map_err(|err| format!("prepare elements lookup: {err}"))?;
    elements_stmt
        .query_map([], |row| {
            Ok(RawElement {
                choice_id: row.get(0)?,
                related_choice_id: row.get(1)?,
                geoset_id: row.get(2)?,
                material_id: row.get(3)?,
                skinned_model_id: row.get(4)?,
                has_unsupported_effects: row.get(5)?,
            })
        })
        .map_err(|err| format!("query elements: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read elements row: {err}"))
}

fn load_materials(conn: &Connection) -> Result<HashMap<u32, RawMaterial>, String> {
    let mut materials_stmt = conn
        .prepare("SELECT id, texture_target_id, material_resources_id FROM materials")
        .map_err(|err| format!("prepare materials lookup: {err}"))?;
    materials_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                RawMaterial {
                    texture_target_id: row.get(1)?,
                    material_resources_id: row.get(2)?,
                },
            ))
        })
        .map_err(|err| format!("query materials: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read materials row: {err}"))
}

fn load_geosets(conn: &Connection) -> Result<HashMap<u32, RawGeoset>, String> {
    let mut geosets_stmt = conn
        .prepare("SELECT id, geoset_type, geoset_id FROM geosets")
        .map_err(|err| format!("prepare geosets lookup: {err}"))?;
    geosets_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                RawGeoset {
                    geoset_type: row.get(1)?,
                    geoset_id: row.get(2)?,
                },
            ))
        })
        .map_err(|err| format!("query geosets: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read geosets row: {err}"))
}

fn load_skinned_models(conn: &Connection) -> Result<HashMap<u32, RawSkinnedModel>, String> {
    let mut stmt = conn
        .prepare("SELECT id, collection_fdid, geoset_type, geoset_id FROM skinned_models")
        .map_err(|err| format!("prepare skinned_models lookup: {err}"))?;
    stmt.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            RawSkinnedModel {
                collection_fdid: row.get(1)?,
                geoset_type: row.get(2)?,
                geoset_id: row.get(3)?,
            },
        ))
    })
    .map_err(|err| format!("query skinned_models: {err}"))?
    .collect::<Result<HashMap<_, _>, _>>()
    .map_err(|err| format!("read skinned_models row: {err}"))
}

fn load_hair_geosets(conn: &Connection) -> Result<HashMap<HairGeosetKey, bool>, String> {
    let mut hair_stmt = conn
        .prepare("SELECT model_id, geoset_type, geoset_id, shows_scalp FROM hair_geosets")
        .map_err(|err| format!("prepare hair_geosets lookup: {err}"))?;
    hair_stmt
        .query_map([], |row| {
            Ok((
                (
                    row.get::<_, u32>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, u16>(2)?,
                ),
                row.get::<_, bool>(3)?,
            ))
        })
        .map_err(|err| format!("query hair_geosets: {err}"))?
        .collect::<Result<HashMap<HairGeosetKey, bool>, _>>()
        .map_err(|err| format!("read hair_geosets row: {err}"))
}

fn load_texture_fdids(conn: &Connection) -> Result<HashMap<u32, u32>, String> {
    let mut texture_stmt = conn
        .prepare("SELECT material_resources_id, file_data_id FROM texture_fdids")
        .map_err(|err| format!("prepare texture_fdids lookup: {err}"))?;
    texture_stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query texture_fdids: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read texture_fdids row: {err}"))
}
