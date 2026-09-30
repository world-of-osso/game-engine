use std::collections::HashMap;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shared::components::{
    Health as NetHealth, Npc, Player as NetPlayer, UnitFactionTemplate, UnitLevel, UnitPowers,
    UnitTarget,
};
use shared::level_scaling::{LevelDifficulty, LevelScaling, level_for_viewer};

use crate::client_options::HudVisibilityToggles;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::ui_input::walk_up_for_onclick;
use game_engine::buff_data::{AuraInstance, AuraState, UnitAuraState};
use game_engine::faction_reaction::{
    FactionTemplateEntry, Reaction, parse_faction_template_csv, reaction,
};
use game_engine::group_state::{GroupCommand, GroupMenuEntry, GroupState, group_menu_entries};
use game_engine::instance_state::{
    InstanceCatalog, InstanceCommand, InstanceState, MENU_DUNGEON_DIFFICULTIES,
    dungeon_difficulty_enabled,
};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::player_spells::ActiveSpecialization;
use game_engine::status::{CharacterStatsSnapshot, ClassBarPlayer, ClassBarResource};
use game_engine::targeting::{CurrentTarget, FocusTarget, SetFocus, apply_set_focus};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::inworld_unit_frames_component::class_bars::settled_view;
use game_engine::ui::screens::inworld_unit_frames_component::{
    ACTION_UNIT_MENU_CLEAR_FOCUS, ACTION_UNIT_MENU_DUNGEON_DIFFICULTY, ACTION_UNIT_MENU_INSPECT,
    ACTION_UNIT_MENU_SET_DUNGEON_DIFFICULTY_PREFIX, ACTION_UNIT_MENU_SET_FOCUS,
    ACTION_UNIT_MENU_TRADE, DIFFICULTY_MENU_W, DifficultyMenuEntry, DifficultyMenuState,
    InWorldUnitFramesState, MAX_BOSS_FRAMES, PowerBarState, SmallUnitFrameState, TargetAuraView,
    UNIT_MENU_W, UnitFrameMenuState, UnitFrameState, UnitMenuItem, boss_frame_name,
    difficulty_menu_height, format_value_text, fraction, inworld_unit_frames_screen,
    set_target_auras, target_level_text, unit_menu_height,
};
use ui_toolkit::screen::{Screen, SharedContext};

type UnitComponents<'a> = (
    Option<&'a NetPlayer>,
    Option<&'a NetHealth>,
    Option<&'a UnitPowers>,
    Option<&'a Npc>,
    Option<&'a Name>,
    Option<&'a UnitAuraState>,
    Option<&'a UnitLevel>,
    Option<&'a UnitFactionTemplate>,
    Option<&'a LevelScaling>,
);

const FACTION_TEMPLATE_CSV: &str = "data/db2/12.1.0.69933/FactionTemplate.csv";

/// `FactionTemplate.csv` rows by id, for target reaction colours.
#[derive(Resource, Default)]
pub(crate) struct FactionTemplates(HashMap<u32, FactionTemplateEntry>);

impl FactionTemplates {
    pub(crate) fn load() -> Self {
        let rows = std::fs::read_to_string(FACTION_TEMPLATE_CSV)
            .map_err(|err| format!("read {FACTION_TEMPLATE_CSV}: {err}"))
            .and_then(|text| parse_faction_template_csv(&text));
        match rows {
            Ok(rows) => Self(rows),
            Err(err) => {
                error!("Unit frame reactions stay neutral: {err}");
                Self::default()
            }
        }
    }

    pub(crate) fn row(
        &self,
        template: Option<&UnitFactionTemplate>,
    ) -> Option<&FactionTemplateEntry> {
        self.0.get(&template?.0)
    }
}

/// What the local player brings to a target's level text and reaction.
struct Viewer<'a> {
    level: Option<u8>,
    template: Option<&'a FactionTemplateEntry>,
}

struct InWorldUnitFramesRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for InWorldUnitFramesRes {}
unsafe impl Sync for InWorldUnitFramesRes {}

#[derive(Resource)]
struct InWorldUnitFramesWrap(InWorldUnitFramesRes);

#[derive(Resource, Clone, PartialEq)]
struct InWorldUnitFramesModel(InWorldUnitFramesState);

/// Set Focus and Clear Focus precede the player entries.
const FOCUS_MENU_ITEMS: usize = 2;
/// menu_primitives' first button top and row pitch (22 px buttons, 3 px gap).
const MENU_ROW_TOP: f32 = 28.0;
const MENU_ROW_PITCH: f32 = 25.0;

/// Right-click menu on a unit frame; `unit` is the entity the menu acts on.
#[derive(Resource, Default, Clone, PartialEq)]
struct UnitFrameMenu {
    unit: Option<Entity>,
    /// Player name of `unit`, for its group entries.
    player_name: Option<String>,
    state: UnitFrameMenuState,
}

/// Entities currently shown by each cluster frame and boss frame.
#[derive(Clone, Default)]
struct FrameUnits {
    player: Option<Entity>,
    target: Option<Entity>,
    target_of_target: Option<Entity>,
    focus: Option<Entity>,
    /// `boss1..boss5` that have a local mirror.
    bosses: Vec<Entity>,
}

impl FrameUnits {
    fn for_root(&self, root: &str) -> Option<Entity> {
        match root {
            "PlayerFrame" => self.player,
            "TargetFrame" => self.target,
            "TargetOfTargetFrame" => self.target_of_target,
            "FocusFrame" => self.focus,
            boss => (0..MAX_BOSS_FRAMES)
                .find(|&index| boss_frame_name(index) == boss)
                .and_then(|index| self.bosses.get(index).copied()),
        }
    }
}

pub struct InWorldUnitFramesPlugin;

