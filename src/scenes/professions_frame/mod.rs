//! Professions scenes: the Retail ProfessionsBook (K) and ProfessionsFrame, built from
//! the owner's professions, the profession catalog and the bags. The book's spell
//! buttons open the frame on that profession; recipe clicks select, category clicks
//! collapse, Create / Create All send [`CraftRequest`]s, the search box filters by name.

mod view;

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::bag_data::InventoryState;
use game_engine::input_bindings::InputAction;
use game_engine::item_icons::item_icon_fdid;
use game_engine::profession::CraftRequest;
use game_engine::professions_data::profession_catalog;
use game_engine::spell_catalog::SpellCatalog;
use game_engine::status::ProfessionStatusSnapshot;
use game_engine::ui::frame::WidgetData;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::professions_book_component::{
    ACTION_CLOSE as ACTION_BOOK_CLOSE, ACTION_OPEN_PREFIX, ProfessionsBookState,
    professions_book_screen,
};
use game_engine::ui::screens::professions_frame_component::{
    ACTION_CLOSE, ACTION_COUNT_DOWN, ACTION_COUNT_UP, ACTION_CREATE, ACTION_CREATE_ALL,
    ACTION_ROW_PREFIX, ACTION_SEARCH, LIST_NAME, ProfessionsFrameState, SEARCH_NAME,
    professions_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::scenes::trainer_frame::wheel_notches_over;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};
pub use view::ProfessionsFrameSelection;
use view::{FrameView, Inputs, RowTarget, build_book, build_frame};

struct ScreenRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for ScreenRes {}
unsafe impl Sync for ScreenRes {}

#[derive(Resource)]
struct ProfessionScreens {
    frame: ScreenRes,
    book: ScreenRes,
}

#[derive(Resource, Clone, PartialEq, Default)]
struct ProfessionModels {
    frame: ProfessionsFrameState,
    book: ProfessionsBookState,
}

pub struct ProfessionsFramePlugin;

impl Plugin for ProfessionsFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProfessionsFrameSelection>()
            .add_message::<CraftRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_profession_screens.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_profession_screens);
        app.add_systems(
            Update,
            (
                toggle_professions_book,
                handle_profession_clicks,
                handle_recipe_wheel,
                handle_search_keys,
                sync_profession_screens,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// What the two screens are built from besides the window state and selection.
#[derive(SystemParam)]
struct ProfessionSources<'w> {
    status: Option<Res<'w, ProfessionStatusSnapshot>>,
    bags: Option<Res<'w, InventoryState>>,
    spells: Option<Res<'w, SpellCatalog>>,
}

fn bag_count(bags: Option<&InventoryState>, item_id: u32) -> u32 {
    bags.map_or(0, |bags| {
        bags.slots
            .iter()
            .flatten()
            .filter(|slot| slot.item_id == item_id)
            .map(|slot| slot.count.max(1))
            .sum()
    })
}

impl ProfessionSources<'_> {
    fn with_inputs<R>(&self, apply: impl FnOnce(&Inputs) -> R) -> R {
        let empty = ProfessionStatusSnapshot::default();
        let bags = self.bags.as_deref();
        let count = |item: u32| bag_count(bags, item);
        let inputs = Inputs {
            catalog: profession_catalog(),
            status: self.status.as_deref().unwrap_or(&empty),
            bag_count: &count,
            item_icon: &item_icon_fdid,
        };
        apply(&inputs)
    }

    fn frame(&self, selection: &ProfessionsFrameSelection, open: bool) -> FrameView {
        self.with_inputs(|inputs| build_frame(inputs, selection, open))
    }

    fn models(
        &self,
        manager: &WindowManager,
        selection: &ProfessionsFrameSelection,
    ) -> ProfessionModels {
        let spell_name_icon = |spell: u32| {
            self.spells
                .as_ref()
                .and_then(|catalog| catalog.get(spell))
                .map(|spell| (spell.name.to_string(), spell.icon_fdid))
        };
        ProfessionModels {
            frame: self
                .frame(selection, manager.is_open(WindowId::Professions))
                .state,
            book: self.with_inputs(|inputs| {
                build_book(
                    inputs,
                    manager.is_open(WindowId::ProfessionsBook),
                    &spell_name_icon,
                )
            }),
        }
    }
}

