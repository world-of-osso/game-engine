use bevy::ecs::system::RunSystemOnce;
use shared::components::{AuraView, Position, PowerEntry};
use shared::loot::LootMode;
use shared::protocol::{
    GroupMemberState, GroupRoleSnapshot, GroupRosterSnapshot, ReadyCheckAnswer,
    ReadyCheckMemberSnapshot, ReadyCheckUpdate,
};

use super::native_layout_support::compute_layout;
use super::*;
use game_engine::group_state::ReadyMark;
use game_engine::ui::screens::group_frames_component::PARTY_FRAME as PARTY_FRAME_NAME;

fn member(name: &str, leader: bool, online: bool, subgroup: u8) -> GroupMemberSnapshot {
    GroupMemberSnapshot {
        name: name.into(),
        role: GroupRoleSnapshot::None,
        is_leader: leader,
        online,
        subgroup,
        // Paladin.
        class: 2,
        level: 30,
        entity: None,
    }
}

fn live(name: &str, health: u32, x: f32) -> GroupMemberState {
    GroupMemberState {
        name: name.into(),
        health,
        max_health: 1000,
        power: Some(PowerEntry {
            power: PowerType::Rage,
            current: 25,
            max: 100,
            partial: 0,
            regen_per_sec: 0.0,
        }),
        death: DeathState::Alive,
        position: Position { x, y: 0.0, z: 0.0 },
        debuffs: Vec::new(),
    }
}

fn group_with(is_raid: bool, members: Vec<GroupMemberSnapshot>) -> GroupState {
    let mut group = GroupState::default();
    group.apply_roster(GroupRosterSnapshot {
        is_raid,
        ready_count: 0,
        total_count: 0,
        members,
        loot_method: LootMode::PersonalLoot,
    });
    group
}

fn no_icon(_: u32) -> u32 {
    0
}

fn views(group: &GroupState, local: &str, target: Option<&str>) -> GroupFramesState {
    build_frames(&ViewContext {
        group,
        local_name: Some(local),
        target_name: target,
        icon_fdid: &no_icon,
    })
    .0
}

#[test]
fn party_frames_put_the_player_first_with_live_bars_status_and_range() {
    let mut group = group_with(
        false,
        vec![
            member("Ann", true, true, 1),
            member("Bob", false, true, 1),
            member("Cid", false, true, 1),
            member("Dee", false, false, 1),
        ],
    );
    let mut dead = live("Cid", 0, 10.0);
    dead.death = DeathState::Ghost;
    let mut far = live("Bob", 412, 90.0);
    far.debuffs = vec![AuraView {
        instance_id: 3,
        spell_id: 589,
        caster: None,
        stacks: 1,
        charges: 0,
        duration_ms: 18_000,
        remaining_ms: 9_000,
        harmful: true,
        dispel_type: 1,
        flags: 0,
    }];
    group.apply_member_states(vec![live("Ann", 1000, 0.0), far, dead]);

    let state = views(&group, "Bob", Some("Cid"));

    let names: Vec<_> = state.party.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["Bob", "Ann", "Cid", "Dee"]);
    let bob = &state.party[0];
    assert_eq!(bob.health_fraction, Some(0.412));
    assert_eq!(bob.power, Some((0.25, [1.0, 0.0, 0.0])));
    assert_eq!(bob.class_rgb, [0.96, 0.55, 0.73], "paladin class colour");
    assert!(bob.in_range, "the player is always in range");
    assert_eq!(bob.debuffs.len(), 1);
    assert_eq!(bob.debuffs[0].dispel, DebuffType::Magic);
    assert!(!state.party[1].in_range, "Ann is 90 yd from Bob");
    assert_eq!(state.party[2].status, UnitStatus::Dead, "ghost shows Dead");
    assert!(state.party[2].selected);
    assert!(!state.party[1].selected);
    assert_eq!(state.party[3].status, UnitStatus::Offline);
    assert_eq!(state.party[3].health_fraction, None);
    assert!(state.raid.iter().all(Vec::is_empty));
}

