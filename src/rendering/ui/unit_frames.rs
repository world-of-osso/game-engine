use bevy::prelude::*;
use shared::components::{Health as NetHealth, Mana as NetMana, Npc, Player as NetPlayer};

use crate::client_options::HudVisibilityToggles;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use game_engine::buff_data::{AuraInstance, AuraState, UnitAuraState};
use game_engine::char_create_data::class_by_id;
use game_engine::status::{CharacterStatsSnapshot, RestAreaKindEntry};
use game_engine::targeting::CurrentTarget;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::inworld_unit_frames_component::{
    InWorldUnitFramesState, PLAYER_HEALTH_BAR_W, TARGET_HEALTH_BAR_W, TARGET_MANA_BAR_W,
    TargetAuraIconState, UNKNOWN_PORTRAIT_TEXTURE_FILE, UnitFrameState, default_player_frame_state,
    fallback_target_frame_state, fill_width, format_value_text, inworld_unit_frames_screen,
    missing_target_name,
};
use ui_toolkit::screen::{Screen, SharedContext};

type UnitComponents<'a> = (
    Option<&'a NetPlayer>,
    Option<&'a NetHealth>,
    Option<&'a NetMana>,
    Option<&'a Npc>,
    Option<&'a Name>,
    Option<&'a UnitAuraState>,
);

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

pub struct InWorldUnitFramesPlugin;

impl Plugin for InWorldUnitFramesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_inworld_unit_frames_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_inworld_unit_frames_ui);
        app.add_systems(
            Update,
            (
                sync_inworld_unit_frames_root_size,
                sync_inworld_unit_frames_ui,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    player_query: Query<(Entity, UnitComponents), With<LocalPlayer>>,
    entity_query: Query<UnitComponents>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    aura_state: Option<Res<AuraState>>,
    current_target: Res<CurrentTarget>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(
        character_stats.as_deref(),
        aura_state.as_deref(),
        &current_target,
        &player_query,
        &entity_query,
        hud_visibility.as_deref(),
    );
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
}

fn sync_inworld_unit_frames_root_size(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
}

fn sync_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut screen_wrap: Option<ResMut<InWorldUnitFramesWrap>>,
    mut last_model: Option<ResMut<InWorldUnitFramesModel>>,
    player_query: Query<(Entity, UnitComponents), With<LocalPlayer>>,
    entity_query: Query<UnitComponents>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    aura_state: Option<Res<AuraState>>,
    current_target: Res<CurrentTarget>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
) {
    let (Some(mut screen_wrap), Some(mut last_model)) = (screen_wrap.take(), last_model.take())
    else {
        return;
    };
    let state = build_state(
        character_stats.as_deref(),
        aura_state.as_deref(),
        &current_target,
        &player_query,
        &entity_query,
        hud_visibility.as_deref(),
    );
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut screen_wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    aura_state: Option<&AuraState>,
    current_target: &CurrentTarget,
    player_query: &Query<(Entity, UnitComponents), With<LocalPlayer>>,
    entity_query: &Query<UnitComponents>,
    hud_visibility: Option<&HudVisibilityToggles>,
) -> InWorldUnitFramesState {
    let visibility = hud_visibility.cloned().unwrap_or_default();
    let local_player = player_query
        .iter()
        .next()
        .map(|(entity, unit)| (entity, build_player_state(character_stats, unit)));
    let player = local_player
        .as_ref()
        .map(|(_, state)| state.clone())
        .unwrap_or_else(default_player_frame_state);
    let target = current_target
        .0
        .and_then(|entity| entity_query.get(entity).ok())
        .map(|unit| {
            build_target_state(
                current_target.0,
                local_player.as_ref().map(|(entity, _)| *entity),
                unit,
                aura_state,
            )
        });
    InWorldUnitFramesState {
        show_player_frame: visibility.show_player_frame,
        show_target_frame: visibility.show_target_frame,
        player,
        target,
    }
}