impl Plugin for InWorldUnitFramesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusTarget>();
        app.init_resource::<UnitFrameMenu>();
        app.init_resource::<GroupState>();
        app.add_message::<GroupCommand>();
        app.add_message::<InstanceCommand>();
        app.insert_resource(FactionTemplates::load());
        app.add_message::<SetFocus>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_inworld_unit_frames_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_inworld_unit_frames_ui);
        app.add_systems(
            Update,
            (
                handle_unit_frame_pointer,
                apply_set_focus,
                sync_inworld_unit_frames_root_size,
                sync_inworld_unit_frames_ui,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct FrameUnitSources<'w, 's> {
    local_player: Query<'w, 's, Entity, With<LocalPlayer>>,
    unit_targets: Query<'w, 's, &'static UnitTarget>,
    mirror: Option<Res<'w, ReplicationMirrorMap>>,
    current_target: Res<'w, CurrentTarget>,
    focus: Res<'w, FocusTarget>,
    encounter: Option<Res<'w, crate::game::networking_encounter::EncounterFrames>>,
}

impl FrameUnitSources<'_, '_> {
    /// `UnitTarget` carries server entity bits; map them to the local mirror entity.
    fn unit_target_of(&self, entity: Entity) -> Option<Entity> {
        let bits = self.unit_targets.get(entity).ok()?.0?;
        let server = Entity::try_from_bits(bits)?;
        self.mirror.as_deref()?.server_to_main(server)
    }

    /// The unit shown by the unit frame cluster (PlayerFrame, TargetFrame,
    /// TargetOfTargetFrame, FocusFrame) that contains `frame`.
    pub(crate) fn unit_at_frame(&self, registry: &FrameRegistry, frame: u64) -> Option<Entity> {
        self.frame_units()
            .for_root(cluster_root_name(registry, frame)?)
    }

    fn frame_units(&self) -> FrameUnits {
        let target = self.current_target.0;
        let mirror = self.mirror.as_deref();
        let bosses = self
            .encounter
            .as_deref()
            .into_iter()
            .flat_map(|frames| frames.boss_units())
            .filter_map(|bits| mirror?.server_to_main(Entity::try_from_bits(bits)?))
            .collect();
        FrameUnits {
            player: self.local_player.iter().next(),
            target,
            target_of_target: target.and_then(|entity| self.unit_target_of(entity)),
            focus: self.focus.0,
            bosses,
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct UnitFrameSources<'w, 's> {
    units: FrameUnitSources<'w, 's>,
    player_query: Query<'w, 's, UnitComponents<'static>, With<LocalPlayer>>,
    entity_query: Query<'w, 's, UnitComponents<'static>>,
    character_stats: Option<Res<'w, CharacterStatsSnapshot>>,
    spec: Option<Res<'w, ActiveSpecialization>>,
    aura_state: Option<Res<'w, AuraState>>,
    menu: Res<'w, UnitFrameMenu>,
    hud_visibility: Option<Res<'w, HudVisibilityToggles>>,
    faction_templates: Res<'w, FactionTemplates>,
}

fn build_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    sources: UnitFrameSources,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&sources);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(inworld_unit_frames_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(InWorldUnitFramesWrap(InWorldUnitFramesRes {
        screen,
        shared,
    }));
    commands.insert_resource(InWorldUnitFramesModel(state));
}

fn teardown_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screen: Option<ResMut<InWorldUnitFramesWrap>>,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<InWorldUnitFramesWrap>();
    commands.remove_resource::<InWorldUnitFramesModel>();
    commands.insert_resource(UnitFrameMenu::default());
}

fn sync_inworld_unit_frames_root_size(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
}

fn sync_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut screen_wrap: Option<ResMut<InWorldUnitFramesWrap>>,
    mut last_model: Option<ResMut<InWorldUnitFramesModel>>,
    sources: UnitFrameSources,
) {
    let (Some(mut screen_wrap), Some(mut last_model)) = (screen_wrap.take(), last_model.take())
    else {
        return;
    };
    let state = build_state(&sources);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut screen_wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(sources: &UnitFrameSources) -> InWorldUnitFramesState {
    let visibility = sources
        .hud_visibility
        .as_deref()
        .cloned()
        .unwrap_or_default();
    let stats = sources.character_stats.as_deref();
    let spec = sources.spec.as_deref().and_then(|spec| spec.0);
    let units = sources.units.frame_units();
    let player = sources
        .player_query
        .iter()
        .next()
        .map(|unit| build_player_state(stats, spec, unit))
        .unwrap_or_else(|| UnitFrameState::named("Player"));
    let local = sources.player_query.iter().next();
    let viewer = Viewer {
        level: local.and_then(|unit| unit.6).map(|level| level.0),
        template: sources.faction_templates.row(local.and_then(|unit| unit.7)),
    };
    let unit_state = |entity: Option<Entity>| {
        let entity = entity?;
        let unit = sources.entity_query.get(entity).ok()?;
        Some(build_target_state(
            &viewer,
            &sources.faction_templates,
            unit,
        ))
    };
    let mut target = unit_state(units.target);
    if let (Some(target), Some(entity)) = (target.as_mut(), units.target) {
        let unit_auras = sources
            .entity_query
            .get(entity)
            .ok()
            .and_then(|unit| unit.5);
        populate_target_auras(
            target,
            Some(entity),
            units.player,
            unit_auras,
            sources.aura_state.as_deref(),
        );
    }
    InWorldUnitFramesState {
        show_player_frame: visibility.show_player_frame,
        show_target_frame: visibility.show_target_frame,
        player,
        target,
        target_of_target: unit_state(units.target_of_target)
            .as_ref()
            .map(SmallUnitFrameState::from),
        focus: unit_state(units.focus)
            .as_ref()
            .map(SmallUnitFrameState::from),
        bosses: units
            .bosses
            .iter()
            .filter_map(|&boss| unit_state(Some(boss)))
            .collect(),
        menu: sources.menu.state.clone(),
    }
}

fn build_player_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    spec: Option<u32>,
    (player, health, powers, _npc, name, _auras, level, _faction, _scaling): UnitComponents,
) -> UnitFrameState {
    let mut state = UnitFrameState::named(resolve_player_name(player, character_stats, name));
    state.level_text = level.map(|level| level.0.to_string()).unwrap_or_default();
    state.show_combat_icon = character_stats.is_some_and(|stats| stats.in_combat);
    state.show_resting_icon = character_stats.is_some_and(|stats| stats.in_rest_area);
    let class_bar_player = player.map(|player| ClassBarPlayer {
        class: player.class,
        spec,
        level: level.map_or(0, |level| level.0),
        in_combat: state.show_combat_icon,
    });
    state.class_bar = powers
        .zip(class_bar_player)
        .and_then(|(powers, player)| ClassBarResource::for_player(powers, None, &player))
        .as_ref()
        .and_then(settled_view);
    populate_resources(&mut state, health, powers);
    state
}

fn resolve_player_name(
    player: Option<&NetPlayer>,
    character_stats: Option<&CharacterStatsSnapshot>,
    name: Option<&Name>,
) -> String {
    player
        .map(|player| player.name.clone())
        .or_else(|| character_stats.and_then(|stats| stats.name.clone()))
        .or_else(|| name.map(|name| name.as_str().to_string()))
        .unwrap_or_else(|| "Player".to_string())
}

fn build_target_state(
    viewer: &Viewer,
    templates: &FactionTemplates,
    (player, health, powers, npc, name, _auras, level, faction, scaling): UnitComponents,
) -> UnitFrameState {
    let mut state = UnitFrameState::named(resolve_target_name(player, npc, name));
    // A tuned creature shows its level against you (UnitEffectiveLevel).
    let level =
        level.map(|level| level_for_viewer(*level, scaling, viewer.level.unwrap_or(level.0)));
    state.level_text = target_level_text(level, viewer.level);
    // Retail colours the target by its reaction to you; unknown templates are neutral.
    let reaction = reaction(templates.row(faction), viewer.template);
    state.reaction = Some(reaction);
    if let (Some(level), Some(player_level), true) =
        (level, viewer.level, reaction != Reaction::Friendly)
    {
        state.level_color = difficulty_color(LevelDifficulty::for_levels(player_level, level));
    }
    populate_resources(&mut state, health, powers);
    state
}

fn resolve_target_name(
    player: Option<&NetPlayer>,
    npc: Option<&Npc>,
    name: Option<&Name>,
) -> String {
    player
        .map(|player| player.name.clone())
        .or_else(|| npc.map(|npc| npc.name.clone()))
        .or_else(|| name.map(|name| name.as_str().to_string()))
        .unwrap_or_else(|| "Unknown".to_string())
}

/// TargetFrameMixin:CheckLevel colours an attackable target's level by its
/// difficulty for the player; others stay gold.
fn difficulty_color(difficulty: LevelDifficulty) -> String {
    let [r, g, b] = difficulty.color();
    format!("{r},{g},{b},1.0")
}

fn populate_resources(
    state: &mut UnitFrameState,
    health: Option<&NetHealth>,
    powers: Option<&UnitPowers>,
) {
    if let Some(health) = health {
        state.health_text = format_value_text(health.current, health.max);
        state.health_fraction = fraction(health.current, health.max);
    }
    state.power = powers.and_then(PowerBarState::primary);
}

fn handle_unit_frame_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    model: Option<Res<InWorldUnitFramesModel>>,
    sources: FrameUnitSources,
    group: GroupMenuSources,
    mut menu: ResMut<UnitFrameMenu>,
    mut set_focus: MessageWriter<SetFocus>,
    mut group_commands: MessageWriter<GroupCommand>,
    mut inspect: Option<ResMut<game_engine::inspect::InspectRuntimeState>>,
    mut trade: Option<ResMut<game_engine::trade::TradeClientState>>,
    instances: InstanceMenuSources,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let (Some(mouse), Some(model)) = (mouse, model) else {
        return;
    };
    let button = if mouse.just_pressed(MouseButton::Right) {
        MouseButton::Right
    } else if mouse.just_pressed(MouseButton::Left) {
        MouseButton::Left
    } else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let units = sources.frame_units();
    let player_name = |entity: Entity| group.players.get(entity).ok().map(|p| p.name.clone());
    let local_name = units.player.and_then(player_name);
    let default_instance = InstanceState::default();
    let click = UnitFrameClick {
        registry: &ui.registry,
        model: &model.0,
        units: &units,
        group: &group.state,
        local_name: local_name.as_deref(),
        player_name: &player_name,
        instance: instances.state.as_deref().unwrap_or(&default_instance),
        catalog: instances.catalog.as_deref(),
    };
    match click.handle(cursor, button, &mut menu) {
        Some(MenuRequest::Focus(request)) => {
            set_focus.write(request);
        }
        Some(MenuRequest::Group(command)) => {
            group_commands.write(command);
        }
        Some(MenuRequest::Inspect(unit)) => {
            if let Some(runtime) = inspect.as_deref_mut() {
                game_engine::inspect::request_query_for_target(runtime, Some(unit));
            }
        }
        Some(MenuRequest::Trade(name)) => {
            if let Some(trade) = trade.as_deref_mut() {
                trade.queue(game_engine::trade::TradeAction::Initiate(name));
            }
        }
        Some(MenuRequest::Instance(command)) => {
            let mut instances = instances;
            instances.commands.write(command);
        }
        None => {}
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct InstanceMenuSources<'w> {
    state: Option<Res<'w, InstanceState>>,
    catalog: Option<Res<'w, InstanceCatalog>>,
    commands: MessageWriter<'w, InstanceCommand>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct GroupMenuSources<'w, 's> {
    state: Res<'w, GroupState>,
    players: Query<'w, 's, &'static NetPlayer>,
}

#[derive(Debug, PartialEq)]
enum MenuRequest {
    Focus(SetFocus),
    Group(GroupCommand),
    Inspect(Entity),
    /// `InitiateTrade` with the unit's player.
    Trade(String),
    /// `SetDungeonDifficultyID`.
    Instance(InstanceCommand),
}

struct UnitFrameClick<'a> {
    registry: &'a FrameRegistry,
    model: &'a InWorldUnitFramesState,
    units: &'a FrameUnits,
    group: &'a GroupState,
    local_name: Option<&'a str>,
    player_name: &'a dyn Fn(Entity) -> Option<String>,
    instance: &'a InstanceState,
    catalog: Option<&'a InstanceCatalog>,
}

impl UnitFrameClick<'_> {
    /// Right-click on a cluster frame opens the menu for its unit; a left-click on a menu
    /// entry runs it. Any other click closes the menu.
    fn handle(
        &self,
        cursor: Vec2,
        button: MouseButton,
        menu: &mut UnitFrameMenu,
    ) -> Option<MenuRequest> {
        let frame = find_frame_at(self.registry, cursor.x, cursor.y);
        if button == MouseButton::Right {
            *menu = frame
                .and_then(|frame| self.menu_for(frame, cursor))
                .unwrap_or_default();
            return None;
        }
        let action = frame.and_then(|frame| walk_up_for_onclick(self.registry, frame));
        let request = match action.as_deref() {
            Some(ACTION_UNIT_MENU_SET_FOCUS) => {
                menu.unit.map(SetFocus::Unit).map(MenuRequest::Focus)
            }
            Some(ACTION_UNIT_MENU_CLEAR_FOCUS) => Some(MenuRequest::Focus(SetFocus::Clear)),
            Some(ACTION_UNIT_MENU_INSPECT) => menu.unit.map(MenuRequest::Inspect),
            Some(ACTION_UNIT_MENU_TRADE) => menu.player_name.clone().map(MenuRequest::Trade),
            Some(ACTION_UNIT_MENU_DUNGEON_DIFFICULTY) if menu.state.visible => {
                menu.state.difficulty_menu = Some(self.difficulty_menu(&menu.state));
                return None;
            }
            Some(action) if action.starts_with(ACTION_UNIT_MENU_SET_DUNGEON_DIFFICULTY_PREFIX) => {
                let id: Option<u32> = action
                    [ACTION_UNIT_MENU_SET_DUNGEON_DIFFICULTY_PREFIX.len()..]
                    .parse()
                    .ok();
                // A disabled radio does nothing (`IsEnabled` false).
                let enabled = menu.state.difficulty_menu.as_ref().is_some_and(|submenu| {
                    submenu
                        .entries
                        .iter()
                        .any(|entry| Some(entry.difficulty_id) == id && entry.enabled)
                });
                id.filter(|_| enabled)
                    .map(|id| MenuRequest::Instance(InstanceCommand::SetDungeonDifficulty(id)))
            }
            Some(action) => GroupMenuEntry::from_action(action)
                .zip(menu.player_name.as_deref())
                .map(|(entry, name)| MenuRequest::Group(entry.command(name))),
            None => None,
        };
        if menu.state.visible {
            *menu = UnitFrameMenu::default();
        }
        request
    }

    /// The Dungeon Difficulty submenu beside `menu`'s Dungeon Difficulty entry: Normal,
    /// Heroic and Mythic, the player's checked, enabled outside instances for a player
    /// alone or leading its group.
    fn difficulty_menu(&self, menu: &UnitFrameMenuState) -> DifficultyMenuState {
        let local = self.local_name.unwrap_or_default();
        let enabled = dungeon_difficulty_enabled(
            self.instance,
            self.group.in_group(),
            self.group.is_leader(local),
        );
        let entries: Vec<DifficultyMenuEntry> = MENU_DUNGEON_DIFFICULTIES
            .into_iter()
            .map(|difficulty_id| DifficultyMenuEntry {
                difficulty_id,
                label: self
                    .catalog
                    .map_or("", |catalog| catalog.difficulty_name(difficulty_id))
                    .to_owned(),
                checked: self.instance.dungeon_difficulty == Some(difficulty_id),
                enabled,
            })
            .collect();
        let row = FOCUS_MENU_ITEMS
            + menu
                .player_items
                .iter()
                .position(|item| item.action == ACTION_UNIT_MENU_DUNGEON_DIFFICULTY)
                .expect("the Dungeon Difficulty entry opened its submenu");
        let max_x = (self.registry.screen_width - DIFFICULTY_MENU_W).max(0.0);
        let max_y = (self.registry.screen_height - difficulty_menu_height(entries.len())).max(0.0);
        DifficultyMenuState {
            x: (menu.x + UNIT_MENU_W).clamp(0.0, max_x),
            y: (menu.y + MENU_ROW_TOP + row as f32 * MENU_ROW_PITCH).clamp(0.0, max_y),
            entries,
        }
    }

    fn menu_for(&self, frame: u64, cursor: Vec2) -> Option<UnitFrameMenu> {
        let root = cluster_root_name(self.registry, frame)?;
        let unit = self.units.for_root(root)?;
        let player_name = (self.player_name)(unit);
        let player_items = self.player_items(player_name.as_deref());
        let max_x = (self.registry.screen_width - UNIT_MENU_W).max(0.0);
        let max_y = (self.registry.screen_height - unit_menu_height(player_items.len())).max(0.0);
        Some(UnitFrameMenu {
            unit: Some(unit),
            player_name,
            state: UnitFrameMenuState {
                visible: true,
                title: self.unit_name(root),
                x: cursor.x.clamp(0.0, max_x),
                y: cursor.y.clamp(0.0, max_y),
                player_items,
                difficulty_menu: None,
            },
        })
    }

    /// Entries for a player unit (Retail `UnitPopup` SELF / PARTY / PLAYER): group entries,
    /// then Inspect and Trade for other players (`UnitPopupInspectButtonMixin`,
    /// `UnitPopupTradeButtonMixin`).
    fn player_items(&self, player_name: Option<&str>) -> Vec<UnitMenuItem> {
        let (Some(local), Some(unit)) = (self.local_name, player_name) else {
            return Vec::new();
        };
        let mut items: Vec<UnitMenuItem> = group_menu_entries(self.group, local, unit)
            .into_iter()
            .map(|entry| UnitMenuItem {
                name: format!("UnitFrameContextMenu{}", entry.frame_key()),
                label: entry.label().into(),
                action: entry.action().into(),
            })
            .collect();
        if unit == local {
            // UnitPopupMenuSelf: UnitPopupDungeonDifficultyButtonMixin.
            items.push(UnitMenuItem {
                name: "UnitFrameContextMenuDungeonDifficulty".into(),
                label: "Dungeon Difficulty".into(),
                action: ACTION_UNIT_MENU_DUNGEON_DIFFICULTY.into(),
            });
        }
        if unit != local {
            items.push(UnitMenuItem {
                name: "UnitFrameContextMenuInspect".into(),
                label: "Inspect".into(),
                action: ACTION_UNIT_MENU_INSPECT.into(),
            });
            items.push(UnitMenuItem {
                name: "UnitFrameContextMenuTrade".into(),
                label: "Trade".into(),
                action: ACTION_UNIT_MENU_TRADE.into(),
            });
        }
        items
    }

    fn unit_name(&self, root: &str) -> String {
        let model = self.model;
        match root {
            "PlayerFrame" => Some(model.player.name.clone()),
            "TargetFrame" => model.target.as_ref().map(|unit| unit.name.clone()),
            "TargetOfTargetFrame" => model
                .target_of_target
                .as_ref()
                .map(|unit| unit.name.clone()),
            "FocusFrame" => model.focus.as_ref().map(|unit| unit.name.clone()),
            _ => None,
        }
        .unwrap_or_default()
    }
}

