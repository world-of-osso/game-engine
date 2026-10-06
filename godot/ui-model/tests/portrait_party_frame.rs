use std::path::PathBuf;

use game_engine_ui_model::portrait_party_frame_component::{
    PortraitPartyFrameState, PortraitPartyMemberView, portrait_party_frame_screen,
};
use ui_toolkit::atlas::{ActiveSkin, resolve_region};
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn members() -> Vec<PortraitPartyMemberView> {
    ["Theron", "Jaina", "Valeera", "Uther"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| PortraitPartyMemberView {
            name: name.into(),
            health_fraction: [0.75, 0.5, 0.25, 0.0][index],
            power_fraction: [0.5, 0.25, 1.0, 0.0][index],
            class_rgb: [
                [0.96, 0.55, 0.73],
                [0.25, 0.78, 0.92],
                [1.0, 0.96, 0.41],
                [0.96, 0.55, 0.73],
            ][index],
            leader: index == 1,
            offline: index == 2,
            dead: index == 3,
            ..PortraitPartyMemberView::named(name)
        })
        .collect()
}

fn render(skin: ActiveSkin) -> FrameRegistry {
    render_state(
        skin,
        PortraitPartyFrameState {
            members: members(),
            ..Default::default()
        },
    )
}

fn render_state(skin: ActiveSkin, state: PortraitPartyFrameState) -> FrameRegistry {
    // Production mirrors the UI thread's skin into each SharedContext.
    let previous_skin = ui_toolkit::atlas::thread_skin();
    ui_toolkit::atlas::set_thread_skin(skin);
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(portrait_party_frame_screen).sync(&shared, &mut registry);
    game_engine_ui_model::portrait_party_frame_component::apply_portrait_party_postsetup(
        &state,
        &mut registry,
    );
    ui_toolkit::atlas::set_thread_skin(previous_skin);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(data) => data.text.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

fn region(
    registry: &FrameRegistry,
    name: &str,
    skin: ActiveSkin,
) -> ui_toolkit::atlas::AtlasRegion {
    let Some(WidgetData::Texture(data)) = &frame(registry, name).widget_data else {
        panic!("{name}");
    };
    let TextureSource::Atlas(atlas) = &data.source else {
        panic!("{name}");
    };
    resolve_region(atlas, skin).expect(atlas)
}

#[test]
fn four_members_draw_state_bars_names_status_and_only_the_leader() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin);
        for (index, view) in members().iter().enumerate() {
            let root = format!("PartyMemberFrame{}", index + 1);
            assert_eq!(text(&registry, &format!("{root}Name")), view.name);
            assert_eq!(
                frame(&registry, &format!("{root}LeaderIcon")).hidden,
                !view.leader
            );
            assert_eq!(
                text(&registry, &format!("{root}HealthBarText")),
                if view.offline {
                    "Offline"
                } else if view.dead {
                    "Dead"
                } else {
                    ""
                }
            );
            let health = frame(&registry, &format!("{root}HealthBarFill"));
            let power = frame(&registry, &format!("{root}ManaBarFill"));
            assert_eq!(
                health.width,
                ui_toolkit::frame::Dimension::Fixed(
                    70.0 * if view.offline {
                        1.0
                    } else {
                        view.health_fraction
                    }
                )
            );
            let power_width = if skin == ActiveSkin::Forever {
                69.0
            } else {
                74.0
            };
            assert_eq!(
                power.width,
                ui_toolkit::frame::Dimension::Fixed(
                    power_width
                        * if view.offline {
                            1.0
                        } else {
                            view.power_fraction
                        }
                )
            );
            assert!(registry.get_by_name(&format!("{root}Portrait")).is_some());
            let Some(WidgetData::FontString(name)) =
                &frame(&registry, &format!("{root}Name")).widget_data
            else {
                panic!("name");
            };
            assert_eq!(
                name.color,
                [view.class_rgb[0], view.class_rgb[1], view.class_rgb[2], 1.0]
            );
        }
        assert!(registry.get_by_name("PartyMemberFrame5").is_none());
    }
}