#[test]
fn raid_frames_group_members_by_subgroup_and_show_ready_marks() {
    let mut group = group_with(
        true,
        vec![
            member("Ann", true, true, 1),
            member("Bob", false, true, 3),
            member("Cid", false, true, 8),
            member("Eve", false, true, 3),
        ],
    );
    group.apply_ready_check(ReadyCheckUpdate {
        initiator_name: "Ann".into(),
        time_remaining_secs: 25.0,
        members: vec![
            ReadyCheckMemberSnapshot {
                name: "Ann".into(),
                answer: ReadyCheckAnswer::Ready,
            },
            ReadyCheckMemberSnapshot {
                name: "Bob".into(),
                answer: ReadyCheckAnswer::Pending,
            },
        ],
        finished: false,
    });

    let state = views(&group, "Ann", None);

    assert!(state.party.is_empty());
    let columns: Vec<Vec<&str>> = state
        .raid
        .iter()
        .map(|g| g.iter().map(|v| v.name.as_str()).collect())
        .collect();
    assert_eq!(
        columns,
        [
            vec!["Ann"],
            vec![],
            vec!["Bob", "Eve"],
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["Cid"]
        ]
    );
    assert_eq!(state.raid[0][0].ready, Some(ReadyMark::Ready));
    assert_eq!(state.raid[2][0].ready, Some(ReadyMark::Waiting));
    assert_eq!(state.raid[2][1].ready, None);
}

#[test]
fn ready_check_frame_shows_only_for_members_who_must_answer() {
    let mut group = group_with(
        false,
        vec![member("Ann", true, true, 1), member("Bob", false, true, 1)],
    );
    group.apply_ready_check(ReadyCheckUpdate {
        initiator_name: "Ann".into(),
        time_remaining_secs: 30.0,
        members: vec![
            ReadyCheckMemberSnapshot {
                name: "Ann".into(),
                answer: ReadyCheckAnswer::Ready,
            },
            ReadyCheckMemberSnapshot {
                name: "Bob".into(),
                answer: ReadyCheckAnswer::Pending,
            },
        ],
        finished: false,
    });

    assert!(build_ready_check(&group, Some("Bob")).visible);
    assert_eq!(build_ready_check(&group, Some("Bob")).initiator, "Ann");
    assert!(!build_ready_check(&group, Some("Ann")).visible);
}

// --- Clicks against the laid-out frames ---

fn registry_for(state: &GroupFramesState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    Screen::new(group_frames_screen).sync(&shared, &mut registry);
    compute_layout(&mut registry);
    registry
}

fn center(registry: &FrameRegistry, name: &str) -> Vec2 {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name}"));
    let rect = registry.get(id).unwrap().layout_rect.clone().unwrap();
    Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

fn party_fixture() -> (GroupState, GroupFrameClickMap) {
    let group = group_with(
        false,
        vec![member("Ann", true, true, 1), member("Bob", false, true, 1)],
    );
    let (_, click_map) = build_frames(&ViewContext {
        group: &group,
        local_name: Some("Ann"),
        target_name: None,
        icon_fdid: &no_icon,
    });
    (group, click_map)
}

fn click(
    registry: &FrameRegistry,
    click_map: &GroupFrameClickMap,
    menu: &mut GroupFrameMenu,
    at: Vec2,
    button: MouseButton,
) -> Vec<ClickOutcome> {
    let bob = Entity::from_bits(77);
    let entity_of = |name: &str| (name == "Bob").then_some(bob);
    GroupFrameClick {
        registry,
        click_map,
        entity_of: &entity_of,
    }
    .handle(at, button, menu)
}

