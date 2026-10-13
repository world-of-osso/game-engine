use game_engine_ui_model::follower_dungeons::{Action, View, follower_screen};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::button::ButtonState,
};

fn catalog() -> FollowerDungeons {
    FollowerDungeons {
        entries: vec![FollowerDungeonEntry {
            id: 2436,
            name: "Ruby Life Pools".into(),
            map_id: 2521,
        }],
    }
}
fn render(view: View, skin: ActiveSkin) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(follower_screen).sync(&shared, &mut registry);
    game_engine_ui_model::follower_dungeons::apply_follower_postsetup(&mut registry);
    registry
}
#[test]
fn follower_group_finder_role_templates_include_solo_leader_and_centered_backgrounds() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(View { visible: true, ..Default::default() }, skin);
        let leader = registry.get(registry.get_by_name("LFDQueueFrameRoleButtonLeader").expect("solo leader remains visible")).unwrap();
        assert_eq!((leader.width.value(), leader.height.value()), (48.0, 48.0));
        assert_eq!(button(&registry, "LFDQueueFrameRoleButtonLeader").0, ButtonState::Disabled);
        assert!(registry.get_by_name("LFDQueueFrameRoleButtonLeaderCheckBox").is_none());
        for role in ["Tank", "Healer", "DPS"] {
            let background = registry.get(registry.get_by_name(&format!("LFDQueueFrameRoleButton{role}Background")).expect("inherited role background")).unwrap();
            assert_eq!((background.width.value(), background.height.value()), (80.0, 80.0));
            assert_eq!(background.draw_layer, ui_toolkit::strata::DrawLayer::Background);
        }
    }
}

#[test]
fn follower_group_finder_leader_selection_tracks_leadership_without_changing_combat_role() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = View { visible: true, ..Default::default() };
        view.action("follower:leader");
        assert!(!view.volunteer_leader);
        view.group_leader = true;
        let registry = render(view.clone(), skin);
        assert_eq!(button(&registry, "LFDQueueFrameRoleButtonLeader").0, ButtonState::Normal);
        assert!(registry.get_by_name("LFDQueueFrameRoleButtonLeaderCheckBox").is_some());
        view.action("follower:leader");
        assert!(view.volunteer_leader);
        assert_eq!(view.role, GroupRoleSnapshot::Damage);
        assert!(render(view.clone(), skin).get_by_name("LFDQueueFrameRoleButtonLeaderCheck").is_some());
        view.pending = true;
        view.action("follower:leader");
        assert!(view.volunteer_leader);
        assert_eq!(button(&render(view, skin), "LFDQueueFrameRoleButtonLeader").0, ButtonState::Disabled);
    }
}

#[test]
fn follower_group_finder_preserves_native_paper_and_disabled_category_art_in_both_skins() {
    use ui_toolkit::widgets::texture::TextureSource;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(
            View {
                visible: true,
                ..Default::default()
            },
            skin,
        );
        let paper = registry
            .get(registry.get_by_name("LFDQueueFrameBackground").unwrap())
            .unwrap();
        assert_eq!((paper.width.value(), paper.height.value()), (512.0, 256.0));
        let Some(WidgetData::Texture(texture)) = &paper.widget_data else {
            panic!("paper texture")
        };
        assert_eq!(texture.source, TextureSource::FileDataId(337_492));
        for index in 1..=3 {
            let name = format!("GroupFinderFrameGroupButton{index}");
            assert_eq!(
                button(&registry, &name).0,
                if index == 1 {
                    ButtonState::Normal
                } else {
                    ButtonState::Disabled
                }
            );
            for part in ["Ring", "Icon"] {
                let frame = registry
                    .get(registry.get_by_name(&format!("{name}{part}")).unwrap())
                    .unwrap();
                let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                    panic!("category texture")
                };
                assert_eq!(texture.desaturated, index != 1);
            }
        }
    }
}

#[test]
fn follower_group_finder_type_menu_is_a_checked_radio_without_invented_heading() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(
            View {
                visible: true,
                type_open: true,
                ..Default::default()
            },
            skin,
        );
        assert!(registry.get_by_name("LFDQueueFrameTypeTitle").is_none());
        assert!(registry.get_by_name("LFDQueueFrameTypeDivider").is_none());
        let menu = registry
            .get(
                registry
                    .get_by_name("LFDQueueFrameTypeDropdownMenu")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(menu.height.value(), 43.0);
        assert!(
            registry
                .get_by_name("LFDQueueFrameTypeFollowerTick")
                .is_some()
        );
        assert_eq!(
            button(&registry, "LFDQueueFrameTypeFollower").1.as_deref(),
            Some("follower:type:follower")
        );
    }
}