#[test]
fn portrait_party_style_switch_keeps_compact_default_and_raid_visible() {
    use game_engine_core::ui_layout_data::LayoutSettings;
    use game_engine_ui_model::compact_unit_frame_component::{CompactUnitView, UnitStatus};
    use game_engine_ui_model::group_frames_component::{GroupFramesState, group_frames_screen};
    let bob = CompactUnitView {
        name: "Bob".into(),
        class_rgb: [0.25, 0.78, 0.92],
        health_fraction: Some(1.0),
        power: Some((0.5, [0.0, 0.0, 1.0])),
        role: shared::protocol::GroupRoleSnapshot::None,
        status: UnitStatus::Online,
        in_range: true,
        selected: false,
        ready: None,
        debuffs: vec![],
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let previous_skin = ui_toolkit::atlas::thread_skin();
        ui_toolkit::atlas::set_thread_skin(skin);
        game_engine_ui_model::paths::set_data_root(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let mut shared = SharedContext::new();
        shared.insert(skin);
        let mut state = GroupFramesState {
            party: vec![bob.clone()],
            portrait_party: PortraitPartyFrameState {
                members: members(),
                ..Default::default()
            },
            ..Default::default()
        };
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(group_frames_screen);
        screen.sync(&shared, &mut registry);
        assert!(!frame(&registry, "CompactPartyFrame").hidden);
        assert!(frame(&registry, "PartyFrame").hidden);
        shared.insert(LayoutSettings {
            use_raid_style_party_frames: Some(false),
            ..Default::default()
        });
        screen.sync(&shared, &mut registry);
        assert!(frame(&registry, "CompactPartyFrame").hidden);
        assert!(!frame(&registry, "PartyFrame").hidden);
        assert_eq!(text(&registry, "PartyMemberFrame2Name"), "Jaina");
        state.party.clear();
        state.portrait_party = PortraitPartyFrameState::default();
        state.raid = vec![vec![bob.clone()]];
        shared.insert(state);
        screen.sync(&shared, &mut registry);
        assert!(frame(&registry, "CompactPartyFrame").hidden);
        assert!(frame(&registry, "PartyFrame").hidden);
        assert!(!frame(&registry, "CompactRaidFrameContainer").hidden);
        shared.insert(LayoutSettings::default());
        screen.sync(&shared, &mut registry);
        assert!(!frame(&registry, "CompactRaidFrameContainer").hidden);
        ui_toolkit::atlas::set_thread_skin(previous_skin);
    }
}

#[test]
fn portrait_party_offline_uses_retail_full_desaturated_health_and_dimmed_power() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin);
        let health = frame(&registry, "PartyMemberFrame3HealthBarFill");
        assert_eq!(health.width, ui_toolkit::frame::Dimension::Fixed(70.0));
        let Some(WidgetData::Texture(health)) = &health.widget_data else {
            panic!("health texture")
        };
        assert!(health.desaturated);
        let power = frame(&registry, "PartyMemberFrame3ManaBarFill");
        let Some(WidgetData::Texture(power)) = &power.widget_data else {
            panic!("power texture")
        };
        assert_eq!(power.vertex_color, [0.5, 0.5, 0.5, 1.0]);
        assert!(
            !power.desaturated,
            "UnitFrame.lua:540 only desaturates dead player power"
        );
    }
}

#[test]
fn portrait_party_crown_rect_matches_retail_bottom_to_top_anchor() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin);
        let crown = frame(&registry, "PartyMemberFrame2LeaderIcon");
        let atlas = resolve_region("UI-HUD-UnitFrame-Player-Group-LeaderIcon", skin).unwrap();
        assert_eq!(
            crown.width,
            ui_toolkit::frame::Dimension::Fixed(atlas.width)
        );
        assert_eq!(
            crown.height,
            ui_toolkit::frame::Dimension::Fixed(atlas.height)
        );
        assert_eq!(
            crown.position.left,
            ui_toolkit::layout_values::Val::Px(60.0 - 10.0 - atlas.width / 2.0)
        );
        assert_eq!(
            crown.position.top,
            ui_toolkit::layout_values::Val::Px(6.0 - atlas.height)
        );
    }
}

