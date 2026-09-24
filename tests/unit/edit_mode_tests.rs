use std::path::{Path, PathBuf};

use bevy::ui::Val;
use game_engine::input_bindings::InputBindings;
use game_engine::ui::event::EventBus;
use game_engine::ui::frame::WidgetData;
use game_engine::ui::layout::LayoutRect;
use game_engine::ui::screens::edit_mode_component::{
    ACTION_EDIT_MODE_DELETE, ACTION_EDIT_MODE_NEW, ACTION_EDIT_MODE_RENAME, EDIT_MODE_PANEL,
    PANEL_TOP, PANEL_W,
};

use super::layouts::{EditModeLayoutsFile, HudAnchor, PRESET_LAYOUT};
use super::panel::dispatch_edit_mode_action;
use super::*;
use crate::ui_input_mode::UiInputMode;
use crate::window_manager::{WindowId, WindowManager};

const SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);
/// Authored target frame rect used by these tests.
const TARGET_AUTHORED: Vec2 = Vec2::new(1100.0, 800.0);
const TARGET_SIZE: Vec2 = Vec2::new(200.0, 60.0);

fn temp_layout_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "edit-mode-{tag}-{}-{}.ron",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn saved_placement_keeps_its_offset_from_the_anchor_when_the_screen_resizes() {
    let saved = SavedElement::from_top_left(
        HudAnchor::Bottom,
        Vec2::new(860.0, 900.0),
        Vec2::new(200.0, 60.0),
        SCREEN,
    );

    assert_eq!(saved.offset, [0.0, -120.0]);
    assert_eq!(
        saved.top_left(Vec2::new(200.0, 60.0), Vec2::new(2560.0, 1440.0)),
        Vec2::new(1180.0, 1260.0)
    );
}

#[test]
fn snap_rounds_to_the_grid_then_to_screen_edges_and_clamps() {
    let size = Vec2::new(100.0, 40.0);

    assert_eq!(
        snap_position(Vec2::new(203.0, 99.0), size, SCREEN),
        Vec2::new(200.0, 96.0)
    );
    assert_eq!(
        snap_position(Vec2::new(1815.0, 5.0), size, SCREEN),
        Vec2::new(1820.0, 0.0),
        "within one grid step of an edge snaps to it"
    );
    assert_eq!(
        snap_position(Vec2::new(-50.0, 2000.0), size, SCREEN),
        Vec2::new(0.0, 1040.0)
    );
}

#[test]
fn preset_layout_cannot_be_modified_renamed_or_deleted() {
    let mut file = EditModeLayoutsFile::default();

    assert!(
        file.save_layout(PRESET_LAYOUT, EditLayout::default())
            .is_err()
    );
    assert!(file.rename_layout(PRESET_LAYOUT, "Mine").is_err());
    assert!(file.delete_layout(PRESET_LAYOUT).is_err());
    assert_eq!(file.layout_names(), vec![PRESET_LAYOUT.to_string()]);
}

#[test]
fn every_plan_hud_element_is_registered_once() {
    let mut keys: Vec<_> = EDIT_MODE_ELEMENTS.iter().map(|e| e.key).collect();
    let count = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), count);
    for frame in [
        "PlayerFrame",
        "TargetFrame",
        "CastingBarFrame",
        "MainActionBar",
        "MultiBarBottomLeft",
        "MultiBarBottomRight",
        "MultiBarRight",
        "MultiBarLeft",
        "MinimapCluster",
        "ObjectiveTrackerFrame",
        "BuffFrame",
        "PartyFrame",
        "RaidFrame",
        "UIErrorsFrame",
        "MicroMenuContainer",
        "BagsBar",
    ] {
        assert!(
            EDIT_MODE_ELEMENTS.iter().any(|e| e.frame_name == frame),
            "{frame} missing"
        );
    }
}

// --- Bevy App ---

