//! Talent window tests on the real Paladin tree 790 (12.1.0.69933 CSVs);
//! each test fails when `data/db2` lacks them. Node IDs match game-server
//! `trait_config_tests.rs`.

use std::path::Path;
use std::sync::{OnceLock, mpsc};
#[path = "../../../tests/unit/required_asset.rs"]
mod required_asset;
use required_asset::require_asset;

use bevy::ecs::system::RunSystemOnce;
use game_engine::network_events::{dispatch_incoming, dispatch_outgoing, register_message_handler};
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use game_engine::player_spells::receive_specialization_changed;
use game_engine::talent::TalentPlugin;
use game_engine::talent_tree::rules::{TraitContext, granted_entries};
use game_engine::talent_tree::{
    TalentTreeData, TalentTreePaths, load_talent_trees, talent_source_dir,
};
use game_engine::ui::screens::talent_frame_component::{
    TALENT_APPLY_BUTTON, TALENT_RESET_BUTTON, TALENT_STATE_PANEL, talent_node_name,
    talent_spec_button_name,
};
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::{SpecializationChanged, TraitCommitResult, TraitConfigSnapshot};

use super::*;

const PALADIN_TREE: u32 = 790;
const RETRIBUTION: u32 = 70;
const PROTECTION: u32 = 66;
const LEVEL: u8 = 80;
/// Hammer of Wrath, granted to Retribution by group 8486.
const HAMMER_OF_WRATH_NODE: u32 = 81510;
/// Blade of Justice: a Retribution root node, 1 rank (entry 102498).
const BLADE_OF_JUSTICE: (u32, u32) = (81526, 102498);
/// Avenging Wrath needs Expurgation 92689 fully ranked.
const AVENGING_WRATH_NODE: u32 = 81544;
/// Templar / Herald of the Sun hero selection, visible to Retribution only.
const RET_HERO_SELECTION_NODE: u32 = 99837;

fn trees() -> &'static TalentTreeData {
    static TREES: OnceLock<TalentTreeData> = OnceLock::new();
    TREES.get_or_init(|| {
        require_asset(talent_source_dir(Path::new("data")).join("TraitNode.csv"));
        let mut paths = TalentTreePaths::for_data_dir(Path::new("data"));
        paths.cache_path = std::env::temp_dir().join(format!(
            "talent_trees_frame_test_{}.bin",
            std::process::id()
        ));
        let data = load_talent_trees(&paths).expect("load talent trees");
        std::fs::remove_file(&paths.cache_path).expect("remove test cache");
        data
    })
}

/// What the server sends at level 80 for an empty config: granted entries
/// only, 31 class / 30 spec / 10 per hero currency.
fn empty_snapshot(data: &TalentTreeData, spec_id: u32) -> TraitConfigSnapshot {
    let tree = data.tree(PALADIN_TREE).unwrap();
    let ctx = TraitContext {
        spec_id,
        level: LEVEL,
    };
    TraitConfigSnapshot {
        spec_id,
        tree_id: PALADIN_TREE,
        entries: commit_entries(&granted_entries(tree, ctx)),
        unspent: vec![(2801, 31), (2800, 30), (2986, 10), (2987, 10), (2988, 10)],
    }
}

fn talent_app(data: &TalentTreeData) -> App {
    let mut app = App::new();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: Default::default(),
        focused_frame: None,
    });
    app.init_resource::<ConnectionSender>();
    app.add_plugins(TalentPlugin);
    register_message_handler::<SpecializationChanged, _>(
        &mut app,
        receive_specialization_changed,
        |_| true,
    );
    app.insert_resource(TalentTrees {
        state: TalentTreesState::Ready(data.clone()),
    });
    app.init_resource::<SpellCatalog>();
    app.init_resource::<TalentTooltips>();
    let mut window_manager = crate::window_manager::WindowManager::default();
    window_manager.open(crate::window_manager::WindowId::Talents);
    app.insert_resource(window_manager);
    app.init_resource::<PopupStack>();
    app.add_message::<PopupResult>();
    app.world_mut().spawn((LocalPlayer, UnitLevel(LEVEL)));
    app.world_mut()
        .run_system_once(build_talent_frame_ui)
        .unwrap();
    app.add_systems(
        Update,
        (apply_on_confirmation, sync_talent_frame_state)
            .chain()
            .after(game_engine::talent::follow_active_specialization),
    );
    app.update();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.insert_resource(Inbox::new(messages));
    dispatch_incoming(app.world_mut());
    app.update();
}

