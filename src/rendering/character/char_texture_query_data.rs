use crate::asset::char_texture::{TextureLayer, TextureLayout, TextureSection};
use rusqlite::Connection;
use std::collections::HashMap;

/// Cache layout version in the file name (see `customization_cache_file`); 2 adds
/// `model_materials`.
pub fn char_texture_cache_file() -> &'static str {
    "char_texture-v2.sqlite"
}

pub type CharTextureCacheData = (
    Vec<TextureLayer>,
    HashMap<(u32, u32), TextureSection>,
    HashMap<u32, TextureLayout>,
);

pub fn query_char_texture_data(conn: &Connection) -> Result<CharTextureCacheData, String> {
    let layers = load_layers(conn)?;
    let sections = load_sections(conn)?;
    let layouts = load_layouts(conn)?;
    Ok((layers, sections, layouts))
}

fn load_layers(conn: &Connection) -> Result<Vec<TextureLayer>, String> {
    let mut layers_stmt = conn
        .prepare(
            "SELECT texture_type, layer, blend_mode, section_bitmask, target_id, layout_id
             FROM layers
             ORDER BY layout_id, texture_type, layer",
        )
        .map_err(|err| format!("prepare layers lookup: {err}"))?;
    let layers = layers_stmt
        .query_map([], |row| {
            Ok(TextureLayer {
                texture_type: row.get(0)?,
                layer: row.get(1)?,
                blend_mode: row.get(2)?,
                section_bitmask: row.get(3)?,
                target_id: row.get(4)?,
                layout_id: row.get(5)?,
            })
        })
        .map_err(|err| format!("query layers: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read layers row: {err}"))?;
    Ok(layers)
}

fn load_sections(conn: &Connection) -> Result<HashMap<(u32, u32), TextureSection>, String> {
    let mut sections_stmt = conn
        .prepare("SELECT layout_id, section_type, x, y, width, height FROM sections")
        .map_err(|err| format!("prepare sections lookup: {err}"))?;
    let sections = sections_stmt
        .query_map([], |row| {
            Ok((
                (row.get::<_, u32>(0)?, row.get::<_, u32>(1)?),
                TextureSection {
                    x: row.get(2)?,
                    y: row.get(3)?,
                    width: row.get(4)?,
                    height: row.get(5)?,
                },
            ))
        })
        .map_err(|err| format!("query sections: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read sections row: {err}"))?;
    Ok(sections)
}

fn load_layouts(conn: &Connection) -> Result<HashMap<u32, TextureLayout>, String> {
    let mut layouts_stmt = conn
        .prepare("SELECT id, width, height FROM layouts")
        .map_err(|err| format!("prepare layouts lookup: {err}"))?;
    let layouts = layouts_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                TextureLayout {
                    width: row.get(1)?,
                    height: row.get(2)?,
                },
            ))
        })
        .map_err(|err| format!("query layouts: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read layouts row: {err}"))?;
    Ok(layouts)
}

/// ChrModelMaterial canvas sizes: (layout ID, M2 texture type) -> (width, height).
pub fn query_model_material_sizes(
    conn: &Connection,
) -> Result<HashMap<(u32, u32), (u32, u32)>, String> {
    let mut stmt = conn
        .prepare("SELECT layout_id, texture_type, width, height FROM model_materials")
        .map_err(|err| format!("prepare model_materials lookup: {err}"))?;
    stmt.query_map([], |row| {
        Ok((
            (row.get::<_, u32>(0)?, row.get::<_, u32>(1)?),
            (row.get::<_, u32>(2)?, row.get::<_, u32>(3)?),
        ))
    })
    .map_err(|err| format!("query model_materials: {err}"))?
    .collect::<Result<HashMap<_, _>, _>>()
    .map_err(|err| format!("read model_materials row: {err}"))
}
