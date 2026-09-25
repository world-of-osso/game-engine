use bevy::window::PrimaryWindow;
use game_engine::status::InspectStatusSnapshot;
use game_engine::ui::event::EventBus;
use game_engine::ui::frame::Dimension;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;

use super::*;

const CHARACTER_W: f32 = 336.0;

fn manager_with(open: &[WindowId]) -> WindowManager {
    let mut manager = WindowManager::default();
    for id in open {
        manager.open(*id);
    }
    manager
}

#[test]
fn opening_a_third_panel_closes_the_oldest() {
    let manager = manager_with(&[WindowId::Character, WindowId::Friends, WindowId::Guild]);

    assert!(!manager.is_open(WindowId::Character));
    assert_eq!(
        manager.open_windows(),
        &[WindowId::Friends, WindowId::Guild]
    );
    assert_eq!(manager.panel_in_slot(0), Some(WindowId::Friends));
    assert_eq!(manager.panel_in_slot(1), Some(WindowId::Guild));
}

#[test]
fn npc_panel_takes_slot_left_and_pushes_the_others_right() {
    let manager = manager_with(&[WindowId::Character, WindowId::Merchant]);

    assert_eq!(manager.panel_in_slot(0), Some(WindowId::Merchant));
    assert_eq!(manager.panel_in_slot(1), Some(WindowId::Character));
}

#[test]
fn npc_panel_with_two_open_evicts_the_oldest_of_them() {
    let manager = manager_with(&[WindowId::Character, WindowId::Friends, WindowId::Mail]);

    assert_eq!(manager.open_windows(), &[WindowId::Friends, WindowId::Mail]);
    assert_eq!(manager.panel_in_slot(0), Some(WindowId::Mail));
    assert_eq!(manager.panel_in_slot(1), Some(WindowId::Friends));
}

#[test]
fn wide_window_closes_panels_but_not_bags() {
    let manager = manager_with(&[
        WindowId::Character,
        WindowId::Bag(0),
        WindowId::Friends,
        WindowId::Talents,
    ]);

    assert_eq!(
        manager.open_windows(),
        &[WindowId::Bag(0), WindowId::Talents]
    );
}

#[test]
fn panel_closes_an_open_wide_window() {
    let manager = manager_with(&[WindowId::WorldMap, WindowId::Bag(1), WindowId::Character]);

    assert_eq!(
        manager.open_windows(),
        &[WindowId::Bag(1), WindowId::Character]
    );
}

#[test]
fn second_wide_window_replaces_the_first() {
    let manager = manager_with(&[WindowId::WorldMap, WindowId::Achievements]);

    assert_eq!(manager.open_windows(), &[WindowId::Achievements]);
}

#[test]
fn toggle_reports_the_resulting_state() {
    let mut manager = WindowManager::default();

    assert!(manager.toggle(WindowId::Calendar));
    assert!(!manager.toggle(WindowId::Calendar));
    assert!(!manager.any_open());
}

#[test]
fn window_classes_follow_the_plan_table() {
    for id in [
        WindowId::Character,
        WindowId::Spellbook,
        WindowId::ProfessionsBook,
        WindowId::Friends,
        WindowId::Guild,
        WindowId::Mail,
        WindowId::LootRules,
        WindowId::Calendar,
        WindowId::Inspect,
        WindowId::Merchant,
        WindowId::Trainer,
    ] {
        assert_eq!(id.class(), WindowClass::Panel, "{id:?}");
    }
    for id in [
        WindowId::WorldMap,
        WindowId::Talents,
        WindowId::Achievements,
        WindowId::EncounterJournal,
        // ProfessionsFrame is 942 wide (Blizzard_ProfessionsCrafting.lua:344-351).
        WindowId::Professions,
    ] {
        assert_eq!(id.class(), WindowClass::Wide, "{id:?}");
    }
    assert_eq!(WindowId::Bag(3).class(), WindowClass::Container);
}

// --- Bevy App: placement, raise and sessions ---

