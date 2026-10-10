//! Offline refresh of the metadata that selects model asset products.
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
#[path = "../../rust/src/game/equipment/equipment_appearance_data.rs"]
pub mod equipment_appearance_data;
#[path = "../../rust/src/game/creatures/npc_gear_data.rs"]
pub mod npc_gear_data;
use game_engine_core::asset_product::AssetTexture;
pub use game_engine_core::{asset, customization_data, outfit_data};
#[path = "../../rust/src/rendering/character/appearance_options.rs"]
pub mod appearance_options;

fn main() {
    let data = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("data root argument required"),
    );
    let result = import(&data);
    if let Err(error) = result {
        eprintln!("Model metadata import: {error}");
        std::process::exit(1);
    }
}

fn import(data: &Path) -> Result<(), String> {
    let displays = game_engine_core::creature_display_cache::import_creature_display_cache(data)?;
    let outfits = game_engine_core::outfit_catalog_db::import_outfit_links_cache(data)?;
    let conn = rusqlite::Connection::open(displays).map_err(|error| error.to_string())?;
    for id in [139403, 139409] {
        let display = game_engine_core::creature_display_data::query_display(&conn, id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Display {id} missing"))?;
        println!("Display {id}: {display:?}");
    }
    println!("Outfit cache: {}", outfits.display());
    write_required_assets(data, &conn)?;
    Ok(())
}

fn write_required_assets(data: &Path, displays: &rusqlite::Connection) -> Result<(), String> {
    let profiles = rusqlite::Connection::open(data.join("cache/npc_appearance.sqlite"))
        .map_err(|error| error.to_string())?;
    let mut catalogs = game_engine_core::npc_appearance_assets::NpcAppearanceCatalogs::load(data)?;
    let gear = npc_gear_data::NpcGearData::load(&data.join("db2/12.1.0.69933"))?;
    let outfit = outfit_data::OutfitData::load(data);
    let mut assets = BTreeSet::new();
    for id in [139403, 139409, 21774] {
        let display = game_engine_core::creature_display_data::query_display(displays, id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Display {id} missing"))?;
        assets.insert((display.source_product.as_str(), display.model_fdid, "m2"));
        let Some(appearance) =
            game_engine_core::npc_appearance_data::query_authored_npc_appearance(&profiles, id)?
        else {
            continue;
        };
        let (db, compositor) = catalogs.load_for_display(id, appearance.race, appearance.sex)?;
        let selected =
            game_engine_core::npc_appearance_selection_data::select_npc_choices(&appearance, db)?;
        let product = display.source_product;
        for &(_, fdid) in &selected.materials {
            assets.insert((product.as_str(), fdid, "blp"));
        }
        if let Some(fdid) = appearance.baked_texture_fdid {
            assets.insert((product.as_str(), fdid, "blp"));
        }
        let layout_id = db
            .layout_id(appearance.race, appearance.sex)
            .ok_or("NPC layout absent")?;
        let layout = compositor
            .layout(layout_id)
            .ok_or("NPC compositor layout absent")?;
        let default = game_engine_core::asset::m2_texture::default_fdid_for_type(
            1,
            layout.width == 2048 && layout.height == 1024,
            &[0; 3],
        )
        .ok_or("NPC default texture absent")?;
        assets.insert((product.as_str(), default, "blp"));
        let items = gear.display_armor(id)?;
        let armor = if appearance.baked_texture_fdid.is_some() {
            equipment_appearance_data::load_baked_equipment_appearance(
                &items,
                &outfit,
                appearance.race,
                appearance.sex,
            )?
        } else {
            equipment_appearance_data::resolve_equipment_appearance(
                &items,
                &outfit,
                appearance.race,
                appearance.sex,
            )?
        };
        add_armor_assets(&mut assets, &armor)?;
    }
    add_equipped_player_assets(data, &outfit, &mut assets)?;
    let lines: Vec<_> = assets
        .into_iter()
        .map(|(product, fdid, kind)| format!("{product},{fdid},{kind}"))
        .collect();
    let output = data.join("diagnostics/modelisolation-2026-10-09/native-required.csv");
    std::fs::write(&output, lines.join("\n") + "\n").map_err(|error| error.to_string())?;
    println!(
        "Native required assets: {} at {}",
        lines.len(),
        output.display()
    );
    Ok(())
}

fn add_equipped_player_assets(
    data: &Path,
    outfit: &outfit_data::OutfitData,
    assets: &mut BTreeSet<(&'static str, u32, &'static str)>,
) -> Result<(), String> {
    use game_engine_core::npc_appearance_assets::{load_compositor, load_customization_db};
    use shared::components::{
        CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
    };
    let body =
        game_engine_core::player_model_data::player_model_fdids(&data.join("db2/12.1.0.69933"))?;
    assets.insert((
        "wow",
        *body.get(&(1, 0)).ok_or("Human male model absent")?,
        "m2",
    ));
    let db = load_customization_db(data)?;
    let selected =
        appearance_options::selected_choices(&db, 1, 0, 1, &CharacterAppearance::default());
    let ids: BTreeSet<_> = selected.iter().map(|choice| choice.id).collect();
    for choice in selected {
        for &(_, fdid) in &choice.materials {
            assets.insert(("wow", fdid, "blp"));
        }
        for material in &choice.related_materials {
            if ids.contains(&material.related_choice_id) {
                assets.insert(("wow", material.fdid, "blp"));
            }
        }
        for model in &choice.skinned_models {
            if model.related_choice_id == 0 || ids.contains(&model.related_choice_id) {
                assets.insert(("wow", model.collection_fdid, "m2"));
            }
        }
    }
    let compositor = load_compositor(data)?;
    let layout = compositor
        .layout(db.layout_id(1, 0).ok_or("Human male layout absent")?)
        .ok_or("Human male compositor layout absent")?;
    let default = game_engine_core::asset::m2_texture::default_fdid_for_type(
        1,
        layout.width == 2048 && layout.height == 1024,
        &[0; 3],
    )
    .ok_or("Human male default texture absent")?;
    assets.insert(("wow", default, "blp"));
    let equipment = EquipmentAppearance {
        entries: vec![EquippedAppearanceEntry {
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            slot: EquipmentVisualSlot::MainHand,
            item_id: Some(25),
            display_info_id: None,
            inventory_type: 21,
            hidden: false,
        }],
    };
    let armor = equipment_appearance_data::resolve_equipment_appearance(&equipment, outfit, 1, 0)?;
    add_armor_assets(assets, &armor)
}

fn add_armor_assets(
    assets: &mut BTreeSet<(&'static str, u32, &'static str)>,
    armor: &equipment_appearance_data::ResolvedEquipmentAppearance,
) -> Result<(), String> {
    let mut add_texture = |texture: AssetTexture| {
        assets.insert((texture.product.as_str(), texture.fdid, "blp"));
    };
    for &(_, texture) in &armor.body_texture_assets {
        add_texture(texture);
    }
    if let Some(texture) = armor.merged_cape_texture {
        add_texture(texture);
    }
    for model in &armor.runtime_models {
        for (index, &fdid) in model.skin_fdids.iter().enumerate() {
            if fdid != 0 {
                let product = model.skin_products[index].ok_or("Equipment skin source absent")?;
                add_texture(AssetTexture { product, fdid });
            }
        }
        for &(_, texture) in &model.texture_replacements {
            add_texture(texture);
        }
    }
    for model in &armor.runtime_models {
        assets.insert((model.product.as_str(), model.fdid, "m2"));
    }
    Ok(())
}
