use crate::network_runtime::messages::MessageSenders;
use crate::network_runtime::replication::ReplicationMirrorMap;
use crate::player_spells::{
    ActionDrag, ActiveSpecialization, DragSource, DraggedAction, KnownSpells, server_target_bits,
    spell_cast_intent,
};
use crate::spell_catalog::SpellCatalog;
use bevy::prelude::*;
use bevy::{input::ButtonState, input::keyboard::KeyboardInput};

use crate::targeting::CurrentTarget;
use crate::ui::input::ui_cursor_position;
use crate::ui::plugin::UiState;
use crate::ui::spellbook_data::build_spellbook_tabs;
use crate::ui::spellbook_runtime::{SpellbookAction, SpellbookKeyInput, SpellbookUiRuntime};
use shared::protocol::{ActionRef, CombatChannel, SpellCastIntent};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpellbookUiSystems {
    Sync,
    Input,
}

pub fn register_spellbook_frame_systems(app: &mut App) {
    app.insert_non_send(SpellbookUiRuntime::new());
    app.configure_sets(
        Update,
        (SpellbookUiSystems::Sync, SpellbookUiSystems::Input).chain(),
    );
    app.add_systems(
        Update,
        (sync_spellbook_model, sync_screen_ui)
            .chain()
            .in_set(SpellbookUiSystems::Sync),
    );
    app.add_systems(
        Update,
        (handle_spellbook_pointer, handle_spellbook_keyboard)
            .chain()
            .in_set(SpellbookUiSystems::Input),
    );
}

/// Rebuilds the spellbook tabs when the known spells, spec or catalog change.
pub fn sync_spellbook_model(
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
    known: Option<Res<KnownSpells>>,
    spec: Option<Res<ActiveSpecialization>>,
    catalog: Option<Res<SpellCatalog>>,
) {
    let Some(mut runtime) = runtime else { return };
    let changed = runtime.is_added()
        || known.as_ref().is_some_and(|known| known.is_changed())
        || spec.as_ref().is_some_and(|spec| spec.is_changed())
        || catalog.as_ref().is_some_and(|catalog| catalog.is_changed());
    if !changed {
        return;
    }
    let known = known.as_deref().map_or(&[][..], KnownSpells::spells);
    let spec_id = spec.and_then(|spec| spec.0);
    let tabs = build_spellbook_tabs(
        known,
        spec_id,
        catalog.as_deref().and_then(SpellCatalog::data),
        None,
    );
    runtime
        .bypass_change_detection()
        .set_tabs(&mut state.bypass_change_detection().registry, tabs);
}

pub fn sync_screen_ui(mut state: ResMut<UiState>, runtime: Option<NonSendMut<SpellbookUiRuntime>>) {
    if let Some(mut runtime) = runtime
        && (state.is_changed() || runtime.is_changed())
    {
        // Registry mutations carry their own render/layout dirtiness. Consuming
        // an invalidation must not publish another UiState/runtime invalidation.
        runtime
            .bypass_change_detection()
            .sync(&mut state.bypass_change_detection().registry);
    }
}

/// Shows or hides the spellbook; the engine window manager owns the open state.
pub fn set_spellbook_open(state: &mut UiState, runtime: &mut SpellbookUiRuntime, open: bool) {
    if runtime.is_open() != open {
        runtime.set_open(&mut state.registry, open);
    }
}

pub fn teardown_spellbook_ui(
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
) {
    if let Some(mut runtime) = runtime {
        runtime.teardown(&mut state.registry);
        runtime.set_open(&mut state.registry, false);
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct SpellCastTarget<'w> {
    current_target: Option<Res<'w, CurrentTarget>>,
    mirror: Option<Res<'w, ReplicationMirrorMap>>,
}

impl SpellCastTarget<'_> {
    pub fn server_bits(&self) -> Option<u64> {
        server_target_bits(self.current_target.as_deref(), self.mirror.as_deref())
    }
}

