//! ClassTrainerFrame scene: builds the Retail frame from [`TrainerState`], the spell
//! catalog (service names and icons), the owner's professions and money; opens and
//! closes its window with the trainer interaction; turns clicks into
//! [`TrainerRequest`]s. A service that adds a primary profession asks for
//! confirmation first (`CONFIRM_PROFESSION`, Blizzard_TrainerUI.lua:11-33).

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::professions_data::{ProfessionCatalog, profession_catalog};
use game_engine::spell_catalog::SpellCatalog;
use game_engine::status::{CharacterStatsSnapshot, ProfessionStatusSnapshot};
use game_engine::trainer_data::{TrainerRequest, TrainerState};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::trainer_frame_component::{
    ACTION_CLOSE, ACTION_SERVICE_PREFIX, ACTION_TRAIN, LIST_NAME, ServiceRow, TrainerFrameState,
    trainer_frame_screen,
};
use shared::protocol::{TrainerService, TrainerServiceState};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking_quests::NpcInteractionRequest;
use crate::scenes::static_popup::StaticPopupSystems;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

/// `INV_Misc_QuestionMark`, Retail's icon for a spell without one.
const UNKNOWN_ICON_FDID: u32 = 134_400;
const CONFIRM_PROFESSION: &str = "CONFIRM_PROFESSION";

struct TrainerFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for TrainerFrameRes {}
unsafe impl Sync for TrainerFrameRes {}

#[derive(Resource)]
struct TrainerFrameWrap(TrainerFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct TrainerFrameModel(TrainerFrameState);

pub struct TrainerFramePlugin;

impl Plugin for TrainerFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrainerState>()
            .init_resource::<PopupStack>()
            .add_message::<TrainerRequest>()
            .add_message::<PopupResult>()
            .add_message::<NpcInteractionRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_trainer_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_trainer_frame_ui);
        app.add_systems(
            Update,
            (
                handle_trainer_frame_input,
                handle_trainer_wheel,
                confirm_profession,
                sync_trainer_window,
                sync_trainer_frame_state,
            )
                .chain()
                .after(StaticPopupSystems)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Everything the frame shows.
#[derive(SystemParam)]
struct TrainerView<'w> {
    trainer: Res<'w, TrainerState>,
    manager: Res<'w, WindowManager>,
    stats: Option<Res<'w, CharacterStatsSnapshot>>,
    professions: Option<Res<'w, ProfessionStatusSnapshot>>,
    spells: Option<Res<'w, SpellCatalog>>,
}

/// What a service row needs besides the service.
pub(crate) struct Player<'a> {
    pub level: u16,
    pub money: u64,
    pub professions: &'a ProfessionStatusSnapshot,
}

impl TrainerView<'_> {
    fn state(&self) -> TrainerFrameState {
        let empty = ProfessionStatusSnapshot::default();
        let player = Player {
            level: self
                .stats
                .as_ref()
                .and_then(|stats| stats.level)
                .unwrap_or(1),
            money: self.stats.as_ref().map_or(0, |stats| u64::from(stats.gold)),
            professions: self.professions.as_deref().unwrap_or(&empty),
        };
        let spell_name_icon = |spell: u32| {
            self.spells
                .as_ref()
                .and_then(|catalog| catalog.get(spell))
                .map(|spell| (spell.name.to_string(), spell.icon_fdid))
        };
        build_state(
            &self.trainer,
            self.manager.is_open(WindowId::Trainer),
            &player,
            profession_catalog(),
            &spell_name_icon,
        )
    }
}

fn build_trainer_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    view: TrainerView,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = view.state();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(trainer_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(TrainerFrameWrap(TrainerFrameRes { screen, shared }));
    commands.insert_resource(TrainerFrameModel(state));
}

fn teardown_trainer_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<TrainerFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<TrainerFrameWrap>();
    commands.remove_resource::<TrainerFrameModel>();
}

fn sync_trainer_frame_state(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<TrainerFrameWrap>>,
    last_model: Option<ResMut<TrainerFrameModel>>,
    view: TrainerView,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap, last_model) else {
        return;
    };
    let state = view.state();
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// A trainer list opens the window; the window manager closing it ends the
/// interaction with `CloseInteraction`; the server ending it closes the window.
fn sync_trainer_window(
    mut manager: ResMut<WindowManager>,
    mut trainer: ResMut<TrainerState>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut open_npc: Local<Option<u64>>,
) {
    let window_open = manager.is_open(WindowId::Trainer);
    if trainer.npc.is_some() && *open_npc != trainer.npc {
        manager.open(WindowId::Trainer);
    } else if let Some(npc) = trainer.npc.filter(|_| !window_open) {
        trainer.close();
        requests.write(NpcInteractionRequest::Close { npc });
    } else if trainer.npc.is_none() && window_open {
        manager.close(WindowId::Trainer);
    }
    *open_npc = trainer.npc;
}