fn screen_res(
    ui: &mut UiState,
    build: fn(&SharedContext) -> Element,
    insert: impl FnOnce(&mut SharedContext),
) -> ScreenRes {
    let mut shared = SharedContext::new();
    insert(&mut shared);
    let mut screen = Screen::new(build);
    screen.sync(&shared, &mut ui.registry);
    ScreenRes { screen, shared }
}

fn build_profession_screens(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    (sources, manager, selection): (
        ProfessionSources,
        Res<WindowManager>,
        Res<ProfessionsFrameSelection>,
    ),
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let models = sources.models(&manager, &selection);
    let frame = screen_res(&mut ui, professions_frame_screen, |shared| {
        shared.insert(models.frame.clone())
    });
    let book = screen_res(&mut ui, professions_book_screen, |shared| {
        shared.insert(models.book.clone())
    });
    commands.insert_resource(ProfessionScreens { frame, book });
    commands.insert_resource(models);
}

fn teardown_profession_screens(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screens: Option<ResMut<ProfessionScreens>>,
) {
    if let Some(screens) = screens.as_mut() {
        screens.frame.screen.teardown(&mut ui.registry);
        screens.book.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<ProfessionScreens>();
    commands.remove_resource::<ProfessionModels>();
}

fn sync_profession_screens(
    mut ui: ResMut<UiState>,
    screens: Option<ResMut<ProfessionScreens>>,
    models: Option<ResMut<ProfessionModels>>,
    (sources, manager, selection): (
        ProfessionSources,
        Res<WindowManager>,
        Res<ProfessionsFrameSelection>,
    ),
) {
    let (Some(mut screens), Some(mut models)) = (screens, models) else {
        return;
    };
    let next = sources.models(&manager, &selection);
    if next.frame != models.frame {
        let res = &mut screens.frame;
        res.shared.insert(next.frame.clone());
        res.screen.sync(&res.shared, &mut ui.registry);
    }
    if next.book != models.book {
        let res = &mut screens.book;
        res.shared.insert(next.book.clone());
        res.screen.sync(&res.shared, &mut ui.registry);
    }
    *models = next;
}

/// K (`TOGGLEPROFESSIONBOOK`, Bindings_Standard.xml:1238) toggles the book.
fn toggle_professions_book(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut manager: ResMut<WindowManager>,
) {
    if keybinds.just_pressed(InputAction::ToggleProfessions) {
        manager.toggle(WindowId::ProfessionsBook);
    }
}

#[derive(SystemParam)]
struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    /// The `onclick` action under the cursor on a left click this frame (`Some("")`
    /// for a click on a frame without one).
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
        Some(walk_up_for_onclick(&ui.registry, frame_id).unwrap_or_default())
    }
}

fn handle_profession_clicks(
    pointer: Pointer,
    mut ui: ResMut<UiState>,
    mut manager: ResMut<WindowManager>,
    mut selection: ResMut<ProfessionsFrameSelection>,
    mut crafts: MessageWriter<CraftRequest>,
    sources: ProfessionSources,
) {
    if !manager.is_open(WindowId::Professions) && !manager.is_open(WindowId::ProfessionsBook) {
        return;
    }
    let Some(action) = pointer.click(&ui) else {
        return;
    };
    if action != ACTION_SEARCH && selection.search_focused {
        set_search_focus(&mut ui, &mut selection, false);
    }
    if let Some(line) = action
        .strip_prefix(ACTION_OPEN_PREFIX)
        .and_then(|line| line.parse::<u32>().ok())
    {
        if selection.profession != Some(line) {
            *selection = ProfessionsFrameSelection {
                profession: Some(line),
                ..Default::default()
            };
        }
        manager.open(WindowId::Professions);
        return;
    }
    let frame = sources.frame(&selection, manager.is_open(WindowId::Professions));
    match action.as_str() {
        ACTION_BOOK_CLOSE => {
            manager.close(WindowId::ProfessionsBook);
        }
        ACTION_CLOSE => {
            manager.close(WindowId::Professions);
        }
        ACTION_SEARCH => set_search_focus(&mut ui, &mut selection, true),
        ACTION_COUNT_DOWN => {
            selection.craft_count = frame.state.craft_count.saturating_sub(1).max(1)
        }
        ACTION_COUNT_UP => {
            selection.craft_count =
                (frame.state.craft_count + 1).min(frame.state.create_all_count.max(1) as u16)
        }
        ACTION_CREATE | ACTION_CREATE_ALL => {
            if let Some(spell_id) = frame.selected {
                let casts = if action == ACTION_CREATE_ALL {
                    frame.state.create_all_count as u16
                } else {
                    frame.state.craft_count
                };
                crafts.write(CraftRequest { spell_id, casts });
            }
        }
        _ => {
            if let Some(target) = action
                .strip_prefix(ACTION_ROW_PREFIX)
                .and_then(|index| index.parse::<usize>().ok())
                .and_then(|index| frame.targets.get(index))
            {
                match *target {
                    RowTarget::Category(category) => {
                        if !selection.collapsed.remove(&category) {
                            selection.collapsed.insert(category);
                        }
                    }
                    RowTarget::Recipe(spell) => {
                        selection.recipe = Some(spell);
                        selection.craft_count = 1;
                    }
                }
            }
        }
    }
}

