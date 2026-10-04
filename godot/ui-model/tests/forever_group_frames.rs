//! Compact party/raid frames keep Blizzard's shared art under both skins.
//! The c60 party portrait members belong to PartyMemberFrameTemplate, not these frames.

use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::buff_data::DebuffType;
use game_engine_ui_model::compact_unit_frame_component::{
    CompactDebuffView, CompactUnitView, UnitStatus,
};
use game_engine_ui_model::group_frames_component::{GroupFramesState, group_frames_screen};
use game_engine_ui_model::group_state::ReadyMark;
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn members() -> Vec<CompactUnitView> {
    let roles = [
        GroupRoleSnapshot::Tank,
        GroupRoleSnapshot::Healer,
        GroupRoleSnapshot::Damage,
        GroupRoleSnapshot::None,
        GroupRoleSnapshot::Damage,
    ];
    roles
        .into_iter()
        .enumerate()
        .map(|(index, role)| CompactUnitView {
            name: format!("Member{}", index + 1),
            class_rgb: [0.2, 0.4, 0.8],
            health_fraction: Some(0.75),
            power: Some((0.5, [0.0, 0.0, 1.0])),
            role,
            status: match index {
                3 => UnitStatus::Offline,
                4 => UnitStatus::Dead,
                _ => UnitStatus::Online,
            },
            in_range: index != 3,
            selected: index == 0,
            ready: [
                Some(ReadyMark::Ready),
                Some(ReadyMark::Waiting),
                Some(ReadyMark::NotReady),
                None,
                None,
            ][index],
            debuffs: vec![CompactDebuffView {
                icon_fdid: 136_116,
                dispel: DebuffType::Magic,
            }],
        })
        .collect()
}

fn group_frames(skin: ActiveSkin) -> FrameRegistry {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(GroupFramesState {
        party: members(),
        raid: vec![members(), vec![], members()],
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(group_frames_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn dump_frame(registry: &FrameRegistry, id: u64, depth: usize, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(
        out,
        "{:indent$}{:?} {:?} w={:?} h={:?} pos={:?} {:?} margin={:?} translate={:?} \
         hidden={} alpha={} strata={:?} level={} layer={:?} bg={:?} mouse={} click={:?} data={:?}",
        "",
        f.name,
        f.widget_type,
        f.width,
        f.height,
        f.position,
        f.position_type,
        f.margin,
        f.translation,
        f.hidden,
        f.alpha,
        f.strata,
        f.frame_level,
        f.draw_layer,
        f.background_color,
        f.mouse_enabled,
        f.onclick,
        f.widget_data,
        indent = depth * 2,
    )
    .unwrap();
    for child in &f.children {
        dump_frame(registry, *child, depth + 1, out);
    }
}

fn trees(skin: ActiveSkin) -> String {
    let registry = group_frames(skin);
    let mut out = String::new();
    for root in ["CompactPartyFrame", "CompactRaidFrameContainer"] {
        dump_frame(
            &registry,
            registry.get_by_name(root).expect(root),
            0,
            &mut out,
        );
    }
    out
}

#[test]
fn capture_base_modern_party_and_raid_trees() {
    println!(
        "<<<MODERN_TREES\n{}MODERN_TREES>>>",
        trees(ActiveSkin::Modern)
    );
}

fn assert_region(name: &str, skin: ActiveSkin, fdid: u32, sheet: [f32; 2], rect: [f32; 4]) {
    let region = resolve_region(name, skin).unwrap_or_else(|| panic!("{name} under {skin:?}"));
    assert_eq!(
        region.source,
        AtlasSource::FileDataId(fdid),
        "{name} under {skin:?}"
    );
    assert_eq!(
        [
            region.left * sheet[0],
            region.top * sheet[1],
            region.right * sheet[0],
            region.bottom * sheet[1],
        ],
        rect,
        "{name} under {skin:?}"
    );
}

#[test]
fn compact_party_and_raid_draw_shared_names_and_regions_under_both_skins() {
    let pieces = [
        (
            "Background",
            "raidframe-hp-bg-white",
            7_658_229,
            [32.0, 32.0],
            [0.0, 0.0, 32.0, 32.0],
        ),
        (
            "HealthBar",
            "RaidFrame-Hp-Fill",
            7_539_072,
            [32.0, 32.0],
            [0.0, 0.0, 32.0, 32.0],
        ),
        (
            "PowerBarBackground",
            "_RaidFrame-Resource-Background",
            7_539_067,
            [16.0, 64.0],
            [0.0, 43.0, 16.0, 49.0],
        ),
        (
            "PowerBar",
            "_RaidFrame-Resource-Fill",
            7_539_067,
            [16.0, 64.0],
            [0.0, 51.0, 16.0, 57.0],
        ),
        (
            "SelectionHighlight",
            "RaidFrame-TargetFrame",
            7_526_019,
            [256.0, 128.0],
            [145.0, 1.0, 215.0, 35.0],
        ),
    ];
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = group_frames(skin);
        for root in [
            "CompactPartyFrameMember1",
            "CompactRaidGroup1Member1",
            "CompactRaidGroup3Member1",
        ] {
            for (suffix, name, fdid, sheet, rect) in pieces {
                let f = frame(&registry, &format!("{root}{suffix}"));
                let Some(WidgetData::Texture(texture)) = &f.widget_data else {
                    panic!("{} not a texture", f.name.as_deref().unwrap());
                };
                assert_eq!(texture.source, TextureSource::Atlas(name.into()));
                assert_region(name, skin, fdid, sheet, rect);
            }
        }
    }
}

/// Forever DB2 members 39017/39018 on atlas 4019, vs Retail 17762/17761 on 2087.
/// Blizzard_UnitFrame/Mainline/PartyFrameTemplates.xml:106,111 uses these two names.
#[test]
fn c60_party_portraits_are_distinct_from_compact_frame_art() {
    load_atlas_tables();
    for (name, modern, forever) in [
        (
            "UI-HUD-UnitFrame-Party-PortraitOn",
            [123.0, 57.0, 243.0, 106.0],
            [1.0, 53.0, 121.0, 102.0],
        ),
        (
            "UI-HUD-UnitFrame-Party-PortraitOn-Vehicle",
            [133.0, 1.0, 254.0, 51.0],
            [1.0, 1.0, 122.0, 51.0],
        ),
    ] {
        assert_region(name, ActiveSkin::Modern, 4_681_512, [256.0, 256.0], modern);
        assert_region(
            name,
            ActiveSkin::Forever,
            8_116_745,
            [128.0, 128.0],
            forever,
        );
    }
    assert_eq!(trees(ActiveSkin::Forever), trees(ActiveSkin::Modern));
}
