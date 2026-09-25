use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::input_bindings::InputAction;
use game_engine::spell_catalog::{SpellCatalog, SpellTextContext, SpellTextSources};
use game_engine::talent::TalentState;
use game_engine::talent_tree::rules::ConfigEntry;
use game_engine::talent_tree::session::{TalentSession, commit_entries};
use game_engine::talent_tree::{TalentTrees, TalentTreesState};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::talent_frame_component::{
    ACTION_TALENT_APPLY, ACTION_TALENT_CHOICE_PREFIX, ACTION_TALENT_NODE_PREFIX,
    ACTION_TALENT_RESET, ACTION_TALENT_SPEC_PREFIX, TalentBody, TalentFrameState, talent_edge_name,
    talent_frame_screen,
};
use game_engine::ui::screens::talent_frame_view::{
    TalentTooltips, TalentViewInput, build_tree_view,
};
use shared::components::UnitLevel;
use shared::protocol::CommitTraitConfig;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

/// `PopupStack` key of the Apply confirmation.
const APPLY_POPUP_KEY: &str = "TALENT_APPLY_CHANGES";

struct TalentFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for TalentFrameRes {}
unsafe impl Sync for TalentFrameRes {}

#[derive(Resource)]
struct TalentFrameWrap(TalentFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct TalentFrameModel(TalentFrameState);

pub struct TalentFramePlugin;

impl Plugin for TalentFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TalentTooltips>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_talent_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_talent_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_talent_frame,
                handle_talent_frame_input,
                apply_on_confirmation,
                sync_talent_frame_state,
            )
                .chain()
                .after(game_engine::talent::follow_active_specialization)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Inputs of the talent window model.
struct TalentInputs<'a> {
    open: bool,
    trees: &'a TalentTrees,
    state: &'a TalentState,
    catalog: &'a SpellCatalog,
    text_ctx: &'a SpellTextContext,
    level: Option<u8>,
}

fn local_level(levels: &Query<&UnitLevel, With<LocalPlayer>>) -> Option<u8> {
    levels.iter().next().map(|level| level.0)
}

fn build_talent_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    window_manager: Res<WindowManager>,
    trees: Res<TalentTrees>,
    state: Res<TalentState>,
    catalog: Res<SpellCatalog>,
    text: SpellTextSources,
    levels: Query<&UnitLevel, With<LocalPlayer>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let (model, tooltips) = build_state(&TalentInputs {
        open: window_manager.is_open(WindowId::Talents),
        trees: &trees,
        state: &state,
        catalog: &catalog,
        text_ctx: &text.context(),
        level: local_level(&levels),
    });
    let mut shared = SharedContext::new();
    shared.insert(model.clone());
    let mut screen = Screen::new(talent_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    rotate_edges(&mut ui.registry, &model);
    commands.insert_resource(TalentFrameWrap(TalentFrameRes { screen, shared }));
    commands.insert_resource(TalentFrameModel(model));
    commands.insert_resource(tooltips);
}

fn teardown_talent_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<TalentFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<TalentFrameWrap>();
    commands.remove_resource::<TalentFrameModel>();
    commands.insert_resource(TalentTooltips::default());
}

fn toggle_talent_frame(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
) {
    if keybinds.just_pressed(InputAction::ToggleTalents) {
        window_manager.toggle(WindowId::Talents);
    }
}