pub(crate) fn window_app(screen: Vec2, ui_scale: f32) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_state(GameState::InWorld);
    app.insert_resource(ButtonInput::<MouseButton>::default());
    let mut registry = FrameRegistry::new(screen.x / ui_scale, screen.y / ui_scale);
    registry.ui_scale = ui_scale;
    app.insert_resource(UiState {
        registry,
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.world_mut().spawn((
        Window {
            resolution: (screen.x as u32, screen.y as u32).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.add_plugins(WindowManagerPlugin);
    app
}

pub(crate) fn add_window_frame(app: &mut App, id: WindowId, size: Vec2) -> u64 {
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let root = ui.registry.create_frame(&id.root_frame_name(), None);
    let child = ui
        .registry
        .create_frame(&format!("{}Child", id.root_frame_name()), Some(root));
    for frame_id in [root, child] {
        let frame = ui.registry.get_mut(frame_id).unwrap();
        frame.width = Dimension::Fixed(size.x);
        frame.height = Dimension::Fixed(size.y);
    }
    root
}

pub(crate) fn open_window(app: &mut App, id: WindowId) {
    app.world_mut().resource_mut::<WindowManager>().open(id);
    app.update();
}

pub(crate) fn frame_pos(app: &App, id: WindowId) -> Vec2 {
    let ui = app.world().resource::<UiState>();
    let frame_id = ui.registry.get_by_name(&id.root_frame_name()).unwrap();
    let frame = ui.registry.get(frame_id).unwrap();
    match (frame.position.left, frame.position.top) {
        (bevy::ui::Val::Px(x), bevy::ui::Val::Px(y)) => Vec2::new(x, y),
        other => panic!("{id:?} not placed: {other:?}"),
    }
}

fn frame_level(app: &App, name: &str) -> i32 {
    let ui = app.world().resource::<UiState>();
    let id = ui.registry.get_by_name(name).unwrap();
    ui.registry.get(id).unwrap().frame_level
}

pub(crate) fn press_left_at(app: &mut App, cursor: Vec2, ui_scale: f32) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .unwrap()
        .set_cursor_position(Some(cursor * ui_scale));
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release_all();
    mouse.clear();
    mouse.press(MouseButton::Left);
    app.update();
}

#[test]
fn panels_open_in_slot_left_then_right_of_it() {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), 1.0);
    add_window_frame(&mut app, WindowId::Character, Vec2::new(CHARACTER_W, 424.0));
    add_window_frame(&mut app, WindowId::Friends, Vec2::new(384.0, 512.0));

    open_window(&mut app, WindowId::Character);
    open_window(&mut app, WindowId::Friends);

    assert_eq!(frame_pos(&app, WindowId::Character), Vec2::new(16.0, 104.0));
    assert_eq!(
        frame_pos(&app, WindowId::Friends),
        Vec2::new(16.0 + CHARACTER_W + 16.0, 104.0)
    );
}

#[test]
fn wide_window_is_centered_below_the_top_bar() {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), 1.0);
    add_window_frame(&mut app, WindowId::Achievements, Vec2::new(504.0, 480.0));

    open_window(&mut app, WindowId::Achievements);

    assert_eq!(
        frame_pos(&app, WindowId::Achievements),
        Vec2::new((1920.0 - 504.0) / 2.0, 104.0)
    );
}

#[test]
fn bags_stack_upward_from_the_bottom_right() {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), 1.0);
    add_window_frame(&mut app, WindowId::Bag(0), Vec2::new(200.0, 300.0));
    add_window_frame(&mut app, WindowId::Bag(1), Vec2::new(180.0, 200.0));

    open_window(&mut app, WindowId::Bag(1));
    open_window(&mut app, WindowId::Bag(0));

    let backpack = frame_pos(&app, WindowId::Bag(0));
    assert_eq!(
        backpack,
        Vec2::new(1920.0 - 16.0 - 200.0, 1080.0 - 96.0 - 300.0)
    );
    assert_eq!(
        frame_pos(&app, WindowId::Bag(1)),
        Vec2::new(1920.0 - 16.0 - 180.0, backpack.y - 8.0 - 200.0)
    );
}

#[test]
fn clicking_the_lower_window_raises_it_above_the_other() {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), 1.0);
    add_window_frame(&mut app, WindowId::Character, Vec2::new(CHARACTER_W, 424.0));
    add_window_frame(&mut app, WindowId::Friends, Vec2::new(384.0, 512.0));
    open_window(&mut app, WindowId::Character);
    open_window(&mut app, WindowId::Friends);
    assert!(frame_level(&app, "CharacterFrame") < frame_level(&app, "FriendsFrame"));

    press_left_at(&mut app, Vec2::new(40.0, 300.0), 1.0);
    app.update();

    let character = frame_level(&app, "CharacterFrame");
    let friends = frame_level(&app, "FriendsFrame");
    assert!(
        character > friends,
        "character {character} friends {friends}"
    );
    assert_eq!(frame_level(&app, "CharacterFrameChild"), character + 1);
    assert!(
        frame_level(&app, "FriendsFrameChild") < character,
        "the lower window's whole subtree sinks below the raised root"
    );
}

#[test]
fn inspect_window_evicted_by_panels_resets_the_snapshot() {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), 1.0);
    app.init_resource::<InspectStatusSnapshot>();
    app.update();
    app.world_mut()
        .resource_mut::<InspectStatusSnapshot>()
        .target_name = Some("Valeera".into());
    app.update();
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Inspect)
    );

    open_window(&mut app, WindowId::Character);
    open_window(&mut app, WindowId::Friends);

    assert_eq!(
        *app.world().resource::<InspectStatusSnapshot>(),
        InspectStatusSnapshot::default()
    );
}

#[test]
fn leaving_the_world_closes_every_window() {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), 1.0);
    open_window(&mut app, WindowId::Character);
    open_window(&mut app, WindowId::Bag(0));

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::CharSelect);
    app.update();

    assert!(!app.world().resource::<WindowManager>().any_open());
}

// --- Moved windows: drag, clamp, per-character persistence ---

