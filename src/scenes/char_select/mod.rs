use bevy::prelude::*;
use std::time::Instant;

use game_engine::ui::atlas;
use game_engine::ui::frame::{Dimension, NineSlice};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_select_component::{
    BACK_BUTTON, CHAR_LIST_PANEL, CHAR_SELECT_ROOT, CREATE_CHAR_BUTTON, CampsiteEntry,
    CampsiteState, CharDisplayEntry, CharSelectState, DELETE_CHAR_BUTTON, ENTER_WORLD_BUTTON,
    SELECTED_NAME_TEXT, STATUS_TEXT, char_select_screen,
};
use game_engine::ui::widgets::texture::TextureSource;
use game_engine::ui_resource;
use shared::protocol::CharacterListEntry;
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;
use crate::networking::CharacterList;

pub mod input;
pub mod scene;
pub mod scene_tree;

use input::CharSelectClickEvent;
pub use scene::CharSelectScenePlugin;

const REALM_NAME: &str = "World of Osso";

ui_resource! {
    pub(crate) CharSelectUi {
        root: CHAR_SELECT_ROOT,
        enter_button: ENTER_WORLD_BUTTON,
        create_button: CREATE_CHAR_BUTTON,
        delete_button?: DELETE_CHAR_BUTTON,
        back_button: BACK_BUTTON,
        status_text: STATUS_TEXT,
        selected_name_text: SELECTED_NAME_TEXT,
        list_panel: CHAR_LIST_PANEL,
    }
}

#[derive(Resource, Default)]
pub(crate) struct SelectedCharIndex(pub(crate) Option<usize>);

/// CLI-provided character name to pre-select on the char select screen.
#[derive(Resource)]
pub(crate) struct PreselectedCharName(pub(crate) String);

#[derive(Resource, Default)]
pub(crate) struct CampsitePanelVisible(pub(crate) bool);

#[derive(Resource, Default)]
pub(crate) struct CharSelectFocus(pub(crate) Option<u64>);

#[derive(Resource, Default)]
struct CharSelectReadyLogged(bool);

struct CharSelectScreenRes {
    screen: Screen,
    shared: ui_toolkit::screen::SharedContext,
}
unsafe impl Send for CharSelectScreenRes {}
unsafe impl Sync for CharSelectScreenRes {}

#[derive(Resource)]
struct CharSelectScreenWrap(CharSelectScreenRes);

/// Marker resource: skip CharSelect UI and enter world immediately with first/preselected char.
#[derive(Resource)]
pub struct AutoEnterWorld;

pub struct CharSelectPlugin;

impl Plugin for CharSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedCharIndex>();
        app.init_resource::<CharSelectFocus>();
        app.init_resource::<CharSelectReadyLogged>();
        app.add_message::<CharSelectClickEvent>();
        app.add_systems(OnEnter(GameState::CharSelect), build_char_select_ui);
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_ui);
        app.add_systems(
            Update,
            (
                input::char_select_mouse_input,
                input::char_select_keyboard_input,
                input::char_select_run_automation,
                input::dispatch_char_select_action,
                char_select_update_visuals,
                report_char_select_ready,
                auto_enter_world,
            )
                .into_configs()
                .run_if(in_state(GameState::CharSelect)),
        );
    }
}

// --- UI Building ---

fn build_char_select_state(char_list: &CharacterList, selected: Option<usize>) -> CharSelectState {
    let characters: Vec<CharDisplayEntry> = char_list
        .0
        .iter()
        .map(|ch| CharDisplayEntry {
            name: ch.name.clone(),
            info: format!("Level {}   Race {}   Class {}", ch.level, ch.race, ch.class),
            status: "Ready to enter world".to_string(),
        })
        .collect();
    let selected_name = selected
        .and_then(|i| char_list.0.get(i))
        .map(|ch| ch.name.clone())
        .unwrap_or_else(|| "Character Selection".to_string());
    CharSelectState {
        characters,
        selected_index: selected,
        selected_name,
        status_text: String::new(),
    }
}

fn build_char_select_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    char_list: Res<CharacterList>,
    preselected: Option<Res<PreselectedCharName>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ready_logged: ResMut<CharSelectReadyLogged>,
) {
    let start = Instant::now();
    ready_logged.0 = false;
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let initial_selected = preselected
        .as_ref()
        .and_then(|p| {
            char_list
                .0
                .iter()
                .position(|ch| ch.name.eq_ignore_ascii_case(&p.0))
        })
        .or_else(|| char_list.0.first().map(|_| 0));
    let state = build_char_select_state(&char_list, initial_selected);

    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(build_campsite_state(false));
    let mut screen = Screen::new(char_select_screen);
    screen.sync(&shared, &mut ui.registry);

    let cs = CharSelectUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, &cs);

    commands.insert_resource(SelectedCharIndex(initial_selected));
    commands.insert_resource(CampsitePanelVisible(false));
    commands.insert_resource(CharSelectFocus(None));
    commands.insert_resource(CharSelectScreenWrap(CharSelectScreenRes { screen, shared }));
    commands.insert_resource(cs);
    info!(
        "build_char_select_ui finished in {:.3}s",
        start.elapsed().as_secs_f32()
    );
}

fn apply_post_setup(reg: &mut FrameRegistry, cs: &CharSelectUi) {
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    if let Some(frame) = reg.get_mut(cs.root) {
        frame.width = Dimension::Fixed(sw);
        frame.height = Dimension::Fixed(sh);
    }
    set_list_panel_backdrop(reg, cs.list_panel);
}