fn edit_mode_app(path: &Path, character_id: u64) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_state(GameState::Loading);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<InputBindings>();
    app.init_resource::<UiInputMode>();
    app.add_message::<bevy::input::keyboard::KeyboardInput>();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(SCREEN.x, SCREEN.y),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(UiLayoutStore::load(path.to_path_buf()));
    app.insert_resource(SelectedCharacterId {
        character_id: Some(character_id),
        character_name: Some(format!("Char{character_id}")),
    });
    app.world_mut().spawn((
        Window {
            resolution: (SCREEN.x as u32, SCREEN.y as u32).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.add_plugins(EditModePlugin);
    add_target_frame(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::InWorld);
    app.update();
    app
}

/// Target frame authored at `TARGET_AUTHORED`, with the layout readback the
/// native Bevy UI pass would publish.
fn add_target_frame(app: &mut App) {
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let id = ui.registry.create_frame("TargetFrame", None);
    let frame = ui.registry.get_mut(id).unwrap();
    frame.width = Dimension::Fixed(TARGET_SIZE.x);
    frame.height = Dimension::Fixed(TARGET_SIZE.y);
    frame.position_type = PositionType::Absolute;
    frame.position.left = Val::Px(TARGET_AUTHORED.x);
    frame.position.top = Val::Px(TARGET_AUTHORED.y);
    ui.registry
        .set_computed_layout(
            id,
            LayoutRect {
                x: TARGET_AUTHORED.x,
                y: TARGET_AUTHORED.y,
                width: TARGET_SIZE.x,
                height: TARGET_SIZE.y,
            },
        )
        .unwrap();
}

fn target_position(app: &App) -> Vec2 {
    let registry = &app.world().resource::<UiState>().registry;
    let frame = registry
        .get(registry.get_by_name("TargetFrame").unwrap())
        .unwrap();
    match (frame.position.left, frame.position.top) {
        (Val::Px(x), Val::Px(y)) => Vec2::new(x, y),
        other => panic!("target frame not placed: {other:?}"),
    }
}

fn tap_key(app: &mut App, key: KeyCode) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
    keys.press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
}

fn set_cursor(app: &mut App, cursor: Vec2) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .unwrap()
        .set_cursor_position(Some(cursor));
}

fn press_left_at(app: &mut App, cursor: Vec2) {
    set_cursor(app, cursor);
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release_all();
    mouse.clear();
    mouse.press(MouseButton::Left);
    app.update();
}

fn drag(app: &mut App, from: Vec2, to: Vec2) {
    press_left_at(app, from);
    set_cursor(app, to);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.update();
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.clear();
    mouse.release(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
}

/// Publishes the layout readback for a panel button at its authored rect.
fn click_panel_button(app: &mut App, name: &str, left: f32, top: f32, width: f32) {
    let panel_x = (SCREEN.x - PANEL_W) / 2.0;
    let rect = LayoutRect {
        x: panel_x + left,
        y: PANEL_TOP + top,
        width,
        height: 26.0,
    };
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let id = ui.registry.get_by_name(name).expect(name);
        ui.registry.set_computed_layout(id, rect.clone()).unwrap();
    }
    press_left_at(
        app,
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
    );
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release(MouseButton::Left);
    mouse.clear();
}

fn edit_mode(app: &App) -> &EditMode {
    app.world().resource::<EditMode>()
}

/// Grab 50/20 into the target frame and drop so the snapped top-left is (1400, 496).
fn drag_target_to_1400_496(app: &mut App) {
    drag(
        app,
        TARGET_AUTHORED + Vec2::new(50.0, 20.0),
        Vec2::new(1450.0, 516.0),
    );
}

#[test]
fn f10_shows_a_named_selection_box_for_each_mounted_element() {
    let path = temp_layout_path("boxes");
    let mut app = edit_mode_app(&path, 11);

    tap_key(&mut app, KeyCode::F10);

    assert!(edit_mode(&app).is_active());
    let registry = &app.world().resource::<UiState>().registry;
    assert!(registry.get_by_name(EDIT_MODE_PANEL.0).is_some());
    let label = registry
        .get_by_name("EditModeSelection_target_frameLabel")
        .expect("target frame selection label");
    match &registry.get(label).unwrap().widget_data {
        Some(WidgetData::FontString(text)) => assert_eq!(text.text, "Target Frame"),
        other => panic!("label is not a fontstring: {other:?}"),
    }
    assert!(
        registry
            .get_by_name("EditModeSelection_player_frame")
            .is_none(),
        "unmounted elements have no box"
    );
}

