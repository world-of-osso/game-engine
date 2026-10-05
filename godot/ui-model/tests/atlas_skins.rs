//! Atlas names resolve from the Retail and Forever `UiTextureAtlas*` DB2 exports under
//! the active skin.

use std::path::PathBuf;

#[path = "fixtures/old_atlas_regions.rs"]
mod old_atlas_regions;

use ui_toolkit::atlas::{ActiveSkin, AtlasRegion, AtlasSource, resolve_region};

fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn rect(region: &AtlasRegion, sheet: [f32; 2]) -> [f32; 4] {
    [
        region.left * sheet[0],
        region.top * sheet[1],
        region.right * sheet[0],
        region.bottom * sheet[1],
    ]
}

/// `UI-HUD-UnitFrame-Player-PortraitOn` (element 19918): Retail member 16110 on
/// `uiunitframe.blp` (UiTextureAtlas 2060, 1024x512), Forever set-1 member 38478 on
/// `uiunitframec60.blp` (UiTextureAtlas 3960, 256x512). Both 1x, 198x71.
#[test]
fn player_portrait_on_resolves_to_retail_art_under_modern_and_c60_art_under_forever() {
    load_atlas_tables();
    let name = "UI-HUD-UnitFrame-Player-PortraitOn";

    let modern = resolve_region(name, ActiveSkin::Modern).expect("Modern region");
    assert_eq!(modern.source, AtlasSource::FileDataId(4_631_591));
    assert_eq!(rect(&modern, [1024.0, 512.0]), [1.0, 87.0, 199.0, 158.0]);
    assert_eq!((modern.width, modern.height), (198.0, 71.0));

    let forever = resolve_region(name, ActiveSkin::Forever).expect("Forever region");
    assert_eq!(forever.source, AtlasSource::FileDataId(8_036_204));
    assert_eq!(rect(&forever, [256.0, 512.0]), [1.0, 246.0, 199.0, 317.0]);
    assert_eq!((forever.width, forever.height), (198.0, 71.0));
}

/// Forever has no set-1 member for `common-icon-zoomin`; it draws the Retail set-0 member.
#[test]
fn forever_draws_retail_member_for_names_it_does_not_reskin() {
    load_atlas_tables();
    let name = "common-icon-zoomin";
    assert_eq!(
        resolve_region(name, ActiveSkin::Forever),
        resolve_region(name, ActiveSkin::Modern)
    );
    assert!(resolve_region(name, ActiveSkin::Modern).is_some());
}

/// One region of the baked table ui-toolkit had before the DB2 loader
/// (`fixtures/old_atlas_regions.rs`).
struct OldRegion {
    name: String,
    region: AtlasRegion,
    kind: String,
}

fn old_regions() -> Vec<OldRegion> {
    let text = old_atlas_regions::OLD_ATLAS_REGIONS;
    text.lines()
        .skip(1)
        .map(|line| {
            let f: Vec<&str> = line.split(',').collect();
            let source = match f[1].split_once(':').unwrap() {
                ("fdid", id) => AtlasSource::FileDataId(id.parse().unwrap()),
                ("file", path) => AtlasSource::File(Box::leak(path.to_string().into_boxed_str())),
                other => panic!("source {other:?}"),
            };
            let n = |index: usize| f[index].parse::<f32>().unwrap();
            let region = AtlasRegion {
                source,
                left: n(2),
                right: n(3),
                top: n(4),
                bottom: n(5),
                width: n(6),
                height: n(7),
                tiles_horizontally: false,
                tiles_vertically: false,
                nine_slice_edge: (!f[8].is_empty()).then(|| n(8)),
            };
            OldRegion {
                name: f[0].to_string(),
                region,
                kind: f[9].to_string(),
            }
        })
        .collect()
}

/// The legacy tables cited three Retail sheets by a local file path that no longer
/// exists; the DB2 names them by FileDataID.
fn db2_source(old: AtlasSource) -> AtlasSource {
    match old {
        AtlasSource::File(path) if path.ends_with("UIActionBar.BLP") => {
            AtlasSource::FileDataId(4_613_342)
        }
        AtlasSource::File(path) if path.ends_with("CommonDropdown.BLP") => {
            AtlasSource::FileDataId(5_390_329)
        }
        AtlasSource::File(path) if path.ends_with("UICharacterSelectGluesGrayscale.BLP") => {
            AtlasSource::FileDataId(6_870_745)
        }
        source => source,
    }
}

/// Every name of the old baked table resolves under Modern:
/// - `exact`: project art and the DB2-joined crops, bit-identical;
/// - `rounded`: hand-copied crops whose UVs were rounded to 6 decimals resolve to the
///   same DB2 member (same sheet, same size, UVs within that rounding);
/// - `corrected`: three char-create crops the old table pointed at the wrong pixels of
///   FDID 1253496 now use their 12.1 DB2 member.
#[test]
fn every_old_baked_atlas_name_resolves_the_same_under_modern() {
    load_atlas_tables();
    let old = old_regions();
    assert_eq!(old.len(), 118);
    for OldRegion { name, region, kind } in &old {
        let new = resolve_region(name, ActiveSkin::Modern)
            .unwrap_or_else(|| panic!("{name} does not resolve"));
        match kind.as_str() {
            "exact" => assert_eq!(new, *region, "{name}"),
            "rounded" => {
                assert_eq!(new.source, db2_source(region.source), "{name}");
                assert_eq!(
                    (new.width, new.height),
                    (region.width, region.height),
                    "{name}"
                );
                for (new, old) in [
                    (new.left, region.left),
                    (new.right, region.right),
                    (new.top, region.top),
                    (new.bottom, region.bottom),
                ] {
                    assert!((new - old).abs() <= 1e-6, "{name}: {new} vs {old}");
                }
            }
            "corrected" => {
                assert_eq!(new.source, region.source, "{name}");
                assert_ne!(new, *region, "{name}");
            }
            other => panic!("kind {other}"),
        }
    }
    let icon_lock = resolve_region(
        "charactercreate-customize-dropdown-icon-lock",
        ActiveSkin::Modern,
    )
    .unwrap();
    assert_eq!(
        rect(&icon_lock, [2048.0, 2048.0]),
        [2011.0, 213.0, 2041.0, 256.0]
    );
    assert_eq!((icon_lock.width, icon_lock.height), (8.0, 12.0));
}