#[test]
fn forever_party_draws_camelot_character_art_not_retail_party_art() {
    let modern = render(ActiveSkin::Modern);
    let forever = render(ActiveSkin::Forever);
    let party = region(&forever, "PartyMemberFrame1Art", ActiveSkin::Forever);
    let camelot = resolve_region(
        "UI-HUD-UnitFrame-CharacterFrameOnParty-PortraitOn",
        ActiveSkin::Forever,
    )
    .unwrap();
    assert_eq!(party, camelot);
    assert_ne!(
        party,
        region(&modern, "PartyMemberFrame1Art", ActiveSkin::Modern)
    );
    let ordinary_forever =
        resolve_region("UI-HUD-UnitFrame-Party-PortraitOn", ActiveSkin::Forever).unwrap();
    assert_ne!(
        party.source, ordinary_forever.source,
        "conditional character art is not ordinary uipartyframec60"
    );
    assert_ne!(
        ordinary_forever.source,
        resolve_region("UI-HUD-UnitFrame-Party-PortraitOn", ActiveSkin::Modern)
            .unwrap()
            .source
    );
    assert_eq!(
        (party.width, party.height),
        (ordinary_forever.width, ordinary_forever.height)
    );
    assert_eq!(
        party.source,
        resolve_region("UI-HUD-UnitFrame-Player-PortraitOn", ActiveSkin::Forever)
            .unwrap()
            .source
    );
}

#[path = "fixtures/modern_portrait_party.rs"]
mod fixture;

fn dump_tree(registry: &FrameRegistry, id: u64, depth: usize, out: &mut String) {
    use std::fmt::Write;
    let frame = registry.get(id).unwrap();
    writeln!(
        out,
        "{:indent$}{:?} {:?} w={:?} h={:?} pos={:?} hidden={} data={:?}",
        "",
        frame.name,
        frame.widget_type,
        frame.width,
        frame.height,
        frame.position,
        frame.hidden,
        frame.widget_data,
        indent = depth * 2
    )
    .unwrap();
    for child in &frame.children {
        dump_tree(registry, *child, depth + 1, out);
    }
}

fn modern_capture() -> String {
    let registry = render(ActiveSkin::Modern);
    let mut out = String::new();
    dump_tree(
        &registry,
        registry.get_by_name("PartyFrame").unwrap(),
        0,
        &mut out,
    );
    out
}

/// Regeneration captures authored Registry output, not a hand-maintained layout.
#[test]
#[ignore = "fixture regeneration; run with --ignored --nocapture"]
fn capture_modern_portrait_party_fixture() {
    println!("GOLDEN_START\n{}GOLDEN_END", modern_capture());
}

#[test]
fn modern_portrait_party_matches_captured_golden() {
    assert_eq!(modern_capture(), fixture::MODERN_PORTRAIT_PARTY);
}