pub(crate) fn build_state(
    trainer: &TrainerState,
    window_open: bool,
    player: &Player,
    catalog: &ProfessionCatalog,
    spell_name_icon: &dyn Fn(u32) -> Option<(String, u32)>,
) -> TrainerFrameState {
    let rows = trainer
        .visible_services()
        .iter()
        .map(|service| {
            let mut row = service_row(service, player, catalog, spell_name_icon);
            row.selected = trainer.selected == Some(service.spell_id);
            row
        })
        .collect();
    TrainerFrameState {
        visible: window_open && trainer.is_open(),
        title: trainer.npc_name.clone(),
        rank: rank(trainer, player, catalog),
        rows,
        train_enabled: trainer
            .selected
            .and_then(|spell| trainer.service(spell))
            .is_some_and(|service| can_train(service, player, catalog)),
        money: player.money,
    }
}

/// `GetTrainerTradeskillRankValues`: the line most of the services require.
fn rank(
    trainer: &TrainerState,
    player: &Player,
    catalog: &ProfessionCatalog,
) -> Option<(String, f32)> {
    let mut counts: Vec<(u32, usize)> = Vec::new();
    for line in trainer
        .services
        .iter()
        .map(|service| service.req_skill_line)
        .filter(|line| catalog.line(*line).is_some_and(|info| info.parent != 0))
    {
        match counts.iter_mut().find(|(known, _)| *known == line) {
            Some((_, count)) => *count += 1,
            None => counts.push((line, 1)),
        }
    }
    let line = counts.iter().max_by_key(|(_, count)| *count)?.0;
    let learned = player
        .professions
        .lines
        .iter()
        .find(|known| known.skill_line == line)?;
    (learned.rank > 0).then(|| {
        (
            // TRADESKILL_RANK "%d/%d".
            format!("{}/{}", learned.rank, learned.max_rank),
            f32::from(learned.rank) / f32::from(learned.max_rank.max(1)),
        )
    })
}

fn service_row(
    service: &TrainerService,
    player: &Player,
    catalog: &ProfessionCatalog,
    spell_name_icon: &dyn Fn(u32) -> Option<(String, u32)>,
) -> ServiceRow {
    let (name, icon_fdid) =
        spell_name_icon(service.spell_id).unwrap_or_else(|| ("Unknown".into(), UNKNOWN_ICON_FDID)); // UNKNOWN
    let known = service.state == TrainerServiceState::Known;
    let (sub_text, sub_text_red) = if known {
        ("Already known".to_string(), false) // ITEM_SPELL_KNOWN
    } else {
        requirements(service, player, catalog, spell_name_icon)
    };
    let cost = (!known && service.cost > 0).then_some(service.cost);
    ServiceRow {
        name,
        icon_fdid: if icon_fdid == 0 {
            UNKNOWN_ICON_FDID
        } else {
            icon_fdid
        },
        sub_text,
        sub_text_red,
        unavailable: service.state == TrainerServiceState::Unavailable,
        cost,
        cost_red: cost.is_some_and(|cost| player.money < cost),
        selected: false,
    }
}

/// `REQUIRES_LABEL` followed by level, skill and ability requirements joined by
/// `PLAYER_LIST_DELIMITER` (Blizzard_TrainerUI.lua:208-266); red when one is unmet.
fn requirements(
    service: &TrainerService,
    player: &Player,
    catalog: &ProfessionCatalog,
    spell_name_icon: &dyn Fn(u32) -> Option<(String, u32)>,
) -> (String, bool) {
    let mut parts = Vec::new();
    let mut unmet = false;
    if service.req_level > 1 {
        parts.push(format!("Level {}", service.req_level)); // TRAINER_REQ_LEVEL
        unmet |= player.level < u16::from(service.req_level);
    }
    if service.req_skill_line != 0 {
        let name = catalog
            .line(service.req_skill_line)
            .map_or_else(|| "Unknown".to_string(), |line| line.name.clone());
        parts.push(format!("{name} ({})", service.req_skill_rank)); // TRAINER_REQ_SKILL_RANK
        let rank = player
            .professions
            .lines
            .iter()
            .find(|line| line.skill_line == service.req_skill_line)
            .map_or(0, |line| line.rank);
        unmet |= rank < service.req_skill_rank;
    }
    for ability in &service.req_abilities {
        let name = spell_name_icon(*ability).map_or_else(|| "Unknown".to_string(), |(n, _)| n);
        parts.push(name);
        unmet |= !player.professions.spells.contains(ability);
    }
    if parts.is_empty() {
        return (String::new(), false);
    }
    (format!("Requires: {}", parts.join(", ")), unmet)
}

/// Train is enabled for an available, affordable service; a profession also needs a
/// free slot (Blizzard_TrainerUI.lua:273-307).
fn can_train(service: &TrainerService, player: &Player, catalog: &ProfessionCatalog) -> bool {
    service.state == TrainerServiceState::Available
        && player.money >= service.cost
        && !(service.profession && primary_count(player, catalog) >= 2)
}

fn primary_count(player: &Player, catalog: &ProfessionCatalog) -> usize {
    player
        .professions
        .lines
        .iter()
        .filter(|line| catalog.is_primary(line.skill_line))
        .count()
}

