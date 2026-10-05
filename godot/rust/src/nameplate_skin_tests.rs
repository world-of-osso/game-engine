//! Nameplate background/fill art by skin: Retail's `uicastingbar` under Modern and
//! Forever's set-1 `uicastingbarc60` members; the soft glow is skin-independent. Forever
//! plates carry Camelot's level frame (Blizzard_NamePlates/Camelot/
//! Blizzard_NamePlateLevelFrame.xml) where Modern plates have none.

use std::path::PathBuf;

use game_engine_core::warband_scene_data::AtlasArt;
use ui_toolkit::atlas::resolve_region;

use super::*;
use crate::nameplate_cast_bar::{CastCrops, cast_crops};

fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

/// A `[left, right, top, bottom]` pixel crop of a `sheet`-sized texture.
fn crop(fdid: u32, sheet: [f32; 2], [left, right, top, bottom]: [f32; 4]) -> AtlasArt {
    AtlasArt {
        fdid,
        tex_coords: [
            left / sheet[0],
            right / sheet[0],
            top / sheet[1],
            bottom / sheet[1],
        ],
    }
}

/// `nameplates-InterruptShield` has no set-1 member: both skins draw UiTextureAtlasMember
/// 5409 on `1300837`.
fn shield() -> AtlasArt {
    crop(1_300_837, [256.0, 256.0], [63.0, 77.0, 194.0, 210.0])
}

/// `uicastingbar.blp` (4505182, 512×256): the background and fill are the crops the client
/// hard-coded before (`[57, 85, 209, 11]` and `[268, 124, 209, 11]`).
#[test]
fn modern_cast_bar_draws_the_retail_uicastingbar_crops() {
    load_atlas_tables();
    let sheet = [512.0, 256.0];
    assert_eq!(
        cast_crops(ActiveSkin::Modern).unwrap(),
        CastCrops {
            background: crop(4_505_182, sheet, [57.0, 266.0, 85.0, 96.0]),
            fill: crop(4_505_182, sheet, [268.0, 477.0, 124.0, 135.0]),
            shield: shield(),
        }
    );
}

/// `uicastingbarc60.blp` (8311977, 512×256), UiTextureAtlasMember 42438, 42447 and 42460
/// of atlas 4246 (set 1): a 211×13 background and the 209×11 fill.
#[test]
fn forever_cast_bar_draws_the_c60_sheet() {
    load_atlas_tables();
    let sheet = [512.0, 256.0];
    assert_eq!(
        cast_crops(ActiveSkin::Forever).unwrap(),
        CastCrops {
            background: crop(8_311_977, sheet, [271.0, 482.0, 100.0, 113.0]),
            fill: crop(8_311_977, sheet, [268.0, 477.0, 157.0, 168.0]),
            shield: shield(),
        }
    );
}

/// The glow stays on the moving fill edge without extending above or below the track.
#[test]
fn nameplate_glow_stays_inside_both_cast_presets_at_the_fill_edge() {
    use crate::nameplate_cast_bar::cast_layout;
    use game_engine_core::nameplate_style_data::NameplateBarThickness;

    for preset in [NameplateBarThickness::Thin, NameplateBarThickness::Thick] {
        let style = NameplateStyle::from_presets(preset, preset);
        for fraction in [0.0, 0.25, 0.5, 1.0] {
            let layout = cast_layout(&style, fraction, (-94.0, 94.0));
            assert!(layout.spark.position.y >= layout.background.position.y);
            assert!(layout.spark.end().y <= layout.background.end().y);
            assert!((layout.spark.center().x - layout.fill.end().x).abs() < 1e-5);
        }
    }
}

