//! Own binary: every test holds SKIN for process-global atlas selection.
use game_engine_core::ui_layout_data::{
    self, LayoutSettings, LayoutSkin, PartyAuraOrganization, PartyFrameSettings, PartySort,
};
use game_engine_ui_model::compact_unit_frame_component::{
    CompactDebuffView, CompactUnitView, PartyAuraView, UnitStatus,
};
use game_engine_ui_model::group_frames_component::{GroupFramesState, group_frames_screen};
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, LayoutSystem};
use game_engine_ui_model::options_menu_data::{
    LayoutSlider, SliderField, apply_layout_action, apply_layout_slider, parse_layout_action,
    parse_slider_action,
};
use game_engine_ui_model::portrait_party_frame_component::{
    PortraitPartyFrameState, PortraitPartyMemberView, PortraitPartyPetView,
};
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::{
    atlas::ActiveSkin,
    frame::{Dimension, Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

static SKIN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn setup(skin: ActiveSkin) {
    ui_toolkit::atlas::set_active_skin(skin);
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn state() -> GroupFramesState {
    let party: Vec<_> = [
        ("Zed", GroupRoleSnapshot::Damage),
        ("Amy", GroupRoleSnapshot::Healer),
        ("Bob", GroupRoleSnapshot::Tank),
    ]
    .into_iter()
    .map(|(name, role)| CompactUnitView {
        name: name.into(),
        role,
        health_fraction: Some(0.5),
        power: Some((0.5, [0.0, 0.0, 1.0])),
        class_rgb: [1.0, 0.5, 0.0],
        status: UnitStatus::Online,
        in_range: true,
        selected: false,
        ready: None,
        debuffs: vec![CompactDebuffView {
            icon_fdid: 136118,
            dispel: game_engine_ui_model::buff_data::DebuffType::Magic,
        }],
    })
    .collect();
    let members = party
        .iter()
        .map(|view| PortraitPartyMemberView {
            role: view.role,
            health_fraction: 0.5,
            power_fraction: 0.5,
            pet: Some(PortraitPartyPetView {
                health_fraction: 0.75,
                dead: false,
            }),
            ..PortraitPartyMemberView::named(&view.name)
        })
        .collect();
    let party_auras = party
        .iter()
        .map(|view| {
            (
                view.name.clone(),
                PartyAuraView {
                    buffs: vec![135987, 136048],
                    defensive: Some(135940),
                },
            )
        })
        .collect();
    GroupFramesState {
        party,
        portrait_party: PortraitPartyFrameState {
            members,
            show_pets: false,
        },
        party_auras,
        ..Default::default()
    }
}

fn render(skin: ActiveSkin, settings: LayoutSettings) -> FrameRegistry {
    setup(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state());
    shared.insert(settings);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(group_frames_screen).sync(&shared, &mut registry);
    registry
}
fn frame<'a>(r: &'a FrameRegistry, name: &str) -> &'a Frame {
    r.get(r.get_by_name(name).expect(name)).unwrap()
}
fn text(r: &FrameRegistry, name: &str) -> String {
    match frame(r, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(text) => text.text.clone(),
        other => panic!("{other:?}"),
    }
}
fn rect(r: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(r, name);
    let pixels = |value| match value {
        ui_toolkit::layout_values::Val::Px(value) => value,
        other => panic!("expected fixed geometry: {other:?}"),
    };
    (
        pixels(f.position.left),
        pixels(f.position.top),
        f.width.value(),
        f.height.value(),
    )
}

#[test]
fn party4_options_select_party_and_disable_compact() {
    let _lock = SKIN.lock().unwrap();
    let mut layout = LayoutOptionsView::default();
    let action =
        parse_layout_action("options_toggle:layout_system:6").expect("Party Frames selectable");
    apply_layout_action(action, &mut layout);
    assert_eq!(layout.system, LayoutSystem::PartyFrames);
    apply_layout_action(
        parse_layout_action("options_toggle:party_compact").unwrap(),
        &mut layout,
    );
    assert_eq!(layout.settings.use_raid_style_party_frames, Some(false));
}

#[test]
fn party4_defaults_keep_both_skins_compact_at_98_by_44() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = render(skin, LayoutSettings::default());
        assert!(!frame(&r, "CompactPartyFrame").hidden);
        assert!(frame(&r, "PartyFrame").hidden);
        assert_eq!(
            frame(&r, "CompactPartyFrameMember1").width,
            Dimension::Fixed(98.0)
        );
        assert_eq!(
            frame(&r, "CompactPartyFrameMember1").height,
            Dimension::Fixed(44.0)
        );
        assert_eq!(text(&r, "CompactPartyFrameMember1Name"), "Zed");
        assert!(frame(&r, "CompactPartyFrameSettingsBorder").hidden);
        assert!(frame(&r, "CompactPartyFrameSettingsBackground").hidden);
        assert_eq!(frame(&r, "CompactPartyFrame").alpha, 1.0);
    }
}

