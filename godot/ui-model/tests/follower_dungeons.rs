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
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(follower_screen).sync(&shared, &mut registry);
    registry
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
