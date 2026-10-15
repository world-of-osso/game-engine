use super::*;
use shared::{
    components::{EquipmentVisualSlot, EquippedAppearanceEntry},
    guild_creation::GuildTabardDesign,
};

#[test]
fn guild_tabard_character_selection_composes_the_roster_design() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let design = GuildTabardDesign {
        icon: 1,
        icon_color: 2,
        border: 3,
        border_color: 4,
        background_color: 5,
    };
    let entry = shared::protocol::CharacterListEntry {
        character_id: 7,
        name: "Guildmaster".into(),
        level: 80,
        race: 1,
        class: 2,
        appearance: Default::default(),
        tabard: Some(design),
        equipment_appearance: EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Tabard,
                item_id: Some(5976),
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                display_info_id: None,
                inventory_type: 19,
                hidden: false,
            }],
        },
    };
    let parts = crate::character_select::prepare_character_parts(&data, &entry)
        .expect("roster design must reach player model preparation");
    let painted = &parts.equipment.outfit.item_textures;
    for layer in [
        (3, 180071),
        (3, 180437),
        (3, 180273),
        (4, 180070),
        (4, 180436),
        (4, 180272),
    ] {
        assert!(
            painted.contains(&layer),
            "missing registered torso layer {layer:?}: {painted:?}"
        );
    }
}