fn search_editbox(ui: &UiState) -> Option<u64> {
    ui.registry.get_by_name(&format!("{SEARCH_NAME}Edit"))
}

/// Focus moves keyboard input to the search edit box (`UiInputMode::Text`), which
/// starts with the current search text.
fn set_search_focus(ui: &mut UiState, selection: &mut ProfessionsFrameSelection, focused: bool) {
    selection.search_focused = focused;
    let Some(editbox) = search_editbox(ui) else {
        return;
    };
    if focused {
        if let Some(WidgetData::EditBox(data)) = ui
            .registry
            .get_mut(editbox)
            .and_then(|frame| frame.widget_data.as_mut())
        {
            data.replace_range(0, data.text.len(), &selection.search);
            data.cursor_end();
        }
        ui.registry.focused_frame = Some(editbox);
        ui.focused_frame = Some(editbox);
    } else {
        if ui.focused_frame == Some(editbox) {
            ui.focused_frame = None;
        }
        if ui.registry.focused_frame == Some(editbox) {
            ui.registry.focused_frame = None;
        }
    }
}

/// Typing edits the search text; Enter and Escape end the search focus.
fn handle_search_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    mut selection: ResMut<ProfessionsFrameSelection>,
) {
    if !selection.search_focused {
        keys.clear();
        return;
    }
    let Some(editbox) = search_editbox(&ui) else {
        return;
    };
    for event in keys.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if matches!(event.key_code, KeyCode::Enter | KeyCode::Escape) {
            set_search_focus(&mut ui, &mut selection, false);
            return;
        }
        let Some(WidgetData::EditBox(data)) = ui
            .registry
            .get_mut(editbox)
            .and_then(|frame| frame.widget_data.as_mut())
        else {
            return;
        };
        match event.key_code {
            KeyCode::Backspace => data.backspace(),
            KeyCode::Delete => data.delete_forward(),
            KeyCode::ArrowLeft => data.cursor_left(),
            KeyCode::ArrowRight => data.cursor_right(),
            _ => {
                if let Some(typed) = event.text.as_deref()
                    && !typed.chars().any(char::is_control)
                {
                    data.insert_at_cursor(typed);
                }
            }
        }
        if data.text != selection.search {
            selection.search = data.text.clone();
            selection.scroll = 0;
        }
    }
}

/// The wheel over the recipe list scrolls one row per notch.
fn handle_recipe_wheel(
    windows: Query<&Window, With<PrimaryWindow>>,
    wheel: Option<Res<AccumulatedMouseScroll>>,
    ui: Res<UiState>,
    manager: Res<WindowManager>,
    mut selection: ResMut<ProfessionsFrameSelection>,
    sources: ProfessionSources,
) {
    if !manager.is_open(WindowId::Professions) {
        return;
    }
    let notches = wheel_notches_over(&windows, wheel.as_deref(), &ui, LIST_NAME, 20.0);
    if notches == 0 {
        return;
    }
    let total = sources.frame(&selection, true).total_rows;
    selection.scroll = selection
        .scroll
        .saturating_add_signed(-notches)
        .min(total.saturating_sub(1));
}

#[cfg(test)]
mod tests;
