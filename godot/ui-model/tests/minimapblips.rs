use game_engine_core::minimap_data::MinimapView;
use game_engine_ui_model::group_state::GroupState;
use game_engine_ui_model::minimap::{
    BlipKind, MinimapClusterState, apply_minimap_postsetup, cluster_style, group_minimap_blips,
    minimap_cluster_screen, target_minimap_blip,
};
use shared::components::Position;
use shared::death::DeathState;
use shared::protocol::{GroupMemberSnapshot, GroupMemberState, GroupRoleSnapshot};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

fn group(position: [f32; 2]) -> GroupState {
    let member = |name: &str, id| GroupMemberSnapshot {
        character_id: id,
        name: name.into(),
        role: GroupRoleSnapshot::None,
        is_leader: id == 1,
        online: true,
        subgroup: 1,
        class: 8,
        level: 10,
        entity: None,
        portrait: Default::default(),
    };
    let live = |name: &str, [x, z]: [f32; 2]| GroupMemberState {
        name: name.into(),
        health: 100,
        max_health: 100,
        power: None,
        death: DeathState::Alive,
        position: Position { x, y: 0.0, z },
        debuffs: vec![],
    };
    GroupState {
        members: vec![member("Blipsone", 1), member("Blipstwo", 2)],
        live: [live("Blipsone", [100.0, 200.0]), live("Blipstwo", position)]
            .into_iter()
            .map(|state| (state.name.clone(), state))
            .collect(),
        ..Default::default()
    }
}

fn build(skin: ActiveSkin, state: MinimapClusterState) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
    apply_minimap_postsetup(&state, &mut registry);
    registry
}

fn texture<'a>(registry: &'a FrameRegistry, name: &str) -> &'a TextureData {
    let frame = registry
        .get(registry.get_by_name(name).expect("visible blip"))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("blip texture")
    };
    texture
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "{actual} != {expected}"
    );
}

#[test]
fn minimapblips_member_northeast_offset_rotation_and_class_colour_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let style = cluster_style(skin);
        let view = MinimapView::new([100.0, 200.0], 0).masked(style.mask);
        let blips = group_minimap_blips(
            &group([150.0, 250.0]),
            Some("Blipsone"),
            &view,
            style.map_size,
        );
        assert_eq!(blips.len(), 1, "local member excluded");
        near(blips[0].offset[0], 50.0 / view.diameter);
        near(blips[0].offset[1], -50.0 / view.diameter);
        let registry = build(
            skin,
            MinimapClusterState {
                blips,
                ..Default::default()
            },
        );
        let art = texture(&registry, "MinimapMember2");
        assert_eq!(art.source, TextureSource::FileDataId(1_121_272));
        assert_eq!(art.vertex_color, [0.25, 0.78, 0.92, 1.0]);
        assert_eq!(
            art.rotation, 0.0,
            "north-up dot does not rotate with facing"
        );
        assert_eq!(
            art.tex_coords,
            [
                525.0 / 1024.0,
                557.0 / 1024.0,
                628.0 / 1024.0,
                660.0 / 1024.0
            ]
        );
        let frame = registry
            .get(registry.get_by_name("MinimapMember2").unwrap())
            .unwrap();
        let Dimension::Fixed(width) = frame.width else {
            panic!("fixed blip")
        };
        near(width, 16.0);
        let (Val::Px(left), Val::Px(top)) = (frame.position.left, frame.position.top) else {
            panic!("absolute blip")
        };
        near(
            left,
            style.map_origin[0] + style.map_size * (0.5 + 50.0 / view.diameter) - 8.0,
        );
        near(
            top,
            style.map_origin[1] + style.map_size * (0.5 - 50.0 / view.diameter) - 8.0,
        );
    }
}

#[test]
fn minimapblips_out_of_range_member_clamps_and_rotates_both_masks() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let style = cluster_style(skin);
        let view = MinimapView::new([100.0, 200.0], 0).masked(style.mask);
        let blips = group_minimap_blips(
            &group([100.0, 1200.0]),
            Some("Blipsone"),
            &view,
            style.map_size,
        );
        near(blips[0].offset[0], 0.5 - 8.0 / style.map_size);
        near(blips[0].offset[1], 0.0);
        let registry = build(
            skin,
            MinimapClusterState {
                blips,
                ..Default::default()
            },
        );
        let art = texture(&registry, "MinimapMember2");
        // Retail directional art keeps its authored green centre and yellow/white rim.
        assert!(!art.desaturated);
        assert_eq!(art.vertex_color, [1.0; 4]);
        // Registry angles are counter-clockwise; the Godot projection negates them.
        near(art.rotation, -std::f32::consts::FRAC_PI_2);
        assert_eq!(
            art.tex_coords,
            [
                695.0 / 1024.0,
                727.0 / 1024.0,
                594.0 / 1024.0,
                626.0 / 1024.0
            ]
        );
        let diagonal = group_minimap_blips(
            &group([1100.0, 1200.0]),
            Some("Blipsone"),
            &view,
            style.map_size,
        );
        let edge = 0.5 - 8.0 / style.map_size;
        let component = if skin == ActiveSkin::Modern {
            edge / 2.0_f32.sqrt()
        } else {
            // The north rim of Forever's visible map starts below its 22-unit header.
            edge - 22.0 / style.map_size
        };
        near(diagonal[0].offset[0], component);
        near(diagonal[0].offset[1], -component);
        let registry = build(
            skin,
            MinimapClusterState {
                blips: diagonal,
                ..Default::default()
            },
        );
        near(
            texture(&registry, "MinimapMember2").rotation,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}

