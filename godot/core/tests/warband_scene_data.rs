use std::path::Path;

use game_engine_core::warband_scene_data::{
    AtlasArt, read_atlas_art, read_authored_catalog, read_texture_kit_art,
};

#[test]
fn authored_adventurers_rest_records_preserve_camera_focus_and_primary_tile() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = read_authored_catalog(&data_root).expect("read authored Warband CSVs");
    let scene = catalog
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");

    assert_eq!(scene.name, "Adventurer's Rest");
    assert_eq!(scene.map_id, 2703);
    assert_eq!(scene.position, [-2982.99, 468.057, 455.523]);
    assert_eq!(scene.look_at, [-2985.54, 456.018, 454.399]);
    assert_eq!(scene.fov, 65.0);
    assert_eq!(scene.texture_kit, 5671);
    assert_eq!(scene.tile_coords(), (31, 37));
    assert_eq!(scene.map_name(), "2703");
    assert!(!catalog.scenes.iter().any(|scene| scene.id == 29));

    let first_slot = catalog
        .placements
        .iter()
        .find(|placement| {
            placement.scene_id == 1 && placement.slot_id == 0 && placement.is_character_slot()
        })
        .expect("authored character slot zero");
    assert_eq!(first_slot.id, 1);
    assert_eq!(first_slot.position, [-2981.82, 457.35, 452.826]);
    assert_eq!(first_slot.rotation, 108.0);
    assert!(!catalog.placement_options.is_empty());
}

#[test]
fn missing_authored_catalog_is_an_error_not_an_empty_selection() {
    let absent =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/nonexistent-warband-fixture");
    let error = read_authored_catalog(&absent).expect_err("missing CSV must fail");
    assert!(error.contains("WarbandScene.csv"), "{error}");
}

/// Retail `CampSiteTemplate` calls `Icon:SetAtlas(warbandSceneInfo.textureKit)`: the scene's
/// `UiTextureKit.KitPrefix` is the atlas element drawn on its campsite card.
#[test]
fn every_listed_campsite_resolves_its_texture_kit_atlas_art() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = read_authored_catalog(&data_root).expect("read authored Warband CSVs");
    let kits: Vec<u32> = catalog
        .scenes
        .iter()
        .map(|scene| scene.texture_kit)
        .collect();
    let art = read_texture_kit_art(&data_root, &kits).expect("read texture kit atlas tables");

    for scene in &catalog.scenes {
        let art = art
            .get(&scene.texture_kit)
            .unwrap_or_else(|| panic!("scene {} ({}) has no kit art", scene.id, scene.name));
        let texture = data_root.join(format!("textures/{}.blp", art.fdid));
        assert!(
            texture.is_file(),
            "{} missing for {}",
            texture.display(),
            scene.name
        );
        let [left, right, top, bottom] = art.tex_coords;
        assert!(
            (0.0..right).contains(&left) && right <= 1.0,
            "{}: {art:?}",
            scene.name
        );
        assert!(
            (0.0..bottom).contains(&top) && bottom <= 1.0,
            "{}: {art:?}",
            scene.name
        );
    }

    // campcollection-bg-image5..8 members on UiTextureAtlas 3086 (FDID 6375814, 1024x1024).
    for (scene_id, name, [left, right, top, bottom]) in [
        (25, "Gallagio Grand Gallery", [1.0, 215.0, 176.0, 349.0]),
        (
            119,
            "The Fate of the Devoured",
            [217.0, 431.0, 176.0, 349.0],
        ),
        (145, "Razorwind Shores", [433.0, 647.0, 176.0, 349.0]),
        (146, "Founders Point", [649.0, 863.0, 176.0, 349.0]),
    ] {
        let scene = catalog
            .scenes
            .iter()
            .find(|scene| scene.id == scene_id)
            .unwrap_or_else(|| panic!("listed campsite {name}"));
        assert_eq!(scene.name, name);
        let expected = AtlasArt {
            fdid: 6_375_814,
            tex_coords: [left / 1024.0, right / 1024.0, top / 1024.0, bottom / 1024.0],
        };
        assert_eq!(art.get(&scene.texture_kit), Some(&expected), "{name}");
    }
}

#[test]
fn missing_texture_kit_table_is_an_error() {
    let absent =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/nonexistent-warband-fixture");
    let error = read_texture_kit_art(&absent, &[5743]).expect_err("missing CSV must fail");
    assert!(error.contains("UiTextureKit.csv"), "{error}");
}

#[test]
fn atlas_names_resolve_to_their_scale_one_members() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let art = read_atlas_art(
        &data_root,
        &[
            "UI-CastingBar-Pip",
            "ui-castingbar-pip-red",
            "nameplates-InterruptShield",
        ],
    )
    .expect("read atlas tables");
    // Member of the same name in UiTextureAtlas 1942 (512x256).
    assert_eq!(
        art["ui-castingbar-pip"],
        AtlasArt {
            fdid: 4505182,
            tex_coords: [1.0 / 512.0, 7.0 / 512.0, 151.0 / 256.0, 181.0 / 256.0],
        }
    );
    // No member named after the element: its 6x30 `-1x_red`, not the 10x60 `-2x_red`.
    assert_eq!(
        art["ui-castingbar-pip-red"],
        AtlasArt {
            fdid: 4549775,
            tex_coords: [1009.0 / 1024.0, 1015.0 / 1024.0, 69.0 / 512.0, 99.0 / 512.0],
        }
    );
    assert_eq!(art["nameplates-interruptshield"].fdid, 1300837);
    assert!(read_atlas_art(&data_root, &["no-such-atlas"]).is_err());
}
