use std::{collections::HashMap, path::Path};

use game_engine_core::{
    asset::m2_texture,
    blp,
    npc_appearance_assets::{load_compositor, load_customization_db},
    npc_appearance_data::query_authored_npc_appearance,
    npc_appearance_selection_data::{select_npc_choices, select_npc_type6_texture},
    outfit_data::OutfitData,
};
use rusqlite::{Connection, OpenFlags};

fn read_armor_ids(data: &Path, extra: u32) -> Vec<u32> {
    let path = data.join("db2/1.60.1.70205/NPCModelItemSlotDisplayInfo.csv");
    let mut items = Vec::new();
    game_engine_core::csv_util::read_numeric_rows(
        &path,
        ["NpcModelID", "ItemSlot", "ItemDisplayInfoID"],
        |[npc, slot, item]| {
            if npc == i64::from(extra) && slot != 11 {
                items.push(u32::try_from(item).unwrap());
            }
        },
    )
    .unwrap();
    items
}

#[test]
fn forever_npc_profiles_ventaari_composes_authored_textures_and_gear() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let connection = Connection::open_with_flags(
        data.join("cache/npc_appearance.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let appearance = query_authored_npc_appearance(&connection, 139694)
        .unwrap()
        .unwrap();
    assert_eq!((appearance.race, appearance.sex), (96, 1));
    assert_eq!(appearance.baked_texture_fdid, Some(7487478));
    assert!(!appearance.choice_ids.is_empty());
    let models = Connection::open_with_flags(
        data.join("cache/creature_display.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    for display in [136968, 139694] {
        let fdid: u32 = models
            .query_row(
                "SELECT model_fdid FROM creature_displays WHERE display_id=?1",
                [display],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fdid, 7478494, "both authored NPCs use the female body");
    }
    let db = load_customization_db(&data).unwrap();
    let selected = select_npc_choices(&appearance, &db).unwrap();
    assert!(!selected.materials.is_empty());
    let layout = db.layout_id(appearance.race, appearance.sex).unwrap();
    assert_eq!(layout, 202);
    let compositor = load_compositor(&data).unwrap();
    let size = compositor.layout(layout).unwrap();
    let default =
        m2_texture::default_fdid_for_type(1, size.width == 2048 && size.height == 1024, &[0, 0, 0])
            .unwrap();
    let mut decoded = HashMap::new();
    for fdid in selected
        .materials
        .iter()
        .map(|(_, fdid)| *fdid)
        .chain([default, appearance.baked_texture_fdid.unwrap()])
    {
        let bytes = std::fs::read(data.join(format!("textures/{fdid}.blp"))).unwrap();
        let image = blp::decode_rgba(&bytes).unwrap();
        assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
        decoded.insert(fdid, (image.pixels, image.width, image.height));
    }
    let composed = compositor
        .composite_model_textures_with(&selected.materials, &[], layout, default, |fdid| {
            decoded.get(&fdid).cloned()
        })
        .unwrap();
    assert!(composed.body.0.chunks_exact(4).any(|pixel| pixel[3] != 0));
    let type6 = select_npc_type6_texture(
        compositor.declares_hair(&selected.materials, layout),
        composed.hair,
        composed.head,
    )
    .unwrap();
    assert!(type6.is_some());
    let outfit = OutfitData::load(&data);
    let gear = read_armor_ids(&data, 163204);
    assert!(!gear.is_empty());
    for item in gear {
        let resolved = outfit
            .try_resolve_display_info(item, appearance.race, appearance.sex)
            .unwrap()
            .unwrap();
        assert!(
            !resolved.model_fdids.is_empty()
                || !resolved.item_textures.is_empty()
                || !resolved.geoset_overrides.is_empty(),
            "empty gear {item}"
        );
    }
}

#[test]
fn forever_npc_profiles_ailee_authored_bake_does_not_require_body_overlay_mapping() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let connection = Connection::open_with_flags(
        data.join("cache/npc_appearance.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let appearance = query_authored_npc_appearance(&connection, 136968)
        .unwrap()
        .unwrap();
    assert_eq!((appearance.race, appearance.sex), (95, 1));
    assert_eq!(appearance.baked_texture_fdid, Some(7352105));
    assert!(!appearance.choice_ids.is_empty());
    let image =
        blp::decode_rgba(&std::fs::read(data.join("textures/7352105.blp")).unwrap()).unwrap();
    assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    let outfit = OutfitData::load(&data);
    let failures: Vec<_> = read_armor_ids(&data, 162359)
        .into_iter()
        .filter_map(|item| outfit.try_resolve_display_info(item, 95, 1).err())
        .collect();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("1102747"), "{failures:?}");
    for item in read_armor_ids(&data, 162359) {
        let baked = outfit
            .try_load_baked_display_info(item, 95, 1)
            .unwrap()
            .unwrap();
        assert!(baked.item_textures.is_empty(), "baked gear {item}");
    }
}
