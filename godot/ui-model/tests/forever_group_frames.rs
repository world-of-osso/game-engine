//! Compact party/raid frames are Blizzard's shared `CompactUnitFrame` under both skins
//! (Forever `Blizzard_UnitFrame.toc` loads `Shared\\CompactUnitFrame`, no Camelot override).
//! FlareUI only hides the party title (`Core.lua:392`, `Modules/Tweaks.lua:869-874`).

#[path = "fixtures/modern_group_frames.rs"]
mod fixture;

use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::buff_data::DebuffType;
use game_engine_ui_model::compact_unit_frame_component::{
    CompactDebuffView, CompactUnitView, UnitStatus,
};
use game_engine_ui_model::group_frames_component::{GroupFramesState, group_frames_screen};
use game_engine_ui_model::group_state::ReadyMark;
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Frame, WidgetData, WidgetType};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

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
        raid: vec![members()[..2].to_vec(), vec![], members()[2..].to_vec()],
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
fn modern_party_and_raid_trees_are_unchanged() {
    assert_eq!(trees(ActiveSkin::Modern), fixture::MODERN_GROUP_TREES);
}

/// FlareUI's `hidePartyTitle` default hides `CompactPartyFrameTitle` and nothing else:
/// raid group titles, members, art and positions are Blizzard's.
#[test]
fn forever_differs_from_modern_only_by_the_hidden_party_title() {
    let modern = trees(ActiveSkin::Modern);
    let forever = trees(ActiveSkin::Forever);
    let changed: Vec<(&str, &str)> = modern
        .lines()
        .zip(forever.lines())
        .filter(|(m, f)| m != f)
        .collect();
    assert_eq!(modern.lines().count(), forever.lines().count());
    assert_eq!(changed.len(), 1, "{changed:#?}");
    let (was, now) = changed[0];
    assert_eq!(
        now.replace("hidden=true", "hidden=false"),
        was,
        "only the visibility changes"
    );
    for (skin, hidden) in [(ActiveSkin::Modern, false), (ActiveSkin::Forever, true)] {
        let registry = group_frames(skin);
        assert_eq!(frame(&registry, "CompactPartyFrameTitle").hidden, hidden);
        assert!(!frame(&registry, "CompactRaidGroup1Title").hidden);
        assert!(!frame(&registry, "CompactRaidGroup3Title").hidden);
    }
}

fn art(registry: &FrameRegistry, name: &str) -> String {
    let Some(WidgetData::Texture(texture)) = &frame(registry, name).widget_data else {
        panic!("{name} is not a texture");
    };
    format!("{:?}", texture.source)
}

/// Members 1-3 are tank, healer and damage with ready, waiting and not-ready marks;
/// members 4 and 5 have no ready mark and member 4 no role.
#[test]
fn role_and_ready_indicators_follow_member_state_under_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = group_frames(skin);
        let member = |n: usize| format!("CompactPartyFrameMember{n}");
        let shown = |suffix: &str| -> Vec<bool> {
            (1..=5)
                .map(|n| !frame(&registry, &format!("{}{suffix}", member(n))).hidden)
                .collect()
        };
        assert_eq!(
            shown("RoleIcon"),
            [true, true, true, false, true],
            "{skin:?}"
        );
        assert_eq!(
            shown("ReadyCheckIcon"),
            [true, true, true, false, false],
            "{skin:?}"
        );
        assert_eq!(shown("StatusText"), [false, false, false, true, true]);
        assert_eq!(
            shown("SelectionHighlight"),
            [true, false, false, false, false]
        );
        for suffix in ["RoleIcon", "ReadyCheckIcon"] {
            let drawn: Vec<String> = (1..=3)
                .map(|n| art(&registry, &format!("{}{suffix}", member(n))))
                .collect();
            assert!(
                drawn[0] != drawn[1] && drawn[1] != drawn[2] && drawn[0] != drawn[2],
                "{suffix} states share art under {skin:?}: {drawn:?}"
            );
        }
        // Raid group 3 holds members 3-5 of the same roster.
        assert!(!frame(&registry, "CompactRaidGroup3Member1ReadyCheckIcon").hidden);
        assert!(frame(&registry, "CompactRaidGroup3Member2RoleIcon").hidden);
    }
}

/// A member's name and status text paint after its background and bar fills: z is strata,
/// frame level and draw layer (godot/rust/src/ui/projection.rs), ties in tree order.
#[test]
fn member_texts_paint_over_the_bars_under_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = group_frames(skin);
        for root in ["CompactPartyFrameMember5", "CompactRaidGroup3Member2"] {
            let member = frame(&registry, root);
            let mut order: Vec<&Frame> = member
                .children
                .iter()
                .map(|id| registry.get(*id).unwrap())
                .collect();
            order.sort_by_key(|f| {
                i32::from(f.strata as u8) * 100 + f.frame_level + i32::from(f.draw_layer as u8)
            });
            let at = |pick: &dyn Fn(&Frame) -> bool| -> Vec<usize> {
                (0..order.len()).filter(|&i| pick(order[i])).collect()
            };
            let name = |f: &Frame| f.name.clone().unwrap_or_default();
            let bars = at(&|f| {
                ["Background", "HealthBar", "PowerBar"]
                    .iter()
                    .any(|suffix| name(f) == format!("{root}{suffix}"))
            });
            let texts = at(&|f| f.widget_type == WidgetType::FontString);
            assert_eq!((bars.len(), texts.len()), (3, 2), "{root}");
            assert!(bars.last() < texts.first(), "{root} under {skin:?}");
        }
    }
}