fn sync_talent_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<TalentFrameWrap>>,
    mut last_model: Option<ResMut<TalentFrameModel>>,
    mut tooltips: ResMut<TalentTooltips>,
    window_manager: Res<WindowManager>,
    trees: Res<TalentTrees>,
    state: Res<TalentState>,
    catalog: Res<SpellCatalog>,
    text: SpellTextSources,
    levels: Query<&UnitLevel, With<LocalPlayer>>,
    level_changes: Query<(), (With<LocalPlayer>, Changed<UnitLevel>)>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let inputs_changed = window_manager.is_changed()
        || trees.is_changed()
        || state.is_changed()
        || catalog.is_changed()
        || text.is_changed()
        || !level_changes.is_empty();
    if !inputs_changed {
        return;
    }
    let (model, next_tooltips) = build_state(&TalentInputs {
        open: window_manager.is_open(WindowId::Talents),
        trees: &trees,
        state: &state,
        catalog: &catalog,
        text_ctx: &text.context(),
        level: local_level(&levels),
    });
    if *tooltips != next_tooltips {
        *tooltips = next_tooltips;
    }
    if last_model.0 == model {
        return;
    }
    last_model.0 = model.clone();
    let res = &mut wrap.0;
    res.shared.insert(model.clone());
    res.screen.sync(&res.shared, &mut ui.registry);
    rotate_edges(&mut ui.registry, &model);
}

/// `rsx!` has no rotation attribute, so edge textures get their angle after
/// each sync. Toolkit texture rotation is counterclockwise; edge angles are
/// screen angles (y down, clockwise).
fn rotate_edges(registry: &mut FrameRegistry, model: &TalentFrameState) {
    let TalentBody::Tree(view) = &model.body else {
        return;
    };
    for edge in &view.edges {
        let Some(id) = registry.get_by_name(&talent_edge_name(edge.from, edge.to)) else {
            continue;
        };
        if let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
        {
            texture.rotation = -edge.angle;
        }
    }
}

fn build_state(inputs: &TalentInputs) -> (TalentFrameState, TalentTooltips) {
    let visible = inputs.open;
    let loading = || {
        (
            TalentFrameState {
                visible,
                body: TalentBody::Loading,
            },
            TalentTooltips::default(),
        )
    };
    let data = match &inputs.trees.state {
        TalentTreesState::Loading => return loading(),
        TalentTreesState::Failed(err) => {
            let body = TalentBody::Failed(format!("Talent data failed to load: {err}"));
            return (
                TalentFrameState { visible, body },
                TalentTooltips::default(),
            );
        }
        TalentTreesState::Ready(data) => data,
    };
    let (Some(snapshot), Some(level)) = (inputs.state.snapshot.as_ref(), inputs.level) else {
        return loading();
    };
    let Some(tree) = data.tree(snapshot.tree_id) else {
        let body = TalentBody::Failed(format!("Unknown talent tree {}", snapshot.tree_id));
        return (
            TalentFrameState { visible, body },
            TalentTooltips::default(),
        );
    };
    let session = TalentSession::new(tree, level, snapshot);
    let config = inputs.state.pending.as_deref().unwrap_or(&session.base);
    let (view, tooltips) = build_tree_view(&TalentViewInput {
        data,
        session: &session,
        config,
        catalog: inputs.catalog,
        text_ctx: inputs.text_ctx,
        has_pending: inputs.state.pending.is_some(),
    });
    let body = TalentBody::Tree(view);
    (TalentFrameState { visible, body }, tooltips)
}

fn handle_talent_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    window_manager: Res<WindowManager>,
    trees: Res<TalentTrees>,
    mut state: ResMut<TalentState>,
    mut popups: ResMut<PopupStack>,
    levels: Query<&UnitLevel, With<LocalPlayer>>,
) {
    if !window_manager.is_open(WindowId::Talents)
        || !crate::networking::gameplay_input_allowed(reconnect)
        || modal_open.is_some()
    {
        return;
    }
    let Some(mouse) = mouse else { return };
    let button = if mouse.just_pressed(MouseButton::Left) {
        MouseButton::Left
    } else if mouse.just_pressed(MouseButton::Right) {
        MouseButton::Right
    } else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    let level = local_level(&levels);
    dispatch_action(&action, button, &trees, level, &mut state, &mut popups);
}

enum TalentAction {
    Node(u32),
    Choice(u32, u32),
    Spec(u32),
    Apply,
    Reset,
}