fn build_player_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    (player, health, mana, _npc, name, _auras): UnitComponents,
) -> UnitFrameState {
    let mut state = default_player_frame_state();
    state.portrait_texture_file = portrait_texture_for_player(player, character_stats);
    state.secondary_resource = character_stats.and_then(|stats| stats.secondary_resource.clone());
    state.name = player
        .map(|player| player.name.clone())
        .or_else(|| character_stats.and_then(|stats| stats.name.clone()))
        .or_else(|| name.map(|name| name.as_str().to_string()))
        .unwrap_or_else(|| "Player".to_string());
    state.level_text = character_stats
        .and_then(|stats| stats.level)
        .map(|level| level.to_string())
        .unwrap_or_default();
    state.resting_text = character_stats.map(resting_text).unwrap_or_default();
    state.health_text = format_value_text(
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_text = format_value_text(mana.map(|mana| mana.current), mana.map(|mana| mana.max));
    state.health_fill_width = fill_width(
        PLAYER_HEALTH_BAR_W,
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_fill_width = fill_width(
        PLAYER_HEALTH_BAR_W,
        mana.map(|mana| mana.current),
        mana.map(|mana| mana.max),
    );
    state.has_mana = mana.is_some();
    state.show_combat_icon = character_stats.is_some_and(|stats| stats.in_combat);
    state.show_resting_icon = character_stats.is_some_and(|stats| stats.in_rest_area);
    state
}

fn build_target_state(
    target_entity: Option<Entity>,
    local_player_entity: Option<Entity>,
    (player, health, mana, npc, name, unit_auras): UnitComponents,
    local_auras: Option<&AuraState>,
) -> UnitFrameState {
    let mut state = fallback_target_frame_state();
    state.portrait_texture_file = portrait_texture_for_target(player);
    state.name = player
        .map(|player| player.name.clone())
        .or_else(|| npc.map(|npc| format!("Creature {}", npc.template_id)))
        .or_else(|| name.map(|name| name.as_str().to_string()))
        .unwrap_or_else(|| missing_target_name().to_string());
    state.health_text = format_value_text(
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_text = format_value_text(mana.map(|mana| mana.current), mana.map(|mana| mana.max));
    state.health_fill_width = fill_width(
        TARGET_HEALTH_BAR_W,
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_fill_width = fill_width(
        TARGET_MANA_BAR_W,
        mana.map(|mana| mana.current),
        mana.map(|mana| mana.max),
    );
    state.has_mana = mana.is_some();
    populate_target_auras(
        &mut state,
        target_entity,
        local_player_entity,
        unit_auras,
        local_auras,
    );
    state
}

fn portrait_texture_for_player(
    player: Option<&NetPlayer>,
    character_stats: Option<&CharacterStatsSnapshot>,
) -> String {
    let class_id = player
        .map(|player| player.class)
        .or_else(|| character_stats.and_then(|stats| stats.class));
    portrait_texture_for_class(class_id)
}

fn portrait_texture_for_target(player: Option<&NetPlayer>) -> String {
    portrait_texture_for_class(player.map(|player| player.class))
}

fn portrait_texture_for_class(class_id: Option<u8>) -> String {
    class_id
        .and_then(class_by_id)
        .map(|class| class.icon_file.to_string())
        .unwrap_or_else(|| UNKNOWN_PORTRAIT_TEXTURE_FILE.to_string())
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
    state.target_buffs = auras
        .iter()
        .filter(|aura| !aura.is_debuff)
        .take(6)
        .map(target_aura_icon)
        .collect();
    state.target_debuffs = auras
        .iter()
        .filter(|aura| aura.is_debuff)
        .take(6)
        .map(target_aura_icon)
        .collect();
}

fn target_aura_icon(aura: &AuraInstance) -> TargetAuraIconState {
    let border_color = if aura.is_debuff {
        aura.debuff_type.border_color().to_string()
    } else {
        "0.85,0.75,0.35,1.0".to_string()
    };
    TargetAuraIconState {
        icon_fdid: aura.icon_fdid,
        timer_text: aura.timer_text(),
        stacks: aura.stacks,
        border_color,
    }
}

fn resting_text(stats: &CharacterStatsSnapshot) -> String {
    if stats.in_rest_area {
        return "Resting".into();
    }
    if stats.rested_xp > 0 {
        return match stats.rest_area_kind {
            Some(RestAreaKindEntry::City) => "Rested (city)".into(),
            Some(RestAreaKindEntry::Inn) => "Rested (inn)".into(),
            None => "Rested".into(),
        };
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::window::PrimaryWindow;
    use game_engine::buff_data::{self, DebuffType, UnitAuraState, textures};
    use game_engine::targeting::CurrentTarget;
    use game_engine::ui::plugin::UiState;
    use game_engine::ui::{event::EventBus, registry::FrameRegistry};

    #[test]
    fn target_state_uses_player_name_when_available() {
        let player = NetPlayer {
            name: "Thrall".to_string(),
            race: 0,
            class: 7,
            appearance: default(),
        };
        let state = build_target_state(
            None,
            None,
            (Some(&player), None, None, None, None, None),
            None,
        );
        assert_eq!(state.name, "Thrall");
        assert!(
            state
                .portrait_texture_file
                .ends_with("ClassIcon_Shaman.blp")
        );
    }

    #[test]
    fn target_state_falls_back_to_npc_template_label() {
        let npc = Npc { template_id: 42 };
        let state =
            build_target_state(None, None, (None, None, None, Some(&npc), None, None), None);
        assert_eq!(state.name, "Creature 42");
        assert_eq!(state.portrait_texture_file, UNKNOWN_PORTRAIT_TEXTURE_FILE);
    }

    #[test]
    fn player_state_uses_class_icon_from_character_stats() {
        let stats = CharacterStatsSnapshot {
            class: Some(2),
            ..CharacterStatsSnapshot::default()
        };
        let state = build_player_state(Some(&stats), (None, None, None, None, None, None));
        assert!(
            state
                .portrait_texture_file
                .ends_with("ClassIcon_Paladin.blp")
        );
    }

    #[test]
    fn player_state_uses_secondary_resource_from_character_stats() {
        let stats = CharacterStatsSnapshot {
            class: Some(2),
            secondary_resource: Some(game_engine::status::SecondaryResourceEntry {
                kind: game_engine::status::SecondaryResourceKindEntry::HolyPower,
                current: 3,
                max: 5,
            }),
            ..CharacterStatsSnapshot::default()
        };
        let state = build_player_state(Some(&stats), (None, None, None, None, None, None));
        assert_eq!(
            state.secondary_resource,
            Some(game_engine::status::SecondaryResourceEntry {
                kind: game_engine::status::SecondaryResourceKindEntry::HolyPower,
                current: 3,
                max: 5,
            })
        );
    }

    #[test]
    fn target_state_uses_unit_aura_component_for_target_icons() {
        let name = Name::new("Target");
        let auras = UnitAuraState {
            auras: vec![
                buff_data::AuraInstance {
                    spell_id: 1,
                    name: "Fortitude".into(),
                    description: String::new(),
                    icon_fdid: textures::FORTITUDE,
                    source: "Priest".into(),
                    duration: 120.0,
                    remaining: 25.2,
                    stacks: 1,
                    is_debuff: false,
                    debuff_type: DebuffType::None,
                },
                buff_data::AuraInstance {
                    spell_id: 2,
                    name: "Pain".into(),
                    description: String::new(),
                    icon_fdid: textures::SHADOW_WORD_PAIN,
                    source: "Priest".into(),
                    duration: 18.0,
                    remaining: 4.4,
                    stacks: 3,
                    is_debuff: true,
                    debuff_type: DebuffType::Magic,
                },
            ],
        };

        let state = build_target_state(
            None,
            None,
            (None, None, None, None, Some(&name), Some(&auras)),
            None,
        );

        assert_eq!(state.target_buffs.len(), 1);
        assert_eq!(state.target_buffs[0].icon_fdid, textures::FORTITUDE);
        assert_eq!(state.target_buffs[0].timer_text, "26s");
        assert_eq!(state.target_debuffs.len(), 1);
        assert_eq!(
            state.target_debuffs[0].icon_fdid,
            textures::SHADOW_WORD_PAIN
        );
        assert_eq!(state.target_debuffs[0].stacks, 3);
        assert_eq!(
            state.target_debuffs[0].border_color,
            DebuffType::Magic.border_color()
        );
    }

    #[test]
    fn target_state_uses_local_aura_state_when_targeting_self() {
        let local_auras = AuraState {
            auras: vec![buff_data::AuraInstance {
                spell_id: 3,
                name: "Mark".into(),
                description: String::new(),
                icon_fdid: textures::MARK_OF_WILD,
                source: "Druid".into(),
                duration: 3600.0,
                remaining: 3600.0,
                stacks: 1,
                is_debuff: false,
                debuff_type: DebuffType::None,
            }],
        };

        let state = build_target_state(
            Some(Entity::from_bits(1)),
            Some(Entity::from_bits(1)),
            (None, None, None, None, None, None),
            Some(&local_auras),
        );

        assert_eq!(state.target_buffs.len(), 1);
        assert_eq!(state.target_buffs[0].icon_fdid, textures::MARK_OF_WILD);
        assert!(state.target_debuffs.is_empty());
    }

    #[test]
    fn player_state_shows_combat_icon_when_snapshot_is_in_combat() {
        let player = NetPlayer {
            name: "Thrall".to_string(),
            race: 0,
            class: 0,
            appearance: default(),
        };
        let health = NetHealth {
            current: 100.0,
            max: 100.0,
        };
        let stats = CharacterStatsSnapshot {
            in_combat: true,
            ..Default::default()
        };

        let state = build_player_state(
            Some(&stats),
            (Some(&player), Some(&health), None, None, None, None),
        );

        assert!(state.show_combat_icon);
    }

    #[test]
    fn inworld_target_frame_unhides_for_self_target() {
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
        app.add_plugins(InWorldUnitFramesPlugin);
        let player = app
            .world_mut()
            .spawn((
                LocalPlayer,
                NetPlayer {
                    name: "Theron".to_string(),
                    race: 0,
                    class: 0,
                    appearance: default(),
                },
                NetHealth {
                    current: 100.0,
                    max: 100.0,
                },
                Name::new("Theron"),
                UnitAuraState::default(),
            ))
            .id();
        app.world_mut().spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ));

        app.update();
        assert!(
            target_frame_hidden(&app),
            "target frame should start hidden"
        );

        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(player);
        app.update();

        assert!(
            !target_frame_hidden(&app),
            "target frame should unhide after self-targeting the local player"
        );

        app.world_mut()
            .resource_mut::<HudVisibilityToggles>()
            .show_target_frame = false;
        app.update();

        assert!(
            target_frame_hidden(&app),
            "target frame should hide when HUD toggle is off"
        );
    }

    fn target_frame_hidden(app: &App) -> bool {
        let ui = app.world().resource::<UiState>();
        let target_frame = ui
            .registry
            .get_by_name("TargetFrame")
            .expect("TargetFrame should exist");
        ui.registry
            .get(target_frame)
            .expect("TargetFrame should resolve")
            .hidden
    }
}