fn cluster_root_name(registry: &FrameRegistry, mut frame: u64) -> Option<&'static str> {
    const ROOTS: [&str; 9] = [
        "PlayerFrame",
        "TargetFrame",
        "TargetOfTargetFrame",
        "FocusFrame",
        "Boss1TargetFrame",
        "Boss2TargetFrame",
        "Boss3TargetFrame",
        "Boss4TargetFrame",
        "Boss5TargetFrame",
    ];
    loop {
        let data = registry.get(frame)?;
        if let Some(root) = ROOTS
            .into_iter()
            .find(|root| data.name.as_deref() == Some(*root))
        {
            return Some(root);
        }
        frame = data.parent_id?;
    }
}

fn resolve_target_auras<'a>(
    target_entity: Option<Entity>,
    local_player_entity: Option<Entity>,
    unit_auras: Option<&'a UnitAuraState>,
    local_auras: Option<&'a AuraState>,
) -> &'a [AuraInstance] {
    if target_entity.is_some() && target_entity == local_player_entity {
        return local_auras.map_or(&[], |auras| auras.auras.as_slice());
    }
    if let Some(unit_auras) = unit_auras {
        return &unit_auras.auras;
    }
    &[]
}

fn populate_target_auras(
    state: &mut UnitFrameState,
    target_entity: Option<Entity>,
    local_player_entity: Option<Entity>,
    unit_auras: Option<&UnitAuraState>,
    local_auras: Option<&AuraState>,
) {
    let auras = resolve_target_auras(target_entity, local_player_entity, unit_auras, local_auras);
    let player_is_target = target_entity.is_some() && target_entity == local_player_entity;
    // Replicated units carry no player/NPC split here: a hostile reaction stands for a
    // hostile NPC.
    let view = TargetAuraView {
        player_is_target,
        friendly: player_is_target || state.reaction == Some(Reaction::Friendly),
        hostile_npc: !player_is_target && state.reaction == Some(Reaction::Hostile),
    };
    set_target_auras(state, auras, view);
}