#[test]
fn guide_roles_and_optional_pets_follow_view_state() {
    use game_engine_ui_model::portrait_party_frame_component::PortraitPartyPetView;
    use shared::protocol::GroupRoleSnapshot;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut views = members();
        for (view, role) in views.iter_mut().zip([
            GroupRoleSnapshot::Tank,
            GroupRoleSnapshot::Healer,
            GroupRoleSnapshot::Damage,
            GroupRoleSnapshot::None,
        ]) {
            view.role = role;
            view.pet = Some(PortraitPartyPetView {
                health_fraction: 0.5,
                dead: false,
            });
        }
        views[1].guide = true;
        let with_pets = render_state(
            skin,
            PortraitPartyFrameState {
                members: views.clone(),
                show_pets: true,
            },
        );
        assert!(frame(&with_pets, "PartyMemberFrame2LeaderIcon").hidden);
        assert!(!frame(&with_pets, "PartyMemberFrame2GuideIcon").hidden);
        for (index, atlas) in [
            "roleicon-tiny-tank",
            "roleicon-tiny-healer",
            "roleicon-tiny-dps",
        ]
        .into_iter()
        .enumerate()
        {
            let name = format!("PartyMemberFrame{}RoleIcon", index + 1);
            assert_eq!(
                region(&with_pets, &name, skin),
                resolve_region(atlas, skin).unwrap()
            );
            assert!(!frame(&with_pets, &name).hidden);
        }
        assert!(with_pets.get_by_name("PartyMemberFrame4RoleIcon").is_none());
        assert!(
            with_pets.get_by_name("PartyMemberFrame3Pet").is_none(),
            "offline pet is hidden"
        );
        assert_eq!(
            frame(&with_pets, "PartyMemberFrame1PetHealthBarFill").width,
            ui_toolkit::frame::Dimension::Fixed(17.75)
        );
        assert!(
            with_pets
                .get_by_name("PartyMemberFrame1PetManaBar")
                .is_none()
        );
        let without_pets = render_state(
            skin,
            PortraitPartyFrameState {
                members: views,
                show_pets: false,
            },
        );
        assert!(without_pets.get_by_name("PartyMemberFrame1Pet").is_none());
        assert_eq!(
            frame(&without_pets, "PartyFrame").height,
            ui_toolkit::frame::Dimension::Fixed(242.0)
        );
        assert_eq!(
            frame(&with_pets, "PartyFrame").height,
            ui_toolkit::frame::Dimension::Fixed(290.0)
        );
    }
}

#[test]
fn portrait_party_hides_empty_and_limits_population_to_four() {
    let registry = render_state(ActiveSkin::Modern, PortraitPartyFrameState::default());
    assert!(frame(&registry, "PartyFrame").hidden);
    let mut views = members();
    views.push(PortraitPartyMemberView::named("Fifth"));
    let registry = render_state(
        ActiveSkin::Modern,
        PortraitPartyFrameState {
            members: views,
            show_pets: false,
        },
    );
    assert!(!frame(&registry, "PartyFrame").hidden);
    assert!(registry.get_by_name("PartyMemberFrame5").is_none());
}

#[test]
fn spec_power_fills_reuse_the_existing_unit_frame_art() {
    use game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_art::power_bar_atlas;
    use shared::components::PowerType;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for power_type in [
            PowerType::LunarPower,
            PowerType::Maelstrom,
            PowerType::Insanity,
            PowerType::Fury,
            PowerType::Pain,
        ] {
            let view = PortraitPartyMemberView {
                power_type,
                power_fraction: 0.5,
                ..PortraitPartyMemberView::named("Spec power")
            };
            let registry = render_state(
                skin,
                PortraitPartyFrameState {
                    members: vec![view],
                    show_pets: false,
                },
            );
            assert_eq!(
                region(&registry, "PartyMemberFrame1ManaBarFill", skin),
                resolve_region(power_bar_atlas(power_type).unwrap(), skin).unwrap()
            );
        }
    }
}

#[test]
fn forever_mana_uses_the_player_fill_named_by_the_camelot_override() {
    let registry = render_state(
        ActiveSkin::Forever,
        PortraitPartyFrameState {
            members: vec![PortraitPartyMemberView {
                power_fraction: 1.0,
                ..PortraitPartyMemberView::named("Mana")
            }],
            show_pets: false,
        },
    );
    assert_eq!(
        drawn_crop(
            &registry,
            "PartyMemberFrame1ManaBarFill",
            ActiveSkin::Forever
        ),
        source_crop("ui-hud-unitframe-player-portraiton-bar-mana")
    );
}

fn source_row(file: &str, column: &str, value: &str) -> std::collections::HashMap<String, String> {
    use game_engine_ui_model::csv_util::parse_csv_records;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/db2/1.60.1.69913")
        .join(file);
    let records = parse_csv_records(&std::fs::read_to_string(path).unwrap());
    let headers = &records[0];
    let index = headers.iter().position(|h| h == column).unwrap();
    let row = records.iter().skip(1).find(|r| r[index] == value).unwrap();
    headers.iter().cloned().zip(row.iter().cloned()).collect()
}

