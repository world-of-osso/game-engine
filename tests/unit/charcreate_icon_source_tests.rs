//! Local-asset regression: original artwork survives the RSX → registry → native path.
use super::GameBlpLoader;
use bevy::prelude::*;
use game_engine::ui::screens::char_create_component::{CharCreateUiState, char_create_screen};
use ui_toolkit::native_render::RegistryNode;
use ui_toolkit::plugin::UiState;
use ui_toolkit::render_texture::{BlpLoader, BlpLoaderRes};
use ui_toolkit::screen::{Screen, SharedContext};

#[path = "../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

// Exact original paths matched case-insensitively against LOCAL community-listfile.csv.
const ICONS: &[(&str, u32, &str)] = &[
    (
        "Race_1_Icon",
        236448,
        "Interface/ICONS/Achievement_Character_Human_Male.blp",
    ),
    (
        "Race_3_Icon",
        236444,
        "Interface/ICONS/Achievement_Character_Dwarf_Male.blp",
    ),
    (
        "Race_4_Icon",
        236450,
        "Interface/ICONS/Achievement_Character_Nightelf_Male.blp",
    ),
    (
        "Race_7_Icon",
        236446,
        "Interface/ICONS/Achievement_Character_Gnome_Male.blp",
    ),
    (
        "Race_11_Icon",
        236442,
        "Interface/ICONS/Achievement_Character_Draenei_Male.blp",
    ),
    (
        "Race_22_Icon",
        455993,
        "Interface/CHARACTERFRAME/TEMPORARYPORTRAIT-MALE-WORGEN.BLP",
    ),
    (
        "Race_29_Icon",
        1786422,
        "Interface/ICONS/Achievement_AlliedRace_VoidElf.blp",
    ),
    (
        "Race_30_Icon",
        1786420,
        "Interface/ICONS/Achievement_AlliedRace_LightforgedDraenei.blp",
    ),
    (
        "Race_34_Icon",
        1851464,
        "Interface/ICONS/Achievement_AlliedRace_DarkIronDwarf.blp",
    ),
    (
        "Race_37_Icon",
        3208032,
        "Interface/ICONS/Achievement_AlliedRace_Mechagnome.blp",
    ),
    (
        "Race_2_Icon",
        236452,
        "Interface/ICONS/Achievement_Character_Orc_Male.blp",
    ),
    (
        "Race_5_Icon",
        236458,
        "Interface/ICONS/Achievement_Character_Undead_Male.blp",
    ),
    (
        "Race_6_Icon",
        236454,
        "Interface/ICONS/Achievement_Character_Tauren_Male.blp",
    ),
    (
        "Race_8_Icon",
        236456,
        "Interface/ICONS/Achievement_Character_Troll_Male.blp",
    ),
    (
        "Race_10_Icon",
        236440,
        "Interface/ICONS/Achievement_Character_Bloodelf_Male.blp",
    ),
    (
        "Race_9_Icon",
        463874,
        "Interface/ICONS/achievement_Goblinhead.blp",
    ),
    (
        "Race_27_Icon",
        1786421,
        "Interface/ICONS/Achievement_AlliedRace_Nightborne.blp",
    ),
    (
        "Race_28_Icon",
        1786419,
        "Interface/ICONS/Achievement_AlliedRace_HighmountainTauren.blp",
    ),
    (
        "Race_31_Icon",
        1851465,
        "Interface/ICONS/Achievement_AlliedRace_ZandalariTroll.blp",
    ),
    (
        "Race_35_Icon",
        3208033,
        "Interface/ICONS/Achievement_AlliedRace_Vulpera.blp",
    ),
    (
        "Race_36_Icon",
        1989713,
        "Interface/ICONS/Achievement_AlliedRace_MagharOrc.blp",
    ),
    (
        "Race_25_Icon",
        626190,
        "Interface/ICONS/Achievement_Character_Pandaren_Female.blp",
    ),
    (
        "Class_1_Icon",
        626008,
        "Interface/ICONS/ClassIcon_Warrior.blp",
    ),
    (
        "Class_2_Icon",
        626003,
        "Interface/ICONS/ClassIcon_Paladin.blp",
    ),
    (
        "Class_3_Icon",
        626000,
        "Interface/ICONS/ClassIcon_Hunter.blp",
    ),
    (
        "Class_4_Icon",
        626005,
        "Interface/ICONS/ClassIcon_Rogue.blp",
    ),
    (
        "Class_5_Icon",
        626004,
        "Interface/ICONS/ClassIcon_Priest.blp",
    ),
    (
        "Class_6_Icon",
        625998,
        "Interface/ICONS/ClassIcon_DeathKnight.blp",
    ),
    (
        "Class_7_Icon",
        626006,
        "Interface/ICONS/ClassIcon_Shaman.blp",
    ),
    ("Class_8_Icon", 626001, "Interface/ICONS/ClassIcon_Mage.blp"),
    (
        "Class_9_Icon",
        626007,
        "Interface/ICONS/ClassIcon_Warlock.blp",
    ),
    (
        "Class_11_Icon",
        625999,
        "Interface/ICONS/ClassIcon_Druid.blp",
    ),
];