#[test]
fn minimapblips_forever_north_edge_arrow_clears_opaque_header() {
    let style = cluster_style(ActiveSkin::Forever);
    let view = MinimapView::new([100.0, 200.0], 0).masked(style.mask);
    let blips = group_minimap_blips(
        &group([1100.0, 200.0]),
        Some("Blipsone"),
        &view,
        style.map_size,
    );
    let registry = build(
        ActiveSkin::Forever,
        MinimapClusterState {
            blips,
            ..Default::default()
        },
    );
    let frame = registry
        .get(registry.get_by_name("MinimapMember2").unwrap())
        .unwrap();
    let Val::Px(top) = frame.position.top else {
        panic!("absolute member")
    };
    near(top, 30.0);
    assert_eq!(texture(&registry, "MinimapMember2").rotation, 0.0);
}

#[test]
fn minimapblips_target_indicator_and_clear_outside_mask() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let style = cluster_style(skin);
        let view = MinimapView::new([100.0, 200.0], 0).masked(style.mask);
        let blip = target_minimap_blip(55, [50.0, 200.0], &view).expect("nearby target");
        assert_eq!(blip.kind, BlipKind::Target);
        near(blip.offset[0], 0.0);
        near(blip.offset[1], 50.0 / view.diameter);
        let registry = build(
            skin,
            MinimapClusterState {
                blips: vec![blip],
                ..Default::default()
            },
        );
        let art = texture(&registry, "MinimapTarget55");
        assert_eq!(art.vertex_color, [1.0; 4]);
        assert_eq!(
            art.tex_coords,
            [
                627.0 / 1024.0,
                659.0 / 1024.0,
                764.0 / 1024.0,
                796.0 / 1024.0
            ]
        );
        assert!(target_minimap_blip(55, [50.0, 1200.0], &view).is_none());
        let cleared = build(skin, MinimapClusterState::default());
        assert!(cleared.get_by_name("MinimapTarget55").is_none());
    }
}

#[test]
fn minimapblips_raid_dot_and_offline_missing_left_members() {
    let style = cluster_style(ActiveSkin::Modern);
    let view = MinimapView::new([100.0, 200.0], 0);
    let mut state = group([150.0, 250.0]);
    state.is_raid = true;
    state.members[1].class = 2;
    let blips = group_minimap_blips(&state, Some("Blipsone"), &view, style.map_size);
    let registry = build(
        ActiveSkin::Modern,
        MinimapClusterState {
            blips,
            ..Default::default()
        },
    );
    let art = texture(&registry, "MinimapMember2");
    assert_eq!(art.vertex_color, [0.96, 0.55, 0.73, 1.0]);
    assert_eq!(
        art.tex_coords,
        [
            525.0 / 1024.0,
            557.0 / 1024.0,
            662.0 / 1024.0,
            694.0 / 1024.0
        ]
    );
    state.members[1].online = false;
    assert!(group_minimap_blips(&state, Some("Blipsone"), &view, style.map_size).is_empty());
    state.members[1].online = true;
    state.live.remove("Blipstwo");
    assert!(group_minimap_blips(&state, Some("Blipsone"), &view, style.map_size).is_empty());
    state.members.clear();
    assert!(group_minimap_blips(&state, Some("Blipsone"), &view, style.map_size).is_empty());
}

#[test]
fn minimapblips_quest_objective_displays_its_number_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(
            skin,
            MinimapClusterState {
                blips: vec![game_engine_ui_model::minimap::MinimapBlip {
                    unit: 28766,
                    kind: BlipKind::QuestObjective { number: 1 },
                    offset: [0.1, -0.1],
                }],
                ..Default::default()
            },
        );
        let frame = registry
            .get(
                registry
                    .get_by_name("MinimapQuestObjective28766Number")
                    .unwrap(),
            )
            .unwrap();
        let displayed = match frame.widget_data.as_ref() {
            Some(WidgetData::FontString(text)) => Some(text.text.as_str()),
            _ => None,
        };
        assert_eq!(
            displayed,
            Some("1"),
            "objective icon must contain visible number text"
        );
    }
}