fn ret_app() -> App {
    let data = trees();
    let mut app = talent_app(data);
    deliver(&mut app, vec![empty_snapshot(data, RETRIBUTION)]);
    app
}

fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    let id = reg
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name} missing"));
    match reg.get(id).and_then(|frame| frame.widget_data.as_ref()) {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        _ => panic!("{name} is not a fontstring"),
    }
}

fn registry(app: &App) -> &FrameRegistry {
    &app.world().resource::<UiState>().registry
}

fn has_frame(app: &App, name: &str) -> bool {
    registry(app).get_by_name(name).is_some()
}

fn rank_text(app: &App, node_id: u32) -> String {
    fontstring_text(registry(app), &format!("{}Rank", talent_node_name(node_id)))
}

/// Resolves the frame's onclick like a real click and dispatches it.
fn click(app: &mut App, frame: &str, button: MouseButton) {
    let reg = registry(app);
    let id = reg
        .get_by_name(frame)
        .unwrap_or_else(|| panic!("{frame} missing"));
    let action = walk_up_for_onclick(reg, id).unwrap_or_else(|| panic!("{frame} has no action"));
    app.world_mut()
        .resource_scope(|world, mut popups: Mut<PopupStack>| {
            world.resource_scope(|world, mut state: Mut<TalentState>| {
                let trees = world.resource::<TalentTrees>();
                dispatch_action(&action, button, trees, Some(LEVEL), &mut state, &mut popups);
            });
        });
    app.update();
}

/// Resolves the open Apply confirmation like its popup button and emits the result.
fn answer_apply_popup(app: &mut App, outcome: PopupOutcome) {
    let mut stack = app.world_mut().resource_mut::<PopupStack>();
    let popup = stack.visible().pop().expect("apply confirmation shown");
    assert_eq!(popup.spec.text, "Apply talent changes?");
    assert_eq!(popup.spec.accept_label, "Accept");
    assert_eq!(popup.spec.cancel_label.as_deref(), Some("Cancel"));
    stack.resolve(popup.id, outcome);
    let results = stack.drain_results();
    app.world_mut().write_message_batch(results);
    app.update();
}

fn queued_commit_count(app: &App) -> usize {
    app.world()
        .resource::<TalentState>()
        .queued_commits()
        .count()
}

#[test]
fn loading_until_the_first_snapshot_arrives() {
    let data = trees();
    let app = talent_app(data);
    assert!(has_frame(&app, TALENT_STATE_PANEL));
    assert!(!has_frame(&app, &talent_node_name(HAMMER_OF_WRATH_NODE)));
}

#[test]
fn retribution_snapshot_renders_granted_node_rank_and_points() {
    let app = ret_app();
    assert!(!has_frame(&app, TALENT_STATE_PANEL));
    assert_eq!(rank_text(&app, HAMMER_OF_WRATH_NODE), "1/1");
    assert_eq!(rank_text(&app, BLADE_OF_JUSTICE.0), "0/1");
    assert_eq!(
        fontstring_text(registry(&app), "TalentSpecPointsText"),
        "Retribution  30"
    );
    assert_eq!(
        fontstring_text(registry(&app), "TalentClassPointsText"),
        "Paladin  31"
    );
}

#[test]
fn clicking_an_available_node_buys_a_pending_rank() {
    let mut app = ret_app();
    click(
        &mut app,
        &talent_node_name(BLADE_OF_JUSTICE.0),
        MouseButton::Left,
    );
    assert_eq!(rank_text(&app, BLADE_OF_JUSTICE.0), "1/1");
    assert_eq!(
        fontstring_text(registry(&app), "TalentSpecPointsText"),
        "Retribution  29"
    );
    click(
        &mut app,
        &talent_node_name(BLADE_OF_JUSTICE.0),
        MouseButton::Right,
    );
    assert_eq!(rank_text(&app, BLADE_OF_JUSTICE.0), "0/1");
    assert!(app.world().resource::<TalentState>().pending.is_none());
}

#[test]
fn clicking_a_node_with_an_unmet_prerequisite_does_nothing() {
    let mut app = ret_app();
    click(
        &mut app,
        &talent_node_name(AVENGING_WRATH_NODE),
        MouseButton::Left,
    );
    assert_eq!(rank_text(&app, AVENGING_WRATH_NODE), "0/1");
    assert!(app.world().resource::<TalentState>().pending.is_none());
    assert_eq!(
        fontstring_text(registry(&app), "TalentSpecPointsText"),
        "Retribution  30"
    );
}