#[cfg(test)]
#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod layout_test_support;

#[cfg(test)]
mod tests {
    use super::layout_test_support::compute_layout;
    use super::*;
    use game_engine::buff_data::{self, DebuffType};
    use game_engine::ui::event::EventBus;
    use game_engine::ui::frame::{Dimension, WidgetData};
    use game_engine::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::{
        AtlasArt, HEALTH_BAR, power_bar_art,
    };
    use game_engine::ui::screens::inworld_unit_frames_component::{BAR_W, reaction_color};
    use game_engine::ui::widgets::texture::{TextureData, TextureSource};
    use shared::components::{PowerEntry, PowerType};

    fn unit_frames_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.insert_state(GameState::InWorld);
        app.insert_resource(UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            focused_frame: None,
        });
        app.insert_resource(CurrentTarget::default());
        app.insert_resource(HudVisibilityToggles::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.add_plugins(InWorldUnitFramesPlugin);
        app.world_mut().spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ));
        app
    }

    // FactionTemplate.csv (build 12.1.0.69933) ids: 1 Human player, 2 Orc player,
    // 11 Stormwind guard (Faction 72), 14 Monster.
    const HUMAN_TEMPLATE: u32 = 1;
    const ORC_TEMPLATE: u32 = 2;
    const STORMWIND_GUARD_TEMPLATE: u32 = 11;
    const MONSTER_TEMPLATE: u32 = 14;

    fn spawn_local_player(app: &mut App, powers: Vec<PowerEntry>) -> Entity {
        app.world_mut()
            .spawn((
                LocalPlayer,
                NetPlayer {
                    name: "Theron".to_string(),
                    race: 1,
                    class: 2,
                    appearance: default(),
                },
                NetHealth {
                    current: 80.0,
                    max: 100.0,
                },
                UnitPowers {
                    entries: powers,
                    charged_points: Vec::new(),
                },
                UnitFactionTemplate(HUMAN_TEMPLATE),
            ))
            .id()
    }

    fn spawn_npc(app: &mut App, name: &str, level: u8) -> Entity {
        app.world_mut()
            .spawn((
                Npc {
                    template_id: 299,
                    name: name.into(),
                },
                NetHealth {
                    current: 30.0,
                    max: 120.0,
                },
                UnitLevel(level),
                UnitFactionTemplate(MONSTER_TEMPLATE),
            ))
            .id()
    }

    fn power(power: PowerType, current: i32, max: i32) -> PowerEntry {
        PowerEntry {
            power,
            current,
            max,
            partial: 0,
            regen_per_sec: 0.0,
        }
    }

    fn frame<'a>(app: &'a App, name: &str) -> &'a game_engine::ui::frame::Frame {
        let registry = &app.world().resource::<UiState>().registry;
        registry
            .get(registry.get_by_name(name).expect(name))
            .expect(name)
    }

    fn app_frame_missing(app: &App, name: &str) -> bool {
        app.world()
            .resource::<UiState>()
            .registry
            .get_by_name(name)
            .is_none()
    }

    fn text(app: &App, name: &str) -> String {
        match frame(app, name).widget_data.as_ref() {
            Some(WidgetData::FontString(text)) => text.text.clone(),
            _ => panic!("{name} is not a FontString"),
        }
    }

    fn texture<'a>(app: &'a App, name: &str) -> &'a TextureData {
        match frame(app, name).widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => texture,
            _ => panic!("{name} is not a Texture"),
        }
    }

    /// The bar texture shows `art`, revealed up to `fraction` of its width.
    fn assert_bar_art(app: &App, name: &str, art: &AtlasArt, fraction: f32) {
        let fill = texture(app, name);
        assert_eq!(fill.source, TextureSource::FileDataId(art.fdid), "{name}");
        assert_eq!(fill.tex_coords, rgba(&art.tex_coords(fraction)), "{name}");
    }

    fn rgba(color: &str) -> [f32; 4] {
        let parts: Vec<f32> = color.split(',').map(|part| part.parse().unwrap()).collect();
        [parts[0], parts[1], parts[2], parts[3]]
    }

    fn layout(app: &mut App) {
        compute_layout(&mut app.world_mut().resource_mut::<UiState>().registry);
    }

    fn click(app: &mut App, name: &str, button: MouseButton) {
        layout(app);
        let rect = frame(app, name).layout_rect.clone().expect(name);
        let centre = Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let mut windows = app
            .world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>();
        windows
            .single_mut(app.world_mut())
            .unwrap()
            .set_cursor_position(Some(centre));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .reset_all();
    }

    #[test]
    fn rage_raw_units_show_as_display_units_with_rage_colour() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, vec![power(PowerType::Rage, 350, 1000)]);
        app.update();

        assert_eq!(text(&app, "PlayerManaBarText"), "35 / 100");
        let fill = frame(&app, "PlayerManaBarFill");
        assert_eq!(fill.width, Dimension::Fixed(BAR_W * 0.35));
        let rage = power_bar_art(PowerType::Rage).expect("rage bar art");
        // UI-HUD-UnitFrame-Player-PortraitOff-Bar-Rage in interface/hud/uiunitframe.blp
        assert_eq!(rage.fdid, 4_631_591);
        assert_bar_art(&app, "PlayerManaBarFill", &rage, 0.35);
    }

    #[test]
    fn holy_power_lights_one_pip_per_point() {
        let mut app = unit_frames_app();
        spawn_local_player(
            &mut app,
            vec![
                power(PowerType::Mana, 5000, 10000),
                power(PowerType::HolyPower, 3, 5),
            ],
        );
        app.update();

        let lit = |index: usize| !frame(&app, &format!("PlayerSecondaryResourcePip{index}")).hidden;
        assert_eq!(
            [lit(0), lit(1), lit(2), lit(3), lit(4)],
            [true, true, true, false, false]
        );
        assert!(!frame(&app, "PlayerSecondaryResourceHolder").hidden);
        assert!(
            app.world()
                .resource::<UiState>()
                .registry
                .get_by_name("PlayerSecondaryResourcePip5")
                .is_none()
        );
        assert_eq!(text(&app, "PlayerManaBarText"), "5000 / 10000");
    }

    #[test]
    fn combo_points_light_the_retail_point_icon_over_each_slot() {
        let mut app = unit_frames_app();
        spawn_local_player(
            &mut app,
            vec![
                power(PowerType::Energy, 60, 100),
                power(PowerType::ComboPoints, 2, 5),
            ],
        );
        app.update();

        let shown = |part: &str, index: usize| {
            !frame(&app, &format!("PlayerSecondaryResourcePip{index}{part}")).hidden
        };
        assert_eq!(
            (0..5).map(|index| shown("Lit", index)).collect::<Vec<_>>(),
            [true, true, false, false, false]
        );
        assert!((0..5).all(|index| shown("Background", index)));
        let rogue_points = 4_902_605; // interface/hud/uiroguecombpoints.blp
        assert_eq!(
            texture(&app, "PlayerSecondaryResourcePip0Lit").source,
            TextureSource::FileDataId(rogue_points)
        );
        assert_bar_art(
            &app,
            "PlayerManaBarFill",
            &power_bar_art(PowerType::Energy).unwrap(),
            0.6,
        );
    }

    #[test]
    fn target_npc_shows_replicated_name_level_and_hostile_reaction() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let wolf = spawn_npc(&mut app, "Timber Wolf", 7);
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(wolf);
        app.update();

        assert!(!frame(&app, "TargetFrame").hidden);
        assert_eq!(text(&app, "TargetName"), "Timber Wolf");
        assert_eq!(text(&app, "TargetLevelText"), "7");
        assert_eq!(text(&app, "TargetHealthBarText"), "30 / 120");
        assert_eq!(
            texture(&app, "TargetReputationColor").vertex_color,
            rgba(reaction_color(Reaction::Hostile))
        );
        assert_bar_art(&app, "TargetHealthBarFill", &HEALTH_BAR, 0.25);
        assert!(frame(&app, "TargetManaBar").hidden, "NPC has no powers");
    }

    fn target_reaction_color(app: &mut App, unit: Entity) -> [f32; 4] {
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(unit);
        app.update();
        texture(app, "TargetReputationColor").vertex_color
    }

    #[test]
    fn target_reaction_follows_faction_templates_toward_local_player() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let guard = spawn_npc(&mut app, "Stormwind City Guard", 30);
        app.world_mut()
            .entity_mut(guard)
            .insert(UnitFactionTemplate(STORMWIND_GUARD_TEMPLATE));
        let orc = app
            .world_mut()
            .spawn((
                NetPlayer {
                    name: "Grommash".into(),
                    race: 2,
                    class: 1,
                    appearance: default(),
                },
                NetHealth {
                    current: 50.0,
                    max: 100.0,
                },
                UnitFactionTemplate(ORC_TEMPLATE),
            ))
            .id();
        let untagged = spawn_npc(&mut app, "Old Critter", 1);
        app.world_mut()
            .entity_mut(untagged)
            .remove::<UnitFactionTemplate>();
        app.update();

        let color = |reaction| rgba(reaction_color(reaction));
        assert_eq!(
            target_reaction_color(&mut app, guard),
            color(Reaction::Friendly)
        );
        assert_eq!(
            target_reaction_color(&mut app, orc),
            color(Reaction::Hostile)
        );
        assert_eq!(
            target_reaction_color(&mut app, untagged),
            color(Reaction::Neutral)
        );
    }

    #[test]
    fn target_ten_levels_above_player_shows_question_marks() {
        let mut app = unit_frames_app();
        let player = spawn_local_player(&mut app, Vec::new());
        app.world_mut().entity_mut(player).insert(UnitLevel(70));
        let boss = spawn_npc(&mut app, "Onyxia", 90);
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(boss);
        app.update();

        assert_eq!(text(&app, "PlayerLevelText"), "70");
        assert_eq!(text(&app, "TargetLevelText"), "??");
    }

    fn text_color(app: &App, name: &str) -> [f32; 4] {
        match frame(app, name).widget_data.as_ref() {
            Some(WidgetData::FontString(text)) => text.color,
            _ => panic!("{name} is not a FontString"),
        }
    }

    /// Hogger's ContentTuning 73 range, native level 30.
    fn spawn_hogger(app: &mut App) -> Entity {
        let hogger = spawn_npc(app, "Hogger", 30);
        app.world_mut().entity_mut(hogger).insert(LevelScaling {
            content_tuning_id: 73,
            min_level: 1,
            max_level: 30,
            delta: 0,
        });
        hogger
    }

    fn target_level(app: &mut App, player_level: u8, unit: Entity) -> (String, [f32; 4]) {
        let mut players = app
            .world_mut()
            .query_filtered::<Entity, With<LocalPlayer>>();
        let player = players.single(app.world()).unwrap();
        app.world_mut()
            .entity_mut(player)
            .insert(UnitLevel(player_level));
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(unit);
        app.update();
        (
            text(app, "TargetLevelText"),
            text_color(app, "TargetLevelText"),
        )
    }

    #[test]
    fn tuned_target_shows_its_level_for_the_local_player_in_difficulty_colour() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let hogger = spawn_hogger(&mut app);
        app.update();
        let yellow = [1.0, 0.82, 0.0, 1.0];
        assert_eq!(target_level(&mut app, 10, hogger), ("10".into(), yellow));
        assert_eq!(target_level(&mut app, 25, hogger), ("25".into(), yellow));
        assert_eq!(
            target_level(&mut app, 40, hogger),
            ("30".into(), [0.5, 0.5, 0.5, 1.0]),
            "capped at 30 and grey to a level 40 player"
        );
    }

    #[test]
    fn hostile_target_level_colour_follows_the_level_difference() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let ogre = spawn_npc(&mut app, "Ogre", 15);
        let spider = spawn_npc(&mut app, "Forest Spider", 13);
        let wolf = spawn_npc(&mut app, "Timber Wolf", 5);
        app.update();
        assert_eq!(target_level(&mut app, 10, ogre).1, [1.0, 0.1, 0.1, 1.0]);
        assert_eq!(target_level(&mut app, 10, spider).1, [1.0, 0.5, 0.25, 1.0]);
        assert_eq!(target_level(&mut app, 10, wolf).1, [0.25, 0.75, 0.25, 1.0]);
    }

    #[test]
    fn friendly_target_level_stays_gold() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let guard = spawn_npc(&mut app, "Stormwind City Guard", 90);
        app.world_mut()
            .entity_mut(guard)
            .insert(UnitFactionTemplate(STORMWIND_GUARD_TEMPLATE));
        app.update();
        assert_eq!(target_level(&mut app, 85, guard).1, [1.0, 0.82, 0.0, 1.0]);
    }

    #[test]
    fn set_focus_message_shows_focus_frame_with_unit_name() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let wolf = spawn_npc(&mut app, "Timber Wolf", 7);
        app.update();
        assert!(frame(&app, "FocusFrame").hidden);

        app.world_mut().write_message(SetFocus::Unit(wolf));
        app.update();

        assert!(!frame(&app, "FocusFrame").hidden);
        assert_eq!(text(&app, "FocusName"), "Timber Wolf");

        app.world_mut().write_message(SetFocus::Clear);
        app.update();
        assert!(frame(&app, "FocusFrame").hidden);
    }

    #[test]
    fn right_click_target_frame_set_focus_focuses_target() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let wolf = spawn_npc(&mut app, "Timber Wolf", 7);
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(wolf);
        app.update();

        click(&mut app, "TargetFrame", MouseButton::Right);
        app.update();
        assert!(!frame(&app, "UnitFrameContextMenu").hidden);
        assert_eq!(text(&app, "UnitFrameContextMenuTitle"), "Timber Wolf");

        click(&mut app, "UnitFrameContextMenuSetFocus", MouseButton::Left);
        app.update();
        assert_eq!(app.world().resource::<FocusTarget>().0, Some(wolf));
        assert!(frame(&app, "UnitFrameContextMenu").hidden);
        assert_eq!(text(&app, "FocusName"), "Timber Wolf");
    }

    #[test]
    fn right_click_player_target_invites_it_and_player_frame_offers_leave() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let valeera = app
            .world_mut()
            .spawn((
                NetPlayer {
                    name: "Valeera".to_string(),
                    race: 4,
                    class: 4,
                    appearance: default(),
                },
                NetHealth {
                    current: 90.0,
                    max: 90.0,
                },
            ))
            .id();
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(valeera);
        app.update();

        click(&mut app, "TargetFrame", MouseButton::Right);
        app.update();
        assert!(!frame(&app, "UnitFrameContextMenuInvite").hidden);
        click(&mut app, "UnitFrameContextMenuInvite", MouseButton::Left);
        app.update();

        let sent: Vec<GroupCommand> = app
            .world_mut()
            .resource_mut::<Messages<GroupCommand>>()
            .drain()
            .collect();
        assert_eq!(sent, [GroupCommand::Invite("Valeera".into())]);
        assert!(frame(&app, "UnitFrameContextMenu").hidden);

        app.world_mut().resource_mut::<GroupState>().apply_roster(
            shared::protocol::GroupRosterSnapshot {
                is_raid: false,
                ready_count: 0,
                total_count: 0,
                members: ["Theron", "Valeera"]
                    .into_iter()
                    .map(|name| shared::protocol::GroupMemberSnapshot {
                        name: name.into(),
                        role: shared::protocol::GroupRoleSnapshot::None,
                        is_leader: name == "Theron",
                        online: true,
                        subgroup: 1,
                        class: 2,
                        level: 10,
                        entity: None,
                    })
                    .collect(),
                loot_method: shared::loot::LootMode::PersonalLoot,
            },
        );
        click(&mut app, "PlayerFrame", MouseButton::Right);
        app.update();
        assert!(!frame(&app, "UnitFrameContextMenuLeave").hidden);
        assert!(!frame(&app, "UnitFrameContextMenuConvertToRaid").hidden);
    }

    fn radio_coords(app: &App, name: &str) -> [f32; 4] {
        match frame(app, name).widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => texture.tex_coords,
            _ => panic!("{name} is not a Texture"),
        }
    }

    #[test]
    fn the_player_menu_dungeon_difficulty_submenu_picks_heroic_outside_instances() {
        let mut app = unit_frames_app();
        app.insert_resource(InstanceCatalog::load().unwrap());
        app.insert_resource(InstanceState {
            dungeon_difficulty: Some(1),
            current_map: Some((0, 0)),
            ..Default::default()
        });
        spawn_local_player(&mut app, Vec::new());
        app.update();

        click(&mut app, "PlayerFrame", MouseButton::Right);
        app.update();
        assert!(frame(&app, "UnitFrameDifficultyMenu").hidden);
        click(
            &mut app,
            "UnitFrameContextMenuDungeonDifficulty",
            MouseButton::Left,
        );
        app.update();
        assert!(!frame(&app, "UnitFrameDifficultyMenu").hidden);
        assert_eq!(text(&app, "UnitFrameDifficultyMenu1Text"), "Normal");
        assert_eq!(text(&app, "UnitFrameDifficultyMenu2Text"), "Heroic");
        assert_eq!(text(&app, "UnitFrameDifficultyMenu23Text"), "Mythic");
        // Normal is checked: the yellow radial tick (common-dropdown-icon-radialtick-yellow).
        assert_ne!(
            radio_coords(&app, "UnitFrameDifficultyMenu1Radio"),
            radio_coords(&app, "UnitFrameDifficultyMenu2Radio")
        );

        click(&mut app, "UnitFrameDifficultyMenu2", MouseButton::Left);
        app.update();
        let sent: Vec<InstanceCommand> = app
            .world_mut()
            .resource_mut::<Messages<InstanceCommand>>()
            .drain()
            .collect();
        assert_eq!(sent, [InstanceCommand::SetDungeonDifficulty(2)]);
        assert!(frame(&app, "UnitFrameContextMenu").hidden);
        assert!(frame(&app, "UnitFrameDifficultyMenu").hidden);

        // Inside a Heroic copy the radios are disabled: grey, no command.
        app.world_mut().resource_mut::<InstanceState>().current_map = Some((670, 2));
        click(&mut app, "PlayerFrame", MouseButton::Right);
        app.update();
        click(
            &mut app,
            "UnitFrameContextMenuDungeonDifficulty",
            MouseButton::Left,
        );
        app.update();
        assert_eq!(
            text_color(&app, "UnitFrameDifficultyMenu23Text"),
            [0.5, 0.5, 0.5, 1.0]
        );
        click(&mut app, "UnitFrameDifficultyMenu23", MouseButton::Left);
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<Messages<InstanceCommand>>()
                .drain()
                .next()
                .is_none()
        );
    }

    #[test]
    fn targeting_a_player_never_inspects_but_the_target_menu_inspect_entry_does() {
        use game_engine::inspect::{InspectPlugin, InspectRuntimeState};
        use game_engine::network_runtime::messages::ConnectionSender;

        let mut app = unit_frames_app();
        let (commands, sent_commands) = std::sync::mpsc::channel();
        app.insert_resource(ConnectionSender::new(Some(commands)))
            .init_resource::<ReplicationMirrorMap>()
            .init_resource::<game_engine::status::InspectStatusSnapshot>()
            .add_plugins(InspectPlugin);
        spawn_local_player(&mut app, Vec::new());
        let valeera = app
            .world_mut()
            .spawn(NetPlayer {
                name: "Valeera".to_string(),
                race: 4,
                class: 4,
                appearance: default(),
            })
            .id();
        let server_valeera = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<ReplicationMirrorMap>()
            .insert(server_valeera, valeera);
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(valeera);
        app.update();
        app.update();
        game_engine::network_events::dispatch_outgoing(app.world_mut());
        assert!(
            !app.world()
                .resource::<InspectRuntimeState>()
                .pending_query()
        );
        assert!(
            sent_commands.try_recv().is_err(),
            "targeting sent an inspect query"
        );

        click(&mut app, "PlayerFrame", MouseButton::Right);
        app.update();
        let registry = &app.world().resource::<UiState>().registry;
        assert!(
            registry
                .get_by_name("UnitFrameContextMenuInspect")
                .is_none(),
            "no Inspect on self"
        );

        click(&mut app, "TargetFrame", MouseButton::Right);
        app.update();
        assert!(!frame(&app, "UnitFrameContextMenuInspect").hidden);
        click(&mut app, "UnitFrameContextMenuInspect", MouseButton::Left);
        assert!(
            app.world()
                .resource::<InspectRuntimeState>()
                .pending_query()
        );
        game_engine::network_events::dispatch_outgoing(app.world_mut());
        assert!(
            sent_commands.try_recv().is_ok(),
            "menu Inspect sent no query"
        );
        assert!(frame(&app, "UnitFrameContextMenu").hidden);
    }

    #[test]
    fn the_target_menu_trade_entry_asks_the_player_to_trade() {
        use game_engine::trade::{TradeAction, TradeClientState};

        let mut app = unit_frames_app();
        app.init_resource::<TradeClientState>();
        spawn_local_player(&mut app, Vec::new());
        let valeera = app
            .world_mut()
            .spawn(NetPlayer {
                name: "Valeera".to_string(),
                race: 4,
                class: 4,
                appearance: default(),
            })
            .id();
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(valeera);
        app.update();
        app.update();

        click(&mut app, "PlayerFrame", MouseButton::Right);
        app.update();
        assert!(
            app.world()
                .resource::<UiState>()
                .registry
                .get_by_name("UnitFrameContextMenuTrade")
                .is_none(),
            "no Trade on self"
        );
        click(&mut app, "TargetFrame", MouseButton::Right);
        app.update();
        click(&mut app, "UnitFrameContextMenuTrade", MouseButton::Left);

        let queued = app
            .world_mut()
            .resource_mut::<TradeClientState>()
            .take_queued();
        assert_eq!(queued, vec![TradeAction::Initiate("Valeera".into())]);
    }

    #[test]
    fn target_of_target_frame_follows_target_unit_target() {
        let mut app = unit_frames_app();
        let player = spawn_local_player(&mut app, Vec::new());
        let wolf = spawn_npc(&mut app, "Timber Wolf", 7);
        let server_player = Entity::from_bits(0x0000_0001_0000_002A);
        let mut mirror = ReplicationMirrorMap::default();
        mirror.insert(server_player, player);
        app.insert_resource(mirror);
        app.world_mut()
            .entity_mut(wolf)
            .insert(UnitTarget(Some(server_player.to_bits())));
        app.update();
        assert!(frame(&app, "TargetOfTargetFrame").hidden);

        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(wolf);
        app.update();
        assert!(!frame(&app, "TargetOfTargetFrame").hidden);
        assert_eq!(text(&app, "TargetOfTargetName"), "Theron");
    }

    #[test]
    fn target_frame_unhides_for_self_target_and_obeys_hud_toggle() {
        let mut app = unit_frames_app();
        let player = spawn_local_player(&mut app, Vec::new());
        app.update();
        assert!(frame(&app, "TargetFrame").hidden);

        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(player);
        app.update();
        assert!(!frame(&app, "TargetFrame").hidden);
        assert_eq!(text(&app, "TargetName"), "Theron");

        app.world_mut()
            .resource_mut::<HudVisibilityToggles>()
            .show_target_frame = false;
        app.update();
        assert!(frame(&app, "TargetFrame").hidden);
    }

    #[test]
    fn target_icons_use_unit_aura_component_with_dispel_borders() {
        let mut app = unit_frames_app();
        spawn_local_player(&mut app, Vec::new());
        let wolf = spawn_npc(&mut app, "Timber Wolf", 7);
        app.world_mut().entity_mut(wolf).insert(UnitAuraState {
            auras: vec![buff_data::AuraInstance {
                instance_id: 2,
                spell_id: 2,
                name: "Poison".into(),
                description: String::new(),
                icon_fdid: 136067,
                source: "Rogue".into(),
                from_local_player: false,
                from_player: false,
                duration: 12.0,
                remaining: 6.2,
                stacks: 3,
                is_debuff: true,
                debuff_type: DebuffType::Poison,
            }],
        });
        app.update();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(wolf);
        app.update();

        assert!(!frame(&app, "TargetDebuffIcon0").hidden);
        assert!(app_frame_missing(&app, "TargetBuffIcon0"));
        assert_eq!(text(&app, "TargetDebuffIcon0Count"), "3");
        assert!(!app_frame_missing(&app, "TargetDebuffIcon0Border"));
    }
}