pub(crate) fn temp_layout_path(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "ui-layout-{tag}-{}-{}.ron",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn select_character(app: &mut App, character_id: u64) {
    app.insert_resource(crate::networking::SelectedCharacterId {
        character_id: Some(character_id),
        character_name: Some(format!("Char{character_id}")),
    });
}

/// App with a layout store at `path` (loaded as on startup) and a character frame.
fn persistent_window_app(path: &std::path::Path, ui_scale: f32, character_id: u64) -> App {
    let mut app = window_app(Vec2::new(1920.0, 1080.0), ui_scale);
    app.insert_resource(crate::ui_layout_store::UiLayoutStore::load(
        path.to_path_buf(),
    ));
    select_character(&mut app, character_id);
    add_window_frame(&mut app, WindowId::Character, Vec2::new(CHARACTER_W, 424.0));
    app.update();
    app
}

fn move_cursor_holding_left(app: &mut App, cursor: Vec2, ui_scale: f32) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .unwrap()
        .set_cursor_position(Some(cursor * ui_scale));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.update();
}

fn release_left(app: &mut App) {
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.clear();
    mouse.release(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
}

/// Drags by the title region from `grab` to `drop` (UI units).
fn drag_title(app: &mut App, grab: Vec2, drop: Vec2, ui_scale: f32) {
    press_left_at(app, grab, ui_scale);
    move_cursor_holding_left(app, drop, ui_scale);
    release_left(app);
}

fn reopen(app: &mut App, id: WindowId) {
    app.world_mut().resource_mut::<WindowManager>().close(id);
    app.update();
    open_window(app, id);
}

#[test]
fn dragged_window_reopens_at_its_saved_position_only_for_that_character() {
    let path = temp_layout_path("drag");
    {
        let mut app = persistent_window_app(&path, 1.0, 11);
        open_window(&mut app, WindowId::Character);
        // Grab 10 units into the title region at (16,104), drop 300/200 away.
        drag_title(
            &mut app,
            Vec2::new(40.0, 114.0),
            Vec2::new(340.0, 314.0),
            1.0,
        );
        assert_eq!(
            frame_pos(&app, WindowId::Character),
            Vec2::new(316.0, 304.0)
        );

        reopen(&mut app, WindowId::Character);
        assert_eq!(
            frame_pos(&app, WindowId::Character),
            Vec2::new(316.0, 304.0)
        );
    }

    let mut same = persistent_window_app(&path, 1.0, 11);
    open_window(&mut same, WindowId::Character);
    assert_eq!(
        frame_pos(&same, WindowId::Character),
        Vec2::new(316.0, 304.0),
        "restart restores the character's saved position"
    );

    let mut other = persistent_window_app(&path, 1.0, 22);
    open_window(&mut other, WindowId::Character);
    assert_eq!(
        frame_pos(&other, WindowId::Character),
        Vec2::new(16.0, 104.0),
        "another character opens at the slot"
    );
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn press_below_the_title_region_does_not_move_the_window() {
    let path = temp_layout_path("body");
    let mut app = persistent_window_app(&path, 1.0, 11);
    open_window(&mut app, WindowId::Character);

    drag_title(
        &mut app,
        Vec2::new(40.0, 200.0),
        Vec2::new(340.0, 400.0),
        1.0,
    );

    assert_eq!(frame_pos(&app, WindowId::Character), Vec2::new(16.0, 104.0));
    assert!(!path.exists(), "nothing saved");
}

#[test]
fn dragged_window_clamps_to_screen_edges_at_ui_scale_four_thirds() {
    let path = temp_layout_path("clamp");
    let scale = 4.0 / 3.0;
    let mut app = persistent_window_app(&path, scale, 11);
    let screen = Vec2::new(1920.0 / scale, 1080.0 / scale);
    open_window(&mut app, WindowId::Character);

    // The cursor cannot leave the window; drop at its far corner.
    drag_title(&mut app, Vec2::new(40.0, 114.0), screen - Vec2::ONE, scale);
    assert_eq!(
        frame_pos(&app, WindowId::Character),
        Vec2::new(screen.x - CHARACTER_W, screen.y - 424.0)
    );

    drag_title(
        &mut app,
        Vec2::new(screen.x - CHARACTER_W + 10.0, screen.y - 420.0),
        Vec2::ZERO,
        scale,
    );
    assert_eq!(frame_pos(&app, WindowId::Character), Vec2::ZERO);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn reset_window_positions_returns_moved_windows_to_their_slots() {
    let path = temp_layout_path("reset");
    let mut app = persistent_window_app(&path, 1.0, 11);
    open_window(&mut app, WindowId::Character);
    drag_title(
        &mut app,
        Vec2::new(40.0, 114.0),
        Vec2::new(340.0, 314.0),
        1.0,
    );

    ResetWindowPositionsCommand.apply(app.world_mut());
    app.update();

    assert_eq!(frame_pos(&app, WindowId::Character), Vec2::new(16.0, 104.0));
    let reloaded = crate::ui_layout_store::UiLayoutStore::load(path.clone());
    assert_eq!(reloaded.window_position("11", "CharacterFrame"), None);
    std::fs::remove_file(&path).unwrap();
}
