//! Read imported NPC customization and compositor catalogs without engine dependencies.
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::{
    char_texture_data::CharTextureData,
    char_texture_query_data::{query_char_texture_data, query_model_material_sizes},
    customization_data::{CustomizationDb, RaceModels},
    customization_query_data::query_customization_raw_data,
};

fn open_catalog(data_root: &Path, name: &str) -> Result<Connection, String> {
    let path = data_root.join("cache").join(name);
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| {
        format!(
            "Cannot open NPC appearance catalog {}: {error}",
            path.display()
        )
    })
}

pub fn load_customization_db(data_root: &Path) -> Result<CustomizationDb, String> {
    let connection = open_catalog(data_root, "customization.sqlite")?;
    let races = RaceModels::load(data_root)?;
    let raw = query_customization_raw_data(&connection, races)?;
    let mut db = CustomizationDb::from_raw(&raw);
    db.load_requirements(data_root)?;
    Ok(db)
}

pub fn load_compositor(data_root: &Path) -> Result<CharTextureData, String> {
    let connection = open_catalog(data_root, "char_texture.sqlite")?;
    let (layers, sections, layouts) = query_char_texture_data(&connection)?;
    Ok(CharTextureData::from_parts(layers, sections, layouts)
        .with_material_sizes(query_model_material_sizes(&connection)?))
}
