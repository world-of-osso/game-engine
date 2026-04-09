use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::profession::{ProfessionRuntimeState, queue_craft_action};
use game_engine::status::{ProfessionRecipeEntry, ProfessionStatusSnapshot};
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::professions_frame_component::{
    ACTION_PROFESSION_CRAFT, ACTION_PROFESSION_RECIPE_PREFIX, ACTION_PROFESSION_TAB_PREFIX,
    CraftingDetail, ProfessionTab, ProfessionsFrameState, RecipeState, professions_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

/// Tracks whether the Professions panel is open.
#[derive(Resource, Default)]
pub struct ProfessionsFrameOpen(pub bool);

#[derive(Resource, Default, Clone, PartialEq)]
struct ProfessionsFrameSelection {
    active_profession: Option<String>,
    selected_recipe_id: Option<u32>,
}

struct ProfessionsFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for ProfessionsFrameRes {}
unsafe impl Sync for ProfessionsFrameRes {}

#[derive(Resource)]
struct ProfessionsFrameWrap(ProfessionsFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct ProfessionsFrameModel(ProfessionsFrameState);

pub struct ProfessionsFramePlugin;

impl Plugin for ProfessionsFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProfessionsFrameOpen>();
        app.init_resource::<ProfessionsFrameSelection>();
        app.add_systems(OnEnter(GameState::InWorld), build_professions_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_professions_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_professions_frame,
                sync_professions_frame_state,
                handle_professions_frame_input,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_professions_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    snapshot: Option<Res<ProfessionStatusSnapshot>>,
    open: Res<ProfessionsFrameOpen>,
    selection: Res<ProfessionsFrameSelection>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(snapshot.as_deref(), &open, &selection);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(professions_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(ProfessionsFrameWrap(ProfessionsFrameRes { screen, shared }));
    commands.insert_resource(ProfessionsFrameModel(state));
}

fn teardown_professions_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<ProfessionsFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<ProfessionsFrameWrap>();
    commands.remove_resource::<ProfessionsFrameModel>();
}

fn toggle_professions_frame(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<ProfessionsFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyK) {
        open.0 = !open.0;
    }
}

fn sync_professions_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<ProfessionsFrameWrap>>,
    mut last_model: Option<ResMut<ProfessionsFrameModel>>,
    snapshot: Option<Res<ProfessionStatusSnapshot>>,
    open: Res<ProfessionsFrameOpen>,
    selection: Res<ProfessionsFrameSelection>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(snapshot.as_deref(), &open, &selection);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn handle_professions_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    open: Res<ProfessionsFrameOpen>,
    snapshot: Option<Res<ProfessionStatusSnapshot>>,
    mut selection: ResMut<ProfessionsFrameSelection>,
    mut runtime: ResMut<ProfessionRuntimeState>,
) {
    if !open.0 || !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    dispatch_action(&action, snapshot.as_deref(), &mut selection, &mut runtime);
}

fn build_state(
    snapshot: Option<&ProfessionStatusSnapshot>,
    open: &ProfessionsFrameOpen,
    selection: &ProfessionsFrameSelection,
) -> ProfessionsFrameState {
    let active_profession = resolve_active_profession(snapshot, selection);
    let recipes = filtered_recipes(snapshot, active_profession.as_deref());
    let selected_recipe = resolve_selected_recipe(&recipes, selection.selected_recipe_id);
    ProfessionsFrameState {
        visible: open.0,
        tabs: build_tabs(snapshot, active_profession.as_deref()),
        recipes: recipes
            .iter()
            .map(|recipe| recipe_entry_to_state(recipe, selected_recipe.map(|r| r.spell_id)))
            .collect(),
        crafting: build_crafting_detail(selected_recipe),
        book_recipes: Vec::new(),
    }
}

fn resolve_active_profession(
    snapshot: Option<&ProfessionStatusSnapshot>,
    selection: &ProfessionsFrameSelection,
) -> Option<String> {
    let Some(snapshot) = snapshot else {
        return None;
    };
    if let Some(active) = &selection.active_profession
        && snapshot
            .skills
            .iter()
            .any(|skill| skill.profession == *active)
    {
        return Some(active.clone());
    }
    snapshot
        .skills
        .first()
        .map(|skill| skill.profession.clone())
}

fn filtered_recipes<'a>(
    snapshot: Option<&'a ProfessionStatusSnapshot>,
    active_profession: Option<&str>,
) -> Vec<&'a ProfessionRecipeEntry> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };
    snapshot
        .recipes
        .iter()
        .filter(|recipe| {
            active_profession
                .map(|profession| recipe.profession == profession)
                .unwrap_or(true)
        })
        .collect()
}

fn resolve_selected_recipe<'a>(
    recipes: &'a [&'a ProfessionRecipeEntry],
    selected_recipe_id: Option<u32>,
) -> Option<&'a ProfessionRecipeEntry> {
    selected_recipe_id
        .and_then(|recipe_id| {
            recipes
                .iter()
                .find(|recipe| recipe.spell_id == recipe_id)
                .copied()
        })
        .or_else(|| recipes.first().copied())
}