#[test]
fn party4_size_orientation_chrome_opacity_scale_and_pets_render_live_in_both_families() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for compact in [true, false] {
            let mut settings = LayoutSettings {
                use_raid_style_party_frames: Some(compact),
                ..Default::default()
            };
            let baseline = render(skin, settings);
            let (root, member, second, pet) = if compact {
                (
                    "CompactPartyFrame",
                    "CompactPartyFrameMember1",
                    "CompactPartyFrameMember2",
                    "CompactPartyFrameMember1Pet",
                )
            } else {
                (
                    "PartyFrame",
                    "PartyMemberFrame1",
                    "PartyMemberFrame2",
                    "PartyMemberFrame1Pet",
                )
            };
            let before = rect(&baseline, member);
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(state());
            shared.insert(settings);
            let mut r = FrameRegistry::new(1920.0, 1080.0);
            let mut screen = Screen::new(group_frames_screen);
            screen.sync(&shared, &mut r);
            settings.party = PartyFrameSettings {
                width: Some(120),
                height: Some(60),
                horizontal: Some(true),
                background: Some(true),
                border: Some(true),
                opacity: Some(60),
                show_pets: Some(true),
                ..Default::default()
            };
            shared.insert(settings);
            screen.sync(&shared, &mut r);
            let after = rect(&r, member);
            let next = rect(&r, second);
            assert!(after.2 > before.2 && after.3 > before.3);
            assert!(next.0 > after.0);
            assert_eq!(next.1, after.1);
            assert!(!frame(&r, &format!("{root}SettingsBorder")).hidden);
            assert!(!frame(&r, &format!("{root}SettingsBackground")).hidden);
            assert_eq!(frame(&r, root).alpha, 0.6);
            assert!(r.get_by_name(pet).is_some(), "pets appear in active family");
            settings.party.frame_size = Some(150);
            shared.insert(settings);
            screen.sync(&shared, &mut r);
            assert!((rect(&r, member).2 - after.2 * 1.5).abs() < 0.01);
            settings.party.horizontal = Some(false);
            settings.party.show_pets = Some(false);
            shared.insert(settings);
            screen.sync(&shared, &mut r);
            assert!(rect(&r, second).1 > rect(&r, member).1);
            assert!(r.get_by_name(pet).is_none());
        }
    }
}

#[test]
fn party4_role_and_alphabetical_sort_render_names_in_both_families() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for compact in [true, false] {
            for (sort, names) in [
                (PartySort::Group, ["Zed", "Amy", "Bob"]),
                (PartySort::Alphabetical, ["Amy", "Bob", "Zed"]),
                (PartySort::Role, ["Bob", "Amy", "Zed"]),
            ] {
                let settings = LayoutSettings {
                    use_raid_style_party_frames: Some(compact),
                    party: PartyFrameSettings {
                        sort: Some(sort),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let r = render(skin, settings);
                for (index, name) in names.into_iter().enumerate() {
                    let root = if compact {
                        format!("CompactPartyFrameMember{}Name", index + 1)
                    } else {
                        format!("PartyMemberFrame{}Name", index + 1)
                    };
                    assert_eq!(text(&r, &root), name);
                }
            }
        }
    }
}