#[test]
fn party_frames_sit_left_of_the_player_frame_bottom_aligned_with_it_growing_upward() {
    let (group, _) = party_fixture();
    let registry = registry_for(&views(&group, "Ann", None));
    let id = registry.get_by_name(PARTY_FRAME_NAME).unwrap();
    let rect = registry.get(id).unwrap().layout_rect.clone().unwrap();

    // PlayerFrame left edge is 281 left of centre; the column ends 12 before it.
    assert_eq!(rect.x + rect.width, 960.0 - 281.0 - 12.0);
    assert_eq!(rect.y + rect.height, 1080.0 - 152.0);
    assert_eq!((rect.width, rect.height), (98.0, 14.0 + 2.0 * 44.0));
}

#[test]
fn raid_grid_is_centred_above_the_cluster_and_as_tall_as_its_fullest_group() {
    let group = group_with(
        true,
        vec![
            member("Ann", true, true, 1),
            member("Bob", false, true, 1),
            member("Cid", false, true, 4),
        ],
    );
    let registry = registry_for(&views(&group, "Ann", None));
    let id = registry.get_by_name("CompactRaidFrameContainer").unwrap();
    let rect = registry.get(id).unwrap().layout_rect.clone().unwrap();

    assert_eq!((rect.x, rect.width), (960.0 - 288.0, 576.0));
    assert_eq!(rect.y + rect.height, 1080.0 - 215.0);
    assert_eq!(rect.height, 14.0 + 2.0 * 36.0);
    let cid = registry.get_by_name("CompactRaidGroup4Member1").unwrap();
    let cid_rect = registry.get(cid).unwrap().layout_rect.clone().unwrap();
    assert_eq!(
        (cid_rect.x, cid_rect.width),
        (960.0 - 288.0 + 3.0 * 72.0, 72.0)
    );
    assert!(registry.get_by_name("CompactRaidGroup2Title").is_none());
}

#[test]
fn left_click_targets_a_member_and_right_click_menu_promotes_them() {
    let (group, click_map) = party_fixture();
    let registry = registry_for(&views(&group, "Ann", None));
    let mut menu = GroupFrameMenu::default();
    let bob_frame = center(&registry, "CompactPartyFrameMember2");

    let targeted = click(
        &registry,
        &click_map,
        &mut menu,
        bob_frame,
        MouseButton::Left,
    );
    assert_eq!(targeted, [ClickOutcome::Target(Entity::from_bits(77))]);

    click(
        &registry,
        &click_map,
        &mut menu,
        bob_frame,
        MouseButton::Right,
    );
    assert_eq!(menu.unit.as_deref(), Some("Bob"));
    let mut state = views(&group, "Ann", None);
    state.menu = build_menu(
        &menu,
        &group,
        Some("Ann"),
        &HashMap::from([("Bob".to_string(), Entity::from_bits(77))]),
    );
    let labels: Vec<_> = state.menu.items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        labels[..5],
        [
            "Target",
            "Inspect",
            "Trade",
            "Promote to Leader",
            "Uninvite"
        ]
    );
    let registry = registry_for(&state);

    let promoted = click(
        &registry,
        &click_map,
        &mut menu,
        center(&registry, "GroupContextMenuPromote"),
        MouseButton::Left,
    );

    assert_eq!(
        promoted,
        [ClickOutcome::Command(GroupCommand::Promote("Bob".into()))]
    );
    assert_eq!(menu.unit, None, "running an entry closes the menu");

    menu.unit = Some("Bob".into());
    let traded = click(
        &registry,
        &click_map,
        &mut menu,
        center(&registry, "GroupContextMenuTrade"),
        MouseButton::Left,
    );
    assert_eq!(traded, [ClickOutcome::Trade("Bob".into())]);
}

#[test]
fn self_menu_converts_to_raid_and_leaves() {
    let (group, click_map) = party_fixture();
    let mut menu = GroupFrameMenu {
        unit: Some("Ann".into()),
        x: 400.0,
        y: 400.0,
    };
    let mut state = views(&group, "Ann", None);
    state.menu = build_menu(&menu, &group, Some("Ann"), &HashMap::new());
    let registry = registry_for(&state);

    let converted = click(
        &registry,
        &click_map,
        &mut menu,
        center(&registry, "GroupContextMenuConvertToRaid"),
        MouseButton::Left,
    );
    assert_eq!(
        converted,
        [ClickOutcome::Command(GroupCommand::ConvertToRaid)]
    );
    menu.unit = Some("Ann".into());
    let left = click(
        &registry,
        &click_map,
        &mut menu,
        center(&registry, "GroupContextMenuLeave"),
        MouseButton::Left,
    );

    assert_eq!(left, [ClickOutcome::Command(GroupCommand::Leave)]);
}