#[test]
fn charcreate_icons_resolve_original_art_from_local_casc() {
    let resolver = game_engine::asset::asset_resolver::resolver();
    let loader = GameBlpLoader;
    assert_eq!(ICONS.len(), 32);
    for &(name, fdid, original_path) in ICONS {
        let obsolete = std::path::Path::new("/home/osso/Projects/wow").join(original_path);
        assert!(
            !obsolete.exists(),
            "absent-file fixture changed: {obsolete:?}"
        );
        assert!(loader.load_blp_to_image(&obsolete).is_err());
        assert_eq!(resolver.lookup_path(original_path), Some(fdid), "{name}");
        let bytes = resolver
            .resolve_bytes(fdid)
            .unwrap_or_else(|| panic!("local CASC missing {name} ({fdid})"));
        let cached = loader
            .ensure_texture(fdid)
            .unwrap_or_else(|| panic!("loader failed {name} ({fdid})"));
        assert_eq!(
            std::fs::read(&cached).unwrap(),
            bytes,
            "cached artwork differs: {name}"
        );
        let image = loader.load_blp_to_image(&cached).unwrap();
        assert!(image.width() > 0 && image.height() > 0, "{name}");
        let pixels = image.data.as_ref().unwrap();
        assert_eq!(
            pixels.len(),
            (image.width() * image.height() * 4) as usize,
            "{name}"
        );
        assert!(
            pixels.chunks_exact(4).any(|p| p[3] > 0),
            "transparent artwork: {name}"
        );
        assert!(
            pixels.chunks_exact(4).any(|p| p != &pixels[..4]),
            "uniform artwork: {name}"
        );
        println!(
            "decoded {name}: FDID {fdid}, {}x{}, {} RGBA bytes",
            image.width(),
            image.height(),
            pixels.len()
        );
    }
}

fn icon_screen_app() -> App {
    let mut app = layout_support::layout_app(1920.0, 1080.0);
    app.insert_resource(BlpLoaderRes(Box::new(GameBlpLoader)));
    app.finish();
    app.cleanup();
    let mut shared = SharedContext::new();
    shared.insert(CharCreateUiState::default());
    let mut screen = Screen::new(char_create_screen);
    screen.sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    for _ in 0..3 {
        app.update();
    }
    app
}

#[test]
fn charcreate_icons_project_original_image_content_without_obsolete_files() {
    let mut app = icon_screen_app();
    let frames: std::collections::HashMap<_, _> = app
        .world_mut()
        .query::<(Entity, &RegistryNode)>()
        .iter(app.world())
        .map(|(entity, node)| (node.0, entity))
        .collect();
    let projected: std::collections::HashMap<_, _> = app
        .world_mut()
        .query::<(&ChildOf, &ImageNode)>()
        .iter(app.world())
        .map(|(parent, image)| (parent.parent(), image.image.clone()))
        .collect();
    let registry = &app.world().resource::<UiState>().registry;
    let images = app.world().resource::<Assets<Image>>();
    let loader = GameBlpLoader;
    let mut missing = Vec::new();
    for &(name, fdid, _) in ICONS {
        let frame_id = registry
            .get_by_name(name)
            .unwrap_or_else(|| panic!("missing authored icon {name}"));
        let entity = frames[&frame_id];
        let Some(actual) = projected.get(&entity).and_then(|handle| images.get(handle)) else {
            missing.push(name);
            continue;
        };
        let path = loader.ensure_texture(fdid).unwrap();
        let expected = loader.load_blp_gpu_image(&path).unwrap();
        assert_eq!(
            actual.size(),
            expected.size(),
            "wrong artwork dimensions: {name}"
        );
        assert_eq!(
            actual.texture_descriptor.format, expected.texture_descriptor.format,
            "{name}"
        );
        assert_eq!(
            actual.data, expected.data,
            "native image content changed: {name}"
        );
        assert!(
            !actual.data.as_ref().unwrap().is_empty(),
            "empty artwork: {name}"
        );
    }
    assert!(
        missing.is_empty(),
        "icons have no native image output: {missing:?}"
    );
}