#[test]
fn party4_aura_organization_and_all_icon_sizes_change_rendered_icons() {
    let _lock = SKIN.lock().unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let baseline = render(skin, Default::default());
        let mut settings = LayoutSettings::default();
        settings.party = PartyFrameSettings {
            debuff_size: Some(200),
            buff_size: Some(150),
            defensive_size: Some(100),
            aura_organization: Some(PartyAuraOrganization::BuffsRight),
            ..Default::default()
        };
        let right = render(skin, settings);
        for (suffix, factor) in [
            ("Debuff1", 2.0),
            ("Buff1", 1.5),
            ("BigDefensive", 100.0 / 75.0),
        ] {
            let name = format!("CompactPartyFrameMember1{suffix}");
            assert!((rect(&right, &name).2 - rect(&baseline, &name).2 * factor).abs() < 0.01);
        }
        assert!(
            rect(&right, "CompactPartyFrameMember1RoleIcon").0
                > rect(&baseline, "CompactPartyFrameMember1RoleIcon").0
        );
        settings.party.aura_organization = Some(PartyAuraOrganization::BuffsTop);
        let top = render(skin, settings);
        assert!(
            rect(&top, "CompactPartyFrameMember1Name").1
                > rect(&right, "CompactPartyFrameMember1Name").1
        );
        assert!(
            rect(&top, "CompactPartyFrameMember1Buff1").1
                < rect(&top, "CompactPartyFrameMember1Debuff1").1
        );
    }
}

#[test]
fn party4_every_control_persists_round_trip_per_character_and_resets() {
    let _lock = SKIN.lock().unwrap();
    for skin in [LayoutSkin::Modern, LayoutSkin::Forever] {
        let path =
            std::env::temp_dir().join(format!("party4-{:?}-{}.ron", skin, std::process::id()));
        let mut layout = LayoutOptionsView {
            skin,
            system: LayoutSystem::PartyFrames,
            ..Default::default()
        };
        for action in [
            "party_compact",
            "party_background",
            "party_horizontal",
            "party_border",
            "party_pets",
            "party_sort:0",
            "party_aura:2",
        ] {
            apply_layout_action(
                parse_layout_action(&format!("options_toggle:{action}")).unwrap(),
                &mut layout,
            );
        }
        for (key, value) in [
            ("party_width", 131.0),
            ("party_height", 61.0),
            ("party_size", 173.0),
            ("party_opacity", 64.0),
            ("party_debuff", 146.0),
            ("party_buff", 136.0),
            ("party_defensive", 83.0),
        ] {
            let Some(SliderField::Layout(slider)) =
                parse_slider_action(&format!("options_slider:{key}"))
            else {
                panic!("slider {key}");
            };
            apply_layout_slider(slider, value, &mut layout);
        }
        ui_layout_data::set_active_layout(
            &path,
            17,
            if skin == LayoutSkin::Modern {
                "Modern"
            } else {
                "Forever"
            },
        )
        .unwrap();
        let saved = ui_layout_data::save_layout_settings(&path, 17, layout.settings).unwrap();
        assert_eq!(ui_layout_data::active_layout(&path, 17).unwrap(), saved);
        assert_eq!(
            ui_layout_data::active_layout(&path, 18).unwrap().settings,
            LayoutSettings::default()
        );
        assert_eq!(saved.settings.party.width, Some(132));
        assert_eq!(saved.settings.party.height, Some(62));
        assert_eq!(saved.settings.party.frame_size, Some(175));
        assert_eq!(saved.settings.party.debuff_size, Some(150));
        assert_eq!(saved.settings.party.buff_size, Some(140));
        assert_eq!(saved.settings.party.defensive_size, Some(85));
        assert_eq!(LayoutSlider::PartyWidth.range().step, 2);
        apply_layout_action(
            parse_layout_action("options_reset_layout_settings").unwrap(),
            &mut layout,
        );
        ui_layout_data::save_layout_settings(&path, 17, layout.settings).unwrap();
        assert_eq!(
            ui_layout_data::active_layout(&path, 17).unwrap().settings,
            LayoutSettings::default()
        );
        std::fs::remove_file(path).unwrap();
    }
}