/// The Camelot frame's three atlases, members 39470-39472 of atlas 4083 (8165538, 64×64).
#[test]
fn forever_plate_level_frame_draws_the_camelot_level_indicator_atlases() {
    load_atlas_tables();
    let atlases = level_frame_atlases(ActiveSkin::Forever).expect("Forever has a level frame");
    assert_eq!(
        atlases,
        LevelAtlases {
            icon: "ui-hud-nameplates-levelindicator",
            selected: "ui-hud-nameplates-levelindicator-selected",
            skull: "ui-hud-nameplates-levelindicator-skull",
        }
    );
    let sheet = [64.0, 64.0];
    let art = |name| skin_art(name, ActiveSkin::Forever).unwrap();
    assert_eq!(
        art(atlases.icon),
        crop(8_165_538, sheet, [1.0, 16.0, 45.0, 60.0])
    );
    assert_eq!(
        art(atlases.selected),
        crop(8_165_538, sheet, [1.0, 19.0, 25.0, 43.0])
    );
    assert_eq!(
        art(atlases.skull),
        crop(8_165_538, sheet, [21.0, 47.0, 25.0, 51.0])
    );
}

/// On the default 188×20 plate (-94..94) the 28×23 frame takes the right 28 px: the health
/// bars end at 66 where the frame begins, the border grows it by 1 and 2 px a side, and the
/// skull is a 23×23 square on its centre (80, 0). The Thin plate's frame is 16 high.
#[test]
fn forever_level_frame_hangs_on_the_shortened_health_bars_right_end() {
    let style = NameplateStyle::default();
    assert_eq!(
        level_layout(&style),
        LevelLayout {
            frame: Rect2::new(Vector2::new(66.0, -11.5), Vector2::new(28.0, 23.0)),
            selected: Rect2::new(Vector2::new(65.0, -13.5), Vector2::new(30.0, 27.0)),
            skull: Rect2::new(Vector2::new(68.5, -11.5), Vector2::new(23.0, 23.0)),
        }
    );
    let plate = plate_layout(&style, 1.0, 28.0);
    // 160×20 body centred on -14: the fill spans -94..66; the Thick frame adds 10×4.
    assert_eq!(plate.fill.position, Vector2::new(-94.0, -9.5));
    assert_eq!(plate.fill.size, Vector2::new(160.0, 19.0));
    assert_eq!(plate.frame.position, Vector2::new(-98.0, -12.5));
    assert_eq!(plate.frame.size, Vector2::new(170.0, 24.0));
    assert_eq!(plate.fill.end().x, level_layout(&style).frame.position.x);
    // The texts keep to the shortened body: the health text ends 2px left of the frame.
    assert_eq!(
        plate.text,
        PlateText::Inside {
            name_left: Vector2::new(-92.0, 0.0),
            health_right: Vector2::new(64.0, 0.0),
        }
    );

    let thin = NameplateStyle {
        health_height: THIN_HEALTH_HEIGHT,
        ..style
    };
    assert_eq!(
        level_layout(&thin).frame,
        Rect2::new(Vector2::new(66.0, -8.0), Vector2::new(28.0, 16.0))
    );
    assert_eq!(
        level_layout(&thin).skull,
        Rect2::new(Vector2::new(72.0, -8.0), Vector2::new(16.0, 16.0))
    );
}

/// Retail 12.1 has no `ui-hud-nameplates-levelindicator` atlas and Mainline plates no
/// level frame: the health bars keep the whole 188 px.
#[test]
fn modern_plate_has_no_level_frame() {
    load_atlas_tables();
    assert_eq!(level_frame_atlases(ActiveSkin::Modern), None);
    assert_eq!(
        resolve_region("ui-hud-nameplates-levelindicator", ActiveSkin::Modern),
        None
    );
    let plate = plate_layout(&NameplateStyle::default(), 1.0, 0.0);
    assert_eq!(plate.fill.position, Vector2::new(-94.0, -9.5));
    assert_eq!(plate.fill.size, Vector2::new(188.0, 19.0));
    assert_eq!(plate.frame.position, Vector2::new(-98.0, -12.5));
    assert_eq!(plate.frame.size, Vector2::new(198.0, 24.0));
    assert_eq!(
        plate.text,
        PlateText::Inside {
            name_left: Vector2::new(-92.0, 0.0),
            health_right: Vector2::new(92.0, 0.0),
        }
    );
}