#[test]
fn apply_sends_commit_with_the_pending_entries_and_reset_discards() {
    let mut app = ret_app();
    click(
        &mut app,
        &talent_node_name(BLADE_OF_JUSTICE.0),
        MouseButton::Left,
    );
    click(&mut app, TALENT_APPLY_BUTTON, MouseButton::Left);
    assert_eq!(
        queued_commit_count(&app),
        0,
        "nothing is sent before Accept"
    );
    answer_apply_popup(&mut app, PopupOutcome::Accepted);
    let commits: Vec<CommitTraitConfig> = app
        .world()
        .resource::<TalentState>()
        .queued_commits()
        .cloned()
        .collect();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].spec_id, RETRIBUTION);
    let blade = commits[0]
        .entries
        .iter()
        .find(|entry| entry.node_id == BLADE_OF_JUSTICE.0)
        .expect("blade of justice committed");
    assert_eq!((blade.entry_id, blade.rank), (BLADE_OF_JUSTICE.1, 1));
    assert!(
        commits[0]
            .entries
            .iter()
            .any(|entry| entry.node_id == HAMMER_OF_WRATH_NODE && entry.rank == 1),
        "granted ranks are sent as totals"
    );

    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    dispatch_outgoing(app.world_mut());
    assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    assert_eq!(
        app.world()
            .resource::<TalentState>()
            .queued_commits()
            .count(),
        0
    );

    click(&mut app, TALENT_RESET_BUTTON, MouseButton::Left);
    assert!(app.world().resource::<TalentState>().pending.is_none());
    assert_eq!(rank_text(&app, BLADE_OF_JUSTICE.0), "0/1");
}

#[test]
fn cancelling_the_apply_confirmation_sends_nothing_and_keeps_pending() {
    let mut app = ret_app();
    click(
        &mut app,
        &talent_node_name(BLADE_OF_JUSTICE.0),
        MouseButton::Left,
    );
    click(&mut app, TALENT_APPLY_BUTTON, MouseButton::Left);
    answer_apply_popup(&mut app, PopupOutcome::Cancelled);
    assert_eq!(queued_commit_count(&app), 0);
    assert!(app.world().resource::<TalentState>().pending.is_some());
    assert_eq!(rank_text(&app, BLADE_OF_JUSTICE.0), "1/1");
}

#[test]
fn rejected_commit_shows_the_server_reason_in_ui_errors() {
    let mut app = ret_app();
    let reason = "node 81544 requires one of nodes [92689] fully ranked";
    deliver(
        &mut app,
        vec![TraitCommitResult {
            ok: false,
            reason: Some(reason.into()),
        }],
    );
    let errors = app.world().resource::<UiErrors>();
    assert_eq!(errors.lines.len(), 1);
    assert_eq!(errors.lines[0].text, reason);
}

#[test]
fn specialization_change_rebuilds_the_tree_for_the_new_spec() {
    let data = trees();
    let mut app = ret_app();
    assert!(has_frame(&app, &talent_node_name(RET_HERO_SELECTION_NODE)));
    assert!(has_frame(&app, &talent_node_name(BLADE_OF_JUSTICE.0)));

    deliver(
        &mut app,
        vec![SpecializationChanged {
            spec_id: PROTECTION,
        }],
    );
    assert!(has_frame(&app, TALENT_STATE_PANEL));

    deliver(&mut app, vec![empty_snapshot(data, PROTECTION)]);
    assert!(!has_frame(&app, TALENT_STATE_PANEL));
    assert!(!has_frame(&app, &talent_node_name(RET_HERO_SELECTION_NODE)));
    assert!(!has_frame(&app, &talent_node_name(BLADE_OF_JUSTICE.0)));
    assert_eq!(
        fontstring_text(registry(&app), "TalentSpecPointsText"),
        "Protection  30"
    );
}

#[test]
fn spec_buttons_request_the_other_specs() {
    let mut app = ret_app();
    let names: Vec<String> = (0..3).map(talent_spec_button_name).collect();
    let prot = names
        .iter()
        .find(|name| {
            let reg = registry(&app);
            reg.get_by_name(name)
                .and_then(|id| walk_up_for_onclick(reg, id))
                .is_some_and(|action| action == format!("talent_spec:{PROTECTION}"))
        })
        .expect("protection spec button")
        .clone();
    click(&mut app, &prot, MouseButton::Left);
    let requests: Vec<u32> = app
        .world()
        .resource::<TalentState>()
        .queued_spec_requests()
        .collect();
    assert_eq!(requests, [PROTECTION]);
}