#[derive(SystemParam)]
struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    /// The `onclick` action under the cursor on a left click this frame.
    fn click(self, ui: &UiState) -> Option<String> {
        if !self.mouse.as_ref()?.just_pressed(MouseButton::Left)
            || self.modal_open.is_some()
            || !crate::networking::gameplay_input_allowed(self.reconnect)
        {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        let frame_id = find_frame_at(&ui.registry, cursor.x, cursor.y)?;
        walk_up_for_onclick(&ui.registry, frame_id)
    }
}

#[derive(SystemParam)]
struct TrainActions<'w> {
    requests: MessageWriter<'w, TrainerRequest>,
    popups: ResMut<'w, PopupStack>,
    professions: Option<Res<'w, ProfessionStatusSnapshot>>,
    spells: Option<Res<'w, SpellCatalog>>,
}

fn handle_trainer_frame_input(
    pointer: Pointer,
    ui: Res<UiState>,
    mut trainer: ResMut<TrainerState>,
    mut manager: ResMut<WindowManager>,
    mut actions: TrainActions,
) {
    if !trainer.is_open() {
        return;
    }
    let Some(action) = pointer.click(&ui) else {
        return;
    };
    if action == ACTION_CLOSE {
        manager.close(WindowId::Trainer);
    } else if action == ACTION_TRAIN {
        train(&trainer, &mut actions);
    } else if let Some(index) = action
        .strip_prefix(ACTION_SERVICE_PREFIX)
        .and_then(|index| index.parse::<usize>().ok())
        && let Some(service) = trainer.visible_services().get(index)
    {
        trainer.selected = Some(service.spell_id);
    }
}

/// `ClassTrainerTrainButton_OnClick`: professions confirm first.
fn train(trainer: &TrainerState, actions: &mut TrainActions) {
    let Some(service) = trainer.selected.and_then(|spell| trainer.service(spell)) else {
        return;
    };
    if !service.profession {
        actions.requests.write(TrainerRequest {
            spell_id: service.spell_id,
        });
        return;
    }
    let name = actions
        .spells
        .as_ref()
        .and_then(|catalog| catalog.get(service.spell_id))
        .map_or_else(|| "Unknown".to_string(), |spell| spell.name.to_string());
    let has_primary = actions.professions.as_ref().is_some_and(|status| {
        status
            .lines
            .iter()
            .any(|line| profession_catalog().is_primary(line.skill_line))
    });
    // PROFESSION_CONFIRMATION1 / 2 (the |cffffd200 name colour is dropped).
    let order = if has_primary { "second" } else { "first" };
    actions.popups.push(PopupSpec {
        key: CONFIRM_PROFESSION.into(),
        text: format!(
            "You may only know two professions at any one time.  Would you like to learn {name} as your {order} one?"
        ),
        accept_label: "Accept".into(),
        cancel_label: Some("Cancel".into()),
        timeout: None,
        confirm_text: None,
    });
}

fn confirm_profession(
    mut results: MessageReader<PopupResult>,
    trainer: Res<TrainerState>,
    mut requests: MessageWriter<TrainerRequest>,
) {
    for result in results.read() {
        if result.key == CONFIRM_PROFESSION
            && result.outcome == PopupOutcome::Accepted
            && let Some(spell_id) = trainer.selected
        {
            requests.write(TrainerRequest { spell_id });
        }
    }
}

/// The wheel over the service list scrolls one row per notch.
fn handle_trainer_wheel(
    windows: Query<&Window, With<PrimaryWindow>>,
    wheel: Option<Res<AccumulatedMouseScroll>>,
    ui: Res<UiState>,
    mut trainer: ResMut<TrainerState>,
) {
    if !trainer.is_open() {
        return;
    }
    let rows = wheel_notches_over(&windows, wheel.as_deref(), &ui, LIST_NAME, 47.0);
    if rows != 0 {
        trainer.scroll_by(-rows);
    }
}

/// Wheel notches (up positive) while the cursor is over the frame `name`.
pub(crate) fn wheel_notches_over(
    windows: &Query<&Window, With<PrimaryWindow>>,
    wheel: Option<&AccumulatedMouseScroll>,
    ui: &UiState,
    name: &str,
    pixels_per_notch: f32,
) -> isize {
    let Some(wheel) = wheel.filter(|wheel| wheel.delta.y != 0.0) else {
        return 0;
    };
    let over = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window))
        .zip(frame_rect(&ui.registry, name))
        .is_some_and(|(cursor, rect)| {
            (rect.x..rect.x + rect.width).contains(&cursor.x)
                && (rect.y..rect.y + rect.height).contains(&cursor.y)
        });
    if !over {
        return 0;
    }
    match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y,
        MouseScrollUnit::Pixel => wheel.delta.y / pixels_per_notch,
    }
    .round() as isize
}

fn frame_rect(registry: &FrameRegistry, name: &str) -> Option<game_engine::ui::layout::LayoutRect> {
    let frame = registry.get(registry.get_by_name(name)?)?;
    frame.layout_rect.clone()
}

#[cfg(test)]
mod tests;