fn set_list_panel_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = atlas_nine_slice(
            "glues-characterselect-card-all-bg",
            frame.resolved_width(),
            frame.resolved_height(),
        );
    }
}

pub(crate) fn atlas_nine_slice(name: &str, frame_w: f32, frame_h: f32) -> Option<NineSlice> {
    let uv_edges = atlas::nine_slice_margins(name)?;
    let _ = (frame_w, frame_h);
    let edge_sizes = uv_edges;
    Some(NineSlice {
        edge_size: edge_sizes[0],
        edge_size_v: Some(edge_sizes[1]),
        edge_sizes: Some(edge_sizes),
        uv_edge_size: Some(uv_edges[0]),
        uv_edge_sizes: Some(uv_edges),
        texture: Some(TextureSource::Atlas(name.to_string())),
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    })
}

fn teardown_char_select_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<CharSelectScreenWrap>>,
    mut commands: Commands,
    mut ready_logged: ResMut<CharSelectReadyLogged>,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    ready_logged.0 = false;
    commands.remove_resource::<CharSelectScreenWrap>();
    commands.remove_resource::<CharSelectUi>();
    ui.focused_frame = None;
}

fn report_char_select_ready(
    screen: Option<Res<CharSelectScreenWrap>>,
    scene_tree: Option<Res<game_engine::scene_tree::SceneTree>>,
    startup: Option<Res<crate::game_state::StartupPerfTimer>>,
    mut ready_logged: ResMut<CharSelectReadyLogged>,
) {
    if ready_logged.0 || screen.is_none() || scene_tree.is_none() {
        return;
    }
    if let Some(startup) = startup {
        info!(
            "CharSelect ready at app_t={:.3}s",
            startup.0.elapsed().as_secs_f32()
        );
    } else {
        info!("CharSelect ready");
    }
    ready_logged.0 = true;
}

// --- Visual Updates ---

fn char_select_update_visuals(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    selected: Res<SelectedCharIndex>,
    campsite_visible: Res<CampsitePanelVisible>,
    char_list: Res<CharacterList>,
    mut screen_res: Option<ResMut<CharSelectScreenWrap>>,
) {
    sync_screen_state(
        &mut screen_res,
        &mut ui.registry,
        cs_ui.as_deref(),
        &char_list,
        &selected,
        &campsite_visible,
    );
    ui.focused_frame = None;
}

fn sync_screen_state(
    screen_res: &mut Option<ResMut<CharSelectScreenWrap>>,
    reg: &mut FrameRegistry,
    cs_ui: Option<&CharSelectUi>,
    char_list: &CharacterList,
    selected: &SelectedCharIndex,
    campsite_visible: &CampsitePanelVisible,
) {
    let Some(res) = screen_res.as_mut() else {
        return;
    };
    let inner = &mut res.0;
    let new_state = build_char_select_state_full(char_list, selected.0);
    inner.shared.insert(new_state);
    inner
        .shared
        .insert(build_campsite_state(campsite_visible.0));
    inner.screen.sync(&inner.shared, reg);
    if let Some(cs) = cs_ui {
        apply_post_setup(reg, cs);
    }
}

fn build_char_select_state_full(
    char_list: &CharacterList,
    selected: Option<usize>,
) -> CharSelectState {
    let characters: Vec<CharDisplayEntry> = char_list
        .0
        .iter()
        .map(|ch| CharDisplayEntry {
            name: ch.name.clone(),
            info: format!("Level {}   Race {}   Class {}", ch.level, ch.race, ch.class),
            status: "Ready to enter world".to_string(),
        })
        .collect();
    let selected_name = selected
        .and_then(|i| char_list.0.get(i))
        .map(|ch| ch.name.clone())
        .unwrap_or_else(|| "Character Selection".to_string());
    let status_text = compute_status_text(&char_list.0, selected);
    CharSelectState {
        characters,
        selected_index: selected,
        selected_name,
        status_text,
    }
}

fn compute_status_text(chars: &[CharacterListEntry], selected: Option<usize>) -> String {
    if let Some(ch) = selected.and_then(|idx| chars.get(idx)) {
        format!(
            "Realm: {}    Level {}    Race {}    Class {}",
            REALM_NAME, ch.level, ch.race, ch.class
        )
    } else if chars.is_empty() {
        "No characters available on this realm".to_string()
    } else {
        "Select a character to enter the world".to_string()
    }
}

pub(crate) fn build_campsite_state(panel_visible: bool) -> CampsiteState {
    let warband = crate::scenes::warband::WarbandScenes::load();
    let selected_id = warband.scenes.first().map(|s| s.id);
    CampsiteState {
        scenes: warband
            .scenes
            .iter()
            .map(|s| CampsiteEntry {
                id: s.id,
                name: s.name.clone(),
                preview_image: s.preview_image_path().map(str::to_string),
            })
            .collect(),
        panel_visible,
        selected_id,
    }
}

fn auto_enter_world(
    auto: Option<Res<AutoEnterWorld>>,
    selected: Res<SelectedCharIndex>,
    char_list: Res<crate::networking_auth::CharacterList>,
    mut senders: Query<&mut lightyear::prelude::MessageSender<shared::protocol::SelectCharacter>>,
    mut commands: Commands,
) {
    if auto.is_none() {
        return;
    }
    input::try_enter_world(&selected, &char_list, &mut senders);
    commands.remove_resource::<AutoEnterWorld>();
}

#[cfg(test)]
#[path = "../../../tests/unit/char_select_tests.rs"]
mod tests;