fn source_crop(name: &str) -> (u32, [f32; 4]) {
    let member = source_row("UiTextureAtlasMember.csv", "CommittedName", name);
    let atlas = source_row("UiTextureAtlas.csv", "ID", &member["UiTextureAtlasID"]);
    let w = atlas["AtlasWidth"].parse::<f32>().unwrap();
    let h = atlas["AtlasHeight"].parse::<f32>().unwrap();
    let number = |key: &str| member[key].parse::<f32>().unwrap();
    (
        atlas["FileDataID"].parse().unwrap(),
        [
            number("CommittedLeft") / w,
            number("CommittedRight") / w,
            number("CommittedTop") / h,
            number("CommittedBottom") / h,
        ],
    )
}

fn drawn_crop(registry: &FrameRegistry, name: &str, skin: ActiveSkin) -> (u32, [f32; 4]) {
    use ui_toolkit::atlas::AtlasSource;
    let Some(WidgetData::Texture(data)) = &frame(registry, name).widget_data else {
        panic!("{name}");
    };
    let (fdid, bounds) = match &data.source {
        TextureSource::FileDataId(fdid) => (*fdid, [0.0, 1.0, 0.0, 1.0]),
        TextureSource::File(path) => {
            assert_eq!(path, "data/forever-1.60.1.70205/textures/4631591.blp");
            (4_631_591, [0.0, 1.0, 0.0, 1.0])
        }
        TextureSource::Atlas(atlas) => {
            let region = resolve_region(atlas, skin).expect(atlas);
            let AtlasSource::FileDataId(fdid) = region.source else {
                panic!("{atlas}");
            };
            (fdid, [region.left, region.right, region.top, region.bottom])
        }
        other => panic!("{other:?}"),
    };
    let [l, r, t, b] = bounds;
    let [u0, u1, v0, v1] = data.tex_coords;
    (
        fdid,
        [
            l + (r - l) * u0,
            l + (r - l) * u1,
            t + (b - t) * v0,
            t + (b - t) * v1,
        ],
    )
}

#[test]
fn portrait_party_binds_product_bytes_without_replacing_retail() {
    for skin in [ActiveSkin::Forever, ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin);
        let Some(WidgetData::Texture(data)) =
            &frame(&registry, "PartyMemberFrame1HealthBarFill").widget_data
        else {
            panic!("health fill");
        };
        match skin {
            ActiveSkin::Forever => assert_eq!(
                data.source,
                TextureSource::File("data/forever-1.60.1.70205/textures/4631591.blp".into())
            ),
            ActiveSkin::Modern => assert_eq!(
                data.source,
                TextureSource::Atlas("UI-HUD-UnitFrame-Party-PortraitOn-Bar-Health".into())
            ),
        }
    }
}

#[test]
fn forever_character_bar_crops_match_the_lua_named_source_records() {
    use shared::components::PowerType;
    for (power_type, power_name) in [
        (PowerType::Rage, "rage"),
        (PowerType::Focus, "focus"),
        (PowerType::Energy, "energy"),
        (PowerType::RunicPower, "runicpower"),
    ] {
        let view = PortraitPartyMemberView {
            power_type,
            health_fraction: 1.0,
            power_fraction: 1.0,
            ..PortraitPartyMemberView::named("Source art")
        };
        let registry = render_state(
            ActiveSkin::Forever,
            PortraitPartyFrameState {
                members: vec![view],
                show_pets: false,
            },
        );
        assert_eq!(
            drawn_crop(
                &registry,
                "PartyMemberFrame1HealthBarFill",
                ActiveSkin::Forever
            ),
            source_crop("ui-hud-unitframe-characterframeonparty-portraiton-bar-health")
        );
        assert_eq!(
            drawn_crop(
                &registry,
                "PartyMemberFrame1ManaBarFill",
                ActiveSkin::Forever
            ),
            source_crop(&format!(
                "ui-hud-unitframe-characterframeonparty-portraiton-bar-{power_name}"
            ))
        );
    }
}