fn build_tabs(
    snapshot: Option<&ProfessionStatusSnapshot>,
    active_profession: Option<&str>,
) -> Vec<ProfessionTab> {
    snapshot
        .map(|snapshot| {
            snapshot
                .skills
                .iter()
                .map(|skill| ProfessionTab {
                    name: format!("{} {}/{}", skill.profession, skill.current, skill.max),
                    active: Some(skill.profession.as_str()) == active_profession,
                    action: format!("{ACTION_PROFESSION_TAB_PREFIX}{}", skill.profession),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_crafting_detail(selected_recipe: Option<&ProfessionRecipeEntry>) -> CraftingDetail {
    let Some(recipe) = selected_recipe else {
        return CraftingDetail::default();
    };
    CraftingDetail {
        recipe_name: recipe.name.clone(),
        reagent_count: 0,
        quality: if recipe.craftable { 1.0 } else { 0.0 },
        quality_text: recipe.cooldown.clone().unwrap_or_else(|| {
            if recipe.craftable {
                "Ready".into()
            } else {
                "Not Ready".into()
            }
        }),
    }
}

fn recipe_entry_to_state(
    entry: &ProfessionRecipeEntry,
    selected_recipe_id: Option<u32>,
) -> RecipeState {
    RecipeState {
        recipe_id: entry.spell_id,
        name: entry.name.clone(),
        profession: entry.profession.clone(),
        craftable: entry.craftable,
        cooldown: entry.cooldown.clone().unwrap_or_default(),
        active: Some(entry.spell_id) == selected_recipe_id,
        action: format!("{ACTION_PROFESSION_RECIPE_PREFIX}{}", entry.spell_id),
    }
}

fn dispatch_action(
    action: &str,
    snapshot: Option<&ProfessionStatusSnapshot>,
    selection: &mut ProfessionsFrameSelection,
    runtime: &mut ProfessionRuntimeState,
) {
    if let Some(profession) = parse_tab_action(action) {
        selection.active_profession = Some(profession);
        selection.selected_recipe_id = None;
        return;
    }
    if let Some(recipe_id) = parse_recipe_action(action) {
        selection.selected_recipe_id = Some(recipe_id);
        return;
    }
    if action == ACTION_PROFESSION_CRAFT
        && let Some(recipe_id) = selected_recipe_id(snapshot, selection)
    {
        queue_craft_action(runtime, recipe_id);
    }
}

fn selected_recipe_id(
    snapshot: Option<&ProfessionStatusSnapshot>,
    selection: &ProfessionsFrameSelection,
) -> Option<u32> {
    let active_profession = resolve_active_profession(snapshot, selection);
    let recipes = filtered_recipes(snapshot, active_profession.as_deref());
    resolve_selected_recipe(&recipes, selection.selected_recipe_id).map(|recipe| recipe.spell_id)
}

fn parse_tab_action(action: &str) -> Option<String> {
    Some(
        action
            .strip_prefix(ACTION_PROFESSION_TAB_PREFIX)?
            .to_string(),
    )
}

fn parse_recipe_action(action: &str) -> Option<u32> {
    action
        .strip_prefix(ACTION_PROFESSION_RECIPE_PREFIX)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::status::{ProfessionSkillEntry, ProfessionSkillUpEntry};

    fn snapshot() -> ProfessionStatusSnapshot {
        ProfessionStatusSnapshot {
            skills: sample_skills(),
            recipes: sample_recipes(),
            last_server_message: None,
            last_skill_up: Some(sample_skill_up()),
            last_error: None,
        }
    }

    fn sample_skills() -> Vec<ProfessionSkillEntry> {
        vec![
            ProfessionSkillEntry {
                profession: "Alchemy".into(),
                current: 25,
                max: 75,
            },
            ProfessionSkillEntry {
                profession: "Mining".into(),
                current: 42,
                max: 75,
            },
        ]
    }

    fn sample_recipes() -> Vec<ProfessionRecipeEntry> {
        vec![
            ProfessionRecipeEntry {
                spell_id: 1001,
                profession: "Alchemy".into(),
                name: "Minor Healing Potion".into(),
                craftable: true,
                cooldown: None,
            },
            ProfessionRecipeEntry {
                spell_id: 2001,
                profession: "Mining".into(),
                name: "Smelt Copper".into(),
                craftable: false,
                cooldown: Some("On Cooldown".into()),
            },
        ]
    }

    fn sample_skill_up() -> ProfessionSkillUpEntry {
        ProfessionSkillUpEntry {
            profession: "Alchemy".into(),
            current: 26,
            max: 75,
        }
    }

    #[test]
    fn build_state_filters_recipes_by_selected_profession() {
        let state = build_state(
            Some(&snapshot()),
            &ProfessionsFrameOpen(true),
            &ProfessionsFrameSelection {
                active_profession: Some("Mining".into()),
                selected_recipe_id: None,
            },
        );

        assert_eq!(state.recipes.len(), 1);
        assert_eq!(state.recipes[0].name, "Smelt Copper");
        assert!(state.tabs[1].active);
    }

    #[test]
    fn build_state_marks_selected_recipe_and_detail() {
        let state = build_state(
            Some(&snapshot()),
            &ProfessionsFrameOpen(true),
            &ProfessionsFrameSelection {
                active_profession: Some("Alchemy".into()),
                selected_recipe_id: Some(1001),
            },
        );

        assert!(state.recipes[0].active);
        assert_eq!(state.crafting.recipe_name, "Minor Healing Potion");
        assert_eq!(state.crafting.quality_text, "Ready");
    }

    #[test]
    fn parse_actions_extract_profession_and_recipe() {
        assert_eq!(
            parse_tab_action("profession_tab:Alchemy").as_deref(),
            Some("Alchemy")
        );
        assert_eq!(parse_recipe_action("profession_recipe:1001"), Some(1001));
    }
}