fn parse_action(action: &str) -> Option<TalentAction> {
    match action {
        ACTION_TALENT_APPLY => return Some(TalentAction::Apply),
        ACTION_TALENT_RESET => return Some(TalentAction::Reset),
        _ => {}
    }
    if let Some(node) = action.strip_prefix(ACTION_TALENT_NODE_PREFIX) {
        return node.parse().ok().map(TalentAction::Node);
    }
    if let Some(spec) = action.strip_prefix(ACTION_TALENT_SPEC_PREFIX) {
        return spec.parse().ok().map(TalentAction::Spec);
    }
    let (node, entry) = action
        .strip_prefix(ACTION_TALENT_CHOICE_PREFIX)?
        .split_once(':')?;
    Some(TalentAction::Choice(
        node.parse().ok()?,
        entry.parse().ok()?,
    ))
}

/// Left click buys a rank, right click refunds one; both only when the
/// mirrored rules accept the result. Apply asks for confirmation first.
fn dispatch_action(
    action: &str,
    button: MouseButton,
    trees: &TalentTrees,
    level: Option<u8>,
    state: &mut TalentState,
    popups: &mut PopupStack,
) {
    let Some(action) = parse_action(action) else {
        return;
    };
    match action {
        TalentAction::Reset => state.pending = None,
        TalentAction::Spec(spec_id) => state.queue_set_spec(spec_id),
        TalentAction::Apply => confirm_apply(state, popups),
        TalentAction::Node(node_id) => edit_rank(trees, level, state, node_id, None, button),
        TalentAction::Choice(node_id, entry_id) => {
            edit_rank(trees, level, state, node_id, Some(entry_id), button)
        }
    }
}

/// Plan rule 11: talent changes need a confirmation before they are sent.
fn confirm_apply(state: &TalentState, popups: &mut PopupStack) {
    if state.pending.is_none() {
        return;
    }
    popups.push(PopupSpec {
        key: APPLY_POPUP_KEY.to_string(),
        text: "Apply talent changes?".to_string(),
        accept_label: "Accept".to_string(),
        cancel_label: Some("Cancel".to_string()),
        timeout: None,
    });
}

/// Sends the pending config when the Apply confirmation is accepted.
fn apply_on_confirmation(mut results: MessageReader<PopupResult>, mut state: ResMut<TalentState>) {
    for result in results.read() {
        if result.key == APPLY_POPUP_KEY && result.outcome == PopupOutcome::Accepted {
            queue_apply(&mut state);
        }
    }
}

fn queue_apply(state: &mut TalentState) {
    let (Some(pending), Some(snapshot)) = (state.pending.as_ref(), state.snapshot.as_ref()) else {
        return;
    };
    let commit = CommitTraitConfig {
        spec_id: snapshot.spec_id,
        entries: commit_entries(pending),
    };
    state.queue_commit(commit);
}

fn edit_rank(
    trees: &TalentTrees,
    level: Option<u8>,
    state: &mut TalentState,
    node_id: u32,
    entry_id: Option<u32>,
    button: MouseButton,
) {
    let (Some(data), Some(snapshot), Some(level)) = (trees.data(), state.snapshot.as_ref(), level)
    else {
        return;
    };
    let Some(tree) = data.tree(snapshot.tree_id) else {
        return;
    };
    let Some(node) = tree.node(node_id) else {
        return;
    };
    let session = TalentSession::new(tree, level, snapshot);
    let config = state.pending.as_deref().unwrap_or(&session.base);
    let next = if button == MouseButton::Right {
        entry_id
            .or_else(|| session.remove_target(config, node))
            .and_then(|entry_id| session.try_remove(config, node_id, entry_id))
    } else {
        entry_id
            .or_else(|| session.add_target(config, node))
            .and_then(|entry_id| session.try_add(config, node_id, entry_id))
    };
    if let Some(next) = next {
        state.pending = (!same_config(&next, &session.base)).then_some(next);
    }
}

fn same_config(a: &[ConfigEntry], b: &[ConfigEntry]) -> bool {
    let sorted = |config: &[ConfigEntry]| {
        let mut config = config.to_vec();
        config.sort_by_key(|entry| (entry.node_id, entry.entry_id));
        config
    };
    sorted(a) == sorted(b)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