#[test]
fn ready_check_buttons_answer_the_check() {
    let (group, click_map) = party_fixture();
    let mut state = views(&group, "Bob", None);
    state.ready_check = ReadyCheckFrameState {
        visible: true,
        initiator: "Ann".into(),
    };
    let registry = registry_for(&state);
    let mut menu = GroupFrameMenu::default();

    let yes = click(
        &registry,
        &click_map,
        &mut menu,
        center(&registry, "ReadyCheckFrameYesButton"),
        MouseButton::Left,
    );
    let no = click(
        &registry,
        &click_map,
        &mut menu,
        center(&registry, "ReadyCheckFrameNoButton"),
        MouseButton::Left,
    );

    assert_eq!(
        yes,
        [ClickOutcome::Command(GroupCommand::RespondReadyCheck(true))]
    );
    assert_eq!(
        no,
        [ClickOutcome::Command(GroupCommand::RespondReadyCheck(
            false
        ))]
    );
}

// --- Invite popup ---

fn popup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<GroupState>()
        .init_resource::<PopupStack>()
        .add_message::<GroupCommand>()
        .add_message::<PopupResult>();
    app
}

fn written_commands(app: &mut App) -> Vec<GroupCommand> {
    app.world_mut()
        .resource_mut::<Messages<GroupCommand>>()
        .drain()
        .collect()
}

#[test]
fn invite_prompt_opens_party_invite_and_accept_answers_the_server() {
    let mut app = popup_app();
    app.world_mut().resource_mut::<GroupState>().pending_invite = Some("party_a".into());
    app.world_mut().run_system_once(sync_invite_popup).unwrap();

    let stack = app.world().resource::<PopupStack>();
    let visible = stack.visible();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].spec.text, "party_a invites you to a group.");
    assert_eq!(visible[0].spec.accept_label, "Accept");
    assert_eq!(visible[0].spec.cancel_label.as_deref(), Some("Decline"));
    assert_eq!(visible[0].spec.timeout, Some(Duration::from_secs(60)));

    let id = visible[0].id;
    app.world_mut()
        .resource_mut::<PopupStack>()
        .resolve(id, PopupOutcome::Accepted);
    let results = app.world_mut().resource_mut::<PopupStack>().drain_results();
    for result in results {
        app.world_mut().write_message(result);
    }
    app.world_mut()
        .run_system_once(answer_invite_popup)
        .unwrap();

    assert_eq!(
        written_commands(&mut app),
        [GroupCommand::RespondInvite(true)]
    );
    assert_eq!(app.world().resource::<GroupState>().pending_invite, None);
}

#[test]
fn timed_out_invite_declines_and_a_server_cancel_closes_the_popup() {
    let mut app = popup_app();
    app.world_mut().write_message(PopupResult {
        id: game_engine::ui::popup::PopupId(9),
        key: PARTY_INVITE_POPUP.into(),
        outcome: PopupOutcome::TimedOut,
    });
    app.world_mut()
        .run_system_once(answer_invite_popup)
        .unwrap();
    assert_eq!(
        written_commands(&mut app),
        [GroupCommand::RespondInvite(false)]
    );

    app.world_mut().resource_mut::<GroupState>().pending_invite = Some("party_a".into());
    app.world_mut().run_system_once(sync_invite_popup).unwrap();
    app.world_mut().resource_mut::<GroupState>().pending_invite = None;
    app.world_mut().run_system_once(sync_invite_popup).unwrap();

    assert!(
        !app.world()
            .resource::<PopupStack>()
            .contains(PARTY_INVITE_POPUP)
    );
}
