//! Read imported NPC customization and compositor catalogs without engine dependencies.
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OpenFlags};

use crate::{
    char_texture_data::CharTextureData,
    char_texture_query_data::{
        char_texture_cache_file, query_char_texture_data, query_model_material_sizes,
    },
    customization_data::{CustomizationDb, RaceModels},
    customization_query_data::{
        customization_cache_file, query_customization_raw_data, query_npc_customization_raw_data,
        query_skyborne_customization_raw_data,
    },
};

/// NPC source is the CDI catalog that owns its display, never its race.
/// Retail IDs keep Retail precedence; Forever-only IDs read a separate full catalog.
pub struct NpcAppearanceCatalogs {
    data_root: PathBuf,
    retail_displays: HashSet<u32>,
    forever_displays: HashSet<u32>,
    retail_customization: CustomizationDb,
    retail_compositor: CharTextureData,
    forever_customization: HashMap<(u8, u8), CustomizationDb>,
    forever_compositor: Option<CharTextureData>,
}

impl NpcAppearanceCatalogs {
    pub fn load(data_root: &Path) -> Result<Self, String> {
        let mut retail_displays = HashSet::new();
        crate::csv_util::read_numeric_rows(
            &data_root.join("db2/12.1.0.69933/CreatureDisplayInfo.csv"),
            ["ID"],
            |[id]| {
                retail_displays.insert(id as u32);
            },
        )?;
        let mut forever_displays = HashSet::new();
        let forever = data_root.join(crate::player_model_data::FOREVER_DB2_DIR);
        if needs_forever(data_root, &forever)? {
            crate::csv_util::read_numeric_rows(
                &forever.join("CreatureDisplayInfo.csv"),
                ["ID"],
                |[id]| {
                    forever_displays.insert(id as u32);
                },
            )?;
        }
        Ok(Self {
            data_root: data_root.to_owned(),
            retail_displays,
            forever_displays,
            retail_customization: read_customization_db(data_root, false)?,
            retail_compositor: read_compositor(data_root)?,
            forever_customization: HashMap::new(),
            forever_compositor: None,
        })
    }

    pub fn load_for_display(
        &mut self,
        display_id: u32,
        race: u8,
        sex: u8,
    ) -> Result<(&CustomizationDb, &CharTextureData), String> {
        if self.retail_displays.contains(&display_id) {
            return Ok((&self.retail_customization, &self.retail_compositor));
        }
        if !self.forever_displays.contains(&display_id) {
            return Err(format!(
                "NPC display {display_id} has no authored CDI source"
            ));
        }
        let forever = self
            .data_root
            .join(crate::player_model_data::FOREVER_DB2_DIR);
        if self.forever_compositor.is_none() {
            self.forever_compositor = Some(read_compositor(&forever)?);
        }
        if let std::collections::hash_map::Entry::Vacant(entry) =
            self.forever_customization.entry((race, sex))
        {
            entry.insert(load_forever_npc_customization_db(&forever, race, sex)?);
        }
        Ok((
            self.forever_customization
                .get(&(race, sex))
                .expect("loaded NPC catalog"),
            self.forever_compositor
                .as_ref()
                .expect("loaded NPC compositor"),
        ))
    }
}

fn load_forever_npc_customization_db(
    data_root: &Path,
    race: u8,
    sex: u8,
) -> Result<CustomizationDb, String> {
    let connection = open_catalog(data_root, &customization_cache_file())?;
    let races = RaceModels::load(data_root)?.for_npc(race, sex)?;
    let raw = query_npc_customization_raw_data(&connection, races)?;
    let mut db = CustomizationDb::from_raw(&raw);
    db.load_requirements(data_root)?;
    Ok(db)
}

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