#[test]
fn edit_mode_moves_the_target_frame_saves_and_survives_restart() {
    let path = temp_layout_path("restart");
    {
        let mut app = edit_mode_app(&path, 11);
        tap_key(&mut app, KeyCode::F10);
        drag_target_to_1400_496(&mut app);
        assert_eq!(target_position(&app), Vec2::new(1400.0, 496.0));
        assert!(edit_mode(&app).is_dirty());

        click_panel_button(&mut app, "EditModeManagerFrameSave", 138.0, 140.0, 104.0);

        assert!(!edit_mode(&app).is_dirty());
        assert_eq!(edit_mode(&app).layout_name(), "Layout 1");
        tap_key(&mut app, KeyCode::F10);
        assert!(!edit_mode(&app).is_active());
        assert_eq!(target_position(&app), Vec2::new(1400.0, 496.0));
    }

    let restarted = edit_mode_app(&path, 11);
    assert_eq!(edit_mode(&restarted).layout_name(), "Layout 1");
    assert_eq!(target_position(&restarted), Vec2::new(1400.0, 496.0));
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn active_layout_is_chosen_per_character_and_layouts_are_account_wide() {
    let path = temp_layout_path("per-char");
    {
        let mut app = edit_mode_app(&path, 11);
        tap_key(&mut app, KeyCode::F10);
        drag_target_to_1400_496(&mut app);
        click_panel_button(&mut app, "EditModeManagerFrameSave", 138.0, 140.0, 104.0);
    }

    let mut other = edit_mode_app(&path, 22);
    assert_eq!(edit_mode(&other).layout_name(), PRESET_LAYOUT);
    assert_eq!(target_position(&other), TARGET_AUTHORED);

    tap_key(&mut other, KeyCode::F10);
    click_panel_button(
        &mut other,
        "EditModeManagerFrameNext",
        PANEL_W - 50.0,
        36.0,
        30.0,
    );
    assert_eq!(edit_mode(&other).layout_name(), "Layout 1");
    other.update();
    assert_eq!(target_position(&other), Vec2::new(1400.0, 496.0));

    let first_again = edit_mode_app(&path, 11);
    assert_eq!(edit_mode(&first_again).layout_name(), "Layout 1");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn exiting_without_saving_restores_the_authored_position() {
    let path = temp_layout_path("discard");
    let mut app = edit_mode_app(&path, 11);
    tap_key(&mut app, KeyCode::F10);
    drag_target_to_1400_496(&mut app);

    tap_key(&mut app, KeyCode::F10);

    assert!(!edit_mode(&app).is_dirty());
    assert_eq!(target_position(&app), TARGET_AUTHORED);
    assert!(!path.exists(), "nothing was saved");
}

#[test]
fn rename_and_delete_update_the_stored_layouts() {
    let path = temp_layout_path("rename");
    let mut store = UiLayoutStore::load(path.clone());
    let mut edit = EditMode::default();
    edit.load_layout(PRESET_LAYOUT.to_string(), EditLayout::default());
    edit.enter();

    edit.name_draft = "Raid".into();
    dispatch_edit_mode_action(ACTION_EDIT_MODE_NEW, &mut edit, &mut store, Some("11"));
    assert_eq!(edit.layout_name(), "Raid");

    edit.name_draft = "Raid 25".into();
    dispatch_edit_mode_action(ACTION_EDIT_MODE_RENAME, &mut edit, &mut store, Some("11"));
    let reloaded = UiLayoutStore::load(path.clone());
    assert_eq!(
        reloaded.file.edit_mode.layout_names(),
        vec![PRESET_LAYOUT.to_string(), "Raid 25".to_string()]
    );
    assert_eq!(
        reloaded.file.edit_mode.active_layout_name(Some("11")),
        "Raid 25"
    );

    dispatch_edit_mode_action(ACTION_EDIT_MODE_DELETE, &mut edit, &mut store, Some("11"));
    assert_eq!(edit.layout_name(), PRESET_LAYOUT);
    let reloaded = UiLayoutStore::load(path.clone());
    assert_eq!(
        reloaded.file.edit_mode.layout_names(),
        vec![PRESET_LAYOUT.to_string()]
    );
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn escape_exits_edit_mode_before_closing_windows() {
    let path = temp_layout_path("escape");
    let mut app = edit_mode_app(&path, 11);
    app.init_resource::<ui_toolkit::native_render::caret::UiCaretBlocked>();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(crate::client_options::HudOptions::default());
    app.insert_resource(crate::client_options::ClientOptionsUiState {
        modal_offset: None,
        legacy_modal_position: None,
    });
    app.init_resource::<game_engine::targeting::CurrentTarget>();
    app.add_plugins((
        crate::scenes::game_menu::GameMenuScreenPlugin,
        crate::window_manager::WindowManagerPlugin,
    ));
    app.update();
    app.world_mut()
        .resource_mut::<WindowManager>()
        .open(WindowId::Character);
    tap_key(&mut app, KeyCode::F10);
    assert!(edit_mode(&app).is_active());

    tap_key(&mut app, KeyCode::Escape);
    assert!(!edit_mode(&app).is_active(), "press 1 exits edit mode");
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Character)
    );

    tap_key(&mut app, KeyCode::Escape);
    assert!(
        !app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Character),
        "press 2 closes windows"
    );
}