pub fn handle_spellbook_pointer(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
    target: SpellCastTarget,
    drag: Option<ResMut<ActionDrag>>,
    mut spell_senders: MessageSenders<SpellCastIntent>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let (Ok(window), Some(mut runtime)) = (windows.single(), runtime) else {
        return;
    };
    let Some(cursor) = ui_cursor_position(&state.registry, window) else {
        *last_cursor = None;
        return;
    };

    let (x, y) = (cursor.x, cursor.y);
    if *last_cursor != Some(cursor) || state.is_changed() || runtime.is_changed() {
        let changed = runtime.bypass_change_detection().handle_pointer_move(
            &mut state.bypass_change_detection().registry,
            x,
            y,
        );
        if changed {
            state.set_changed();
        }
        *last_cursor = Some(cursor);
    }

    let Some(mouse) = mouse else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        let _ = runtime.handle_pointer_button(&mut state.registry, true, x, y);
    } else if mouse.pressed(MouseButton::Left)
        && let Some(mut drag) = drag
        && drag.0.is_none()
        && let Some(spell_id) = runtime.take_drag_start(&mut state.registry, x, y)
    {
        drag.0 = Some(DraggedAction {
            action: ActionRef::Spell(spell_id),
            source: DragSource::Spellbook,
        });
    }
    if mouse.just_released(MouseButton::Left)
        && let Some(action) = runtime.handle_pointer_button(&mut state.registry, false, x, y)
    {
        send_spellbook_action(action, target.server_bits(), &mut spell_senders);
    }
}
pub fn handle_spellbook_keyboard(
    mut key_events: Option<MessageReader<KeyboardInput>>,
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
) {
    let Some(mut runtime) = runtime else { return };
    if !runtime.has_focus() {
        return;
    }
    let Some(mut key_events) = key_events.take() else {
        return;
    };

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        if let bevy::input::keyboard::Key::Character(text) = &event.logical_key {
            for ch in text.chars() {
                let _ =
                    runtime.handle_key_input(&mut state.registry, SpellbookKeyInput::Character(ch));
            }
            continue;
        }

        let key = match event.key_code {
            KeyCode::ArrowLeft => Some(SpellbookKeyInput::PreviousTab),
            KeyCode::ArrowRight => Some(SpellbookKeyInput::NextTab),
            KeyCode::PageUp => Some(SpellbookKeyInput::PreviousPage),
            KeyCode::PageDown => Some(SpellbookKeyInput::NextPage),
            KeyCode::Backspace => Some(SpellbookKeyInput::Backspace),
            _ => None,
        };

        if let Some(key) = key {
            let _ = runtime.handle_key_input(&mut state.registry, key);
        }
    }
}

#[cfg(test)]
mod idle_tests {
    use super::*;
    use crate::network_runtime::messages::ConnectionSender;
    use crate::network_runtime::worker::NetworkCommand;
    use crate::spell_catalog::{CatalogSpell, SpellCatalogData, SpellbookTabIndex};
    use crate::ui::spellbook_runtime::test_support::{center_of, simulate_layout_readback_at};
    use crate::ui::{event::EventBus, frame::WidgetData, registry::FrameRegistry};

    #[derive(Resource, Default)]
    struct Changes(usize);

    /// Where the layout pass places the spellbook root.
    #[derive(Resource)]
    struct LayoutOrigin(Vec2);

    /// Stands in for the UI layout pass, which reads back every frame.
    fn readback_layout(mut state: ResMut<UiState>, origin: Res<LayoutOrigin>) {
        simulate_layout_readback_at(&mut state.bypass_change_detection().registry, origin.0);
    }

    fn record_changes(state: Res<UiState>, mut changes: ResMut<Changes>) {
        if state.is_changed() {
            changes.0 += 1;
        }
    }

    fn catalog() -> SpellCatalog {
        let spell = |id, name: &str| CatalogSpell {
            id,
            name: name.into(),
            icon_fdid: id + 1,
            ..Default::default()
        };
        let tabs = SpellbookTabIndex {
            class_names: [(2, "Paladin".to_string())].into(),
            class_spells: [(35395, 2), (20271, 2)].into(),
            ..Default::default()
        };
        SpellCatalog::ready(SpellCatalogData::from_parts(
            vec![
                spell(6603, "Auto Attack"),
                spell(35395, "Crusader Strike"),
                spell(20271, "Judgment"),
            ],
            tabs,
        ))
    }

