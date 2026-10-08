//! Read imported NPC customization and compositor catalogs without engine dependencies.
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::{
    char_texture_data::CharTextureData,
    char_texture_query_data::{
        char_texture_cache_file, query_char_texture_data, query_model_material_sizes,
    },
    customization_data::{CustomizationDb, RaceModels},
    customization_query_data::{
        customization_cache_file, query_customization_raw_data,
        query_skyborne_customization_raw_data,
    },
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
    let mut db = read_customization_db(data_root, false)?;
    let forever = data_root.join(crate::player_model_data::FOREVER_DB2_DIR);
    if needs_forever(data_root, &forever)? {
        crate::customization_cache::import_customization_cache(&forever)?;
        db.overlay_forever(read_customization_db(&forever, true)?)?;
    }
    Ok(db)
}

fn needs_forever(data_root: &Path, forever: &Path) -> Result<bool, String> {
    if forever.exists() {
        return Ok(true);
    }
    let races = RaceModels::load(data_root)?;
    if crate::player_model_data::FOREVER_RACES
        .iter()
        .any(|&race| races.chr_model_id(race, 0).is_some())
    {
        return Err(format!(
            "missing required Forever Skyborne catalog {}",
            forever.display()
        ));
    }
    Ok(false)
}

fn read_customization_db(data_root: &Path, skyborne_only: bool) -> Result<CustomizationDb, String> {
    let connection = open_catalog(data_root, &customization_cache_file())?;
    let races = RaceModels::load(data_root)?;
    let raw = if skyborne_only {
        query_skyborne_customization_raw_data(&connection, races)?
    } else {
        query_customization_raw_data(&connection, races)?
    };
    let mut db = CustomizationDb::from_raw(&raw);
    db.load_requirements(data_root)?;
    Ok(db)
}

pub fn load_compositor(data_root: &Path) -> Result<CharTextureData, String> {
    let mut compositor = read_compositor(data_root)?;
    let forever = data_root.join(crate::player_model_data::FOREVER_DB2_DIR);
    if needs_forever(data_root, &forever)? {
        crate::char_texture_cache::import_char_texture_cache(&forever)?;
        let mut overlay = read_compositor(&forever)?;
        // ChrModel 218/219 use layouts 201/202 in Forever 1.60.1.70205.
        let layouts = [201, 202];
        for id in layouts {
            let layout = overlay
                .layouts
                .remove(&id)
                .ok_or_else(|| format!("missing Forever texture layout {id}"))?;
            compositor.layouts.insert(id, layout);
        }
        compositor
            .layers
            .retain(|layer| !layouts.contains(&layer.layout_id));
        compositor.layers.extend(
            overlay
                .layers
                .into_iter()
                .filter(|layer| layouts.contains(&layer.layout_id)),
        );
        compositor
            .sections
            .retain(|(layout, _), _| !layouts.contains(layout));
        compositor.sections.extend(
            overlay
                .sections
                .into_iter()
                .filter(|((layout, _), _)| layouts.contains(layout)),
        );
        compositor
            .material_sizes
            .retain(|(layout, _), _| !layouts.contains(layout));
        compositor.material_sizes.extend(
            overlay
                .material_sizes
                .into_iter()
                .filter(|((layout, _), _)| layouts.contains(layout)),
        );
    }
    Ok(compositor)
}

fn read_compositor(data_root: &Path) -> Result<CharTextureData, String> {
    let connection = open_catalog(data_root, char_texture_cache_file())?;
    let (layers, sections, layouts) = query_char_texture_data(&connection)?;
    Ok(CharTextureData::from_parts(layers, sections, layouts)
        .with_material_sizes(query_model_material_sizes(&connection)?))
}