fn button(registry: &FrameRegistry, name: &str) -> (ButtonState, Option<String>) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        panic!("{name} not a button")
    };
    (button.state, frame.onclick.clone())
}
#[test]
fn follower_group_finder_both_skins_use_the_server_catalog_and_real_queue_action() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = View {
            visible: true,
            ..Default::default()
        };
        view.receive_catalog(catalog());
        assert_eq!(view.action("follower:select:2436"), None);
        view.action("follower:role:healer");
        let registry = render(view.clone(), skin);
        assert!(registry.get_by_name("PVEFrame").is_some());
        assert_eq!(
            button(&registry, "LFDQueueFrameFindGroupButton")
                .1
                .as_deref(),
            Some("follower:queue")
        );
        assert_eq!(
            button(&registry, "LFDQueueFrameFollowerDungeon2436")
                .1
                .as_deref(),
            Some("follower:select:2436")
        );
        assert_eq!(
            view.action("follower:queue"),
            Some(Action::Queue(QueueForLfg {
                role: GroupRoleSnapshot::Healer,
                dungeon_ids: vec![2436]
            }))
        );
        assert!(view.pending);
        assert_eq!(view.action("follower:queue"), None);
        assert_eq!(
            button(&render(view.clone(), skin), "LFDQueueFrameFindGroupButton").0,
            ButtonState::Disabled
        );
        view.receive(LfgStateUpdate {
            snapshot: None,
            message: None,
            error: Some("already in a group".into()),
        });
        assert!(!view.pending);
        assert_eq!(
            view.state.as_ref().unwrap().error.as_deref(),
            Some("already in a group")
        );
        assert_eq!(
            button(&render(view, skin), "LFDQueueFrameFindGroupButton").0,
            ButtonState::Normal
        );
    }
}
#[test]
fn follower_group_finder_empty_catalog_and_stale_selection_cannot_queue() {
    let mut view = View {
        visible: true,
        ..Default::default()
    };
    assert_eq!(view.action("follower:queue"), None);
    view.receive_catalog(catalog());
    view.action("follower:select:99999");
    assert_eq!(view.selected, None);
    view.action("follower:select:2436");
    assert_eq!(view.selected, Some(2436));
    view.receive_catalog(FollowerDungeons { entries: vec![] });
    assert_eq!(view.selected, None);
    assert_eq!(view.action("follower:queue"), None);
    let registry = render(view, ActiveSkin::Modern);
    assert_eq!(
        button(&registry, "LFDQueueFrameFindGroupButton").0,
        ButtonState::Disabled
    );
    assert!(
        registry
            .get_by_name("LFDQueueFrameFollowerDungeon2436")
            .is_none()
    );
}
#[test]
fn follower_group_finder_authoritative_match_stops_duplicate_queue_and_preserves_roles() {
    let mut view = View {
        visible: true,
        ..Default::default()
    };
    view.receive_catalog(catalog());
    view.action("follower:select:2436");
    view.action("follower:queue");
    let update = LfgStateUpdate {
        snapshot: Some(LfgSnapshot {
            queued: false,
            selected_role: Some(GroupRoleSnapshot::Damage),
            dungeon_ids: vec![2436],
            queue_size: 0,
            average_wait_secs: 0,
            in_demand_roles: vec![],
            role_check: None,
            match_found: Some(LfgMatchFoundSnapshot {
                dungeon_id: 2436,
                dungeon_name: "Ruby Life Pools".into(),
                assigned_role: GroupRoleSnapshot::Damage,
                members: [
                    ("Followproof", GroupRoleSnapshot::Damage),
                    ("Captain Garrick", GroupRoleSnapshot::Tank),
                    ("Crenna Earth-Daughter", GroupRoleSnapshot::Healer),
                    ("Meredy Huntswell", GroupRoleSnapshot::Damage),
                    ("Shuja Grimaxe", GroupRoleSnapshot::Damage),
                ]
                .into_iter()
                .map(|(name, role)| LfgMatchMemberSnapshot {
                    name: name.into(),
                    role,
                })
                .collect(),
            }),
        }),
        message: Some("follower party ready".into()),
        error: None,
    };
    view.receive(update.clone());
    assert!(!view.pending);
    assert_eq!(view.state, Some(update));
    assert_eq!(view.action("follower:queue"), None);
    assert_eq!(
        button(
            &render(view, ActiveSkin::Modern),
            "LFDQueueFrameFindGroupButton"
        )
        .0,
        ButtonState::Disabled
    );
}