    fn set_open(app: &mut App, open: bool) {
        app.world_mut()
            .resource_scope(|world, mut state: Mut<UiState>| {
                let mut runtime = world.non_send_mut::<SpellbookUiRuntime>();
                set_spellbook_open(&mut state, &mut runtime, open);
            });
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            focused_frame: None,
        });
        app.insert_resource(catalog());
        app.insert_resource(KnownSpells::new(vec![6603, 35395, 20271]));
        app.init_resource::<ActiveSpecialization>();
        app.init_resource::<ActionDrag>();
        app.init_resource::<Changes>();
        app.init_resource::<ConnectionSender>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app
    }

    fn production_app() -> (App, Entity) {
        let mut app = app();
        let mut window = Window::default();
        window.resolution.set(1920.0, 1080.0);
        window.set_cursor_position(Some(Vec2::new(1900.0, 1000.0)));
        let window = app
            .world_mut()
            .spawn((window, bevy::window::PrimaryWindow))
            .id();
        app.add_message::<KeyboardInput>();
        register_spellbook_frame_systems(&mut app);
        app.insert_resource(LayoutOrigin(Vec2::new(80.0, 120.0)));
        app.add_systems(PostUpdate, readback_layout);
        app.add_systems(Last, record_changes);
        set_open(&mut app, true);
        app.update();
        (app, window)
    }

    fn text(app: &App, name: &str) -> String {
        let registry = &app.world().resource::<UiState>().registry;
        let id = registry.get_by_name(name).expect(name);
        let Some(WidgetData::FontString(text)) = &registry.get(id).unwrap().widget_data else {
            panic!("{name} must be a font string");
        };
        text.text.clone()
    }

    fn first_spell(app: &App) -> String {
        text(app, "SpellBookSpellName1")
    }

    /// Moves the cursor and runs one frame, so hover rebuilds are laid out before a press.
    fn point_at(app: &mut App, window: Entity, name: &str) {
        let (x, y) = center_of(&app.world().resource::<UiState>().registry, name);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(x, y)));
        app.update();
    }

    fn assert_idle(app: &mut App) {
        // Consume the last real output invalidation before measuring idle work.
        app.update();
        app.update();
        let initial_changes = app.world().resource::<Changes>().0;
        let initial_calls = app
            .world()
            .non_send::<SpellbookUiRuntime>()
            .execution_counts;
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, initial_changes);
        assert_eq!(
            app.world()
                .non_send::<SpellbookUiRuntime>()
                .execution_counts,
            initial_calls,
            "idle frames must not sync screens or hit-test the pointer"
        );
    }

    fn click(app: &mut App) {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.press(MouseButton::Left);
        app.update();
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.release(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
    }

    #[test]
    fn known_spells_snapshot_lists_catalog_names_on_class_tab() {
        let (app, _) = production_app();
        assert_eq!(text(&app, "SpellBookTabLabel2"), "Paladin");
        assert_eq!(text(&app, "SpellBookSpellName1"), "Crusader Strike");
        assert_eq!(text(&app, "SpellBookSpellName2"), "Judgment");
        let registry = &app.world().resource::<UiState>().registry;
        assert!(registry.get_by_name("SpellBookSpell3").is_none());
    }

    #[test]
    fn learned_spell_appears_without_reopening() {
        let (mut app, _) = production_app();
        *app.world_mut().resource_mut::<KnownSpells>() = KnownSpells::new(vec![20271]);
        app.update();
        assert_eq!(first_spell(&app), "Judgment");
    }

    #[test]
    fn production_spellbook_registration_settles_without_idle_work() {
        let (mut app, window) = production_app();
        assert_idle(&mut app);
        assert_eq!(first_spell(&app), "Crusader Strike");
        point_at(&mut app, window, "SpellBookTabPanel1");
        assert_idle(&mut app);
    }

    fn tab_color(app: &App) -> Option<[f32; 4]> {
        let registry = &app.world().resource::<UiState>().registry;
        registry
            .get(registry.get_by_name("SpellBookTabPanel1").unwrap())
            .unwrap()
            .background_color
    }

    #[test]
    fn production_layout_change_rechecks_stationary_hover_then_settles() {
        let (mut app, window) = production_app();
        assert_idle(&mut app);
        let normal = tab_color(&app);
        point_at(&mut app, window, "SpellBookTabPanel1");
        assert_idle(&mut app);
        assert_ne!(tab_color(&app), normal);
        let moved = Vec2::new(2080.0, 120.0);
        app.world_mut().resource_mut::<LayoutOrigin>().0 = moved;
        // A layout pass that moved the frames under the stationary cursor.
        simulate_layout_readback_at(
            &mut app.world_mut().resource_mut::<UiState>().registry,
            moved,
        );
        assert_idle(&mut app);
        assert_eq!(tab_color(&app), normal);
    }

    #[test]
    fn production_click_release_sends_once_and_keyboard_remains_responsive() {
        let (mut app, window) = production_app();
        let (sender, receiver) = std::sync::mpsc::channel();
        app.insert_resource(ConnectionSender::new(Some(sender)));
        assert_idle(&mut app);
        point_at(&mut app, window, "SpellBookSpellName1");
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(receiver.try_recv().is_err());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(matches!(receiver.try_recv(), Ok(NetworkCommand::Apply(_))));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        assert_idle(&mut app);
        assert!(receiver.try_recv().is_err());
        for ch in ["j", "u", "d"] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyJ,
                logical_key: bevy::input::keyboard::Key::Character(ch.into()),
                state: ButtonState::Pressed,
                text: Some(ch.into()),
                repeat: false,
                window,
            });
        }
        app.update();
        assert_eq!(first_spell(&app), "Judgment");
        assert_idle(&mut app);
    }

    #[test]
    fn dragging_a_spell_puts_it_on_the_cursor() {
        let (mut app, window) = production_app();
        point_at(&mut app, window, "SpellBookSpellName2");
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        let (x, y) = center_of(
            &app.world().resource::<UiState>().registry,
            "SpellBookSpellName2",
        );
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(x + 40.0, y)));
        app.update();
        assert_eq!(
            app.world().resource::<ActionDrag>().0,
            Some(DraggedAction {
                action: ActionRef::Spell(20271),
                source: DragSource::Spellbook,
            })
        );
    }

    #[test]
    fn closing_the_spellbook_hides_it_and_stops_clicks() {
        let (mut app, window) = production_app();
        point_at(&mut app, window, "SpellBookTabPanel1");
        set_open(&mut app, false);
        app.update();
        click(&mut app);
        let registry = &app.world().resource::<UiState>().registry;
        let root = registry.get_by_name("SpellBookRoot").unwrap();
        assert!(!registry.get(root).unwrap().visible);
        assert_eq!(first_spell(&app), "Crusader Strike", "tab click ignored");
    }

    #[test]
    fn screen_sync_preserves_initial_output_without_idle_changes() {
        let mut app = app();
        app.insert_non_send(SpellbookUiRuntime::new());
        app.add_systems(
            Update,
            (sync_spellbook_model, sync_screen_ui, record_changes).chain(),
        );
        app.update();
        assert_eq!(first_spell(&app), "Crusader Strike");
        let initial = app.world().resource::<Changes>().0;
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, initial);
        app.world_mut().resource_mut::<UiState>().focused_frame = Some(0);
        app.update();
        assert_eq!(first_spell(&app), "Crusader Strike");
        app.world_mut().insert_non_send(SpellbookUiRuntime::new());
        app.update();
        assert_eq!(first_spell(&app), "Crusader Strike");
    }

    fn pointer_app() -> (App, Entity) {
        let (mut app, window) = production_app();
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(1900.0, 1000.0)));
        app.update();
        (app, window)
    }

    #[test]
    fn stationary_pointer_is_idle_but_buttons_still_select_tabs() {
        let (mut app, window) = pointer_app();
        point_at(&mut app, window, "SpellBookTabPanel1");
        app.update();
        let initial = app.world().resource::<Changes>().0;
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, initial);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().non_send::<SpellbookUiRuntime>().has_focus());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert_eq!(first_spell(&app), "Auto Attack");
    }

    #[test]
    fn pointer_rechecks_cursor_and_ui_changes() {
        let (mut app, window) = pointer_app();
        app.update();
        let initial = app.world().resource::<Changes>().0;
        point_at(&mut app, window, "SpellBookTabPanel1");
        app.update();
        assert!(app.world().resource::<Changes>().0 > initial);
        let before = app.world().resource::<Changes>().0;
        let calls = app
            .world()
            .non_send::<SpellbookUiRuntime>()
            .execution_counts
            .1;
        app.world_mut().resource_mut::<UiState>().focused_frame = Some(0);
        app.update();
        assert!(app.world().resource::<Changes>().0 > before);
        assert_eq!(
            app.world()
                .non_send::<SpellbookUiRuntime>()
                .execution_counts
                .1,
            calls + 1,
            "a UI change re-hit-tests a stationary pointer"
        );
    }
}

fn send_spellbook_action(
    action: SpellbookAction,
    target_bits: Option<u64>,
    spell_senders: &mut MessageSenders<SpellCastIntent>,
) {
    let SpellbookAction::CastSpell {
        spell_id,
        spell_name,
    } = action;
    let intent = spell_cast_intent(spell_id, &spell_name, target_bits);
    for mut sender in spell_senders.iter_mut() {
        sender.send::<CombatChannel>(intent.clone());
    }
}
