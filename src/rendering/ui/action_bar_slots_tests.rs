use std::sync::mpsc::Receiver;
use std::time::Duration;

use super::*;
use game_engine::input_bindings::InputBindings;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::network_runtime::worker::NetworkCommand;
use game_engine::player_spells::{
    ActionDrag, PlayerSpellsPlugin, receive_action_bar_snapshot, receive_spell_cooldowns,
    tick_spell_timers,
};
use game_engine::spell_catalog::{SpellPowerCost, SpellRange};
use game_engine::ui::event::EventBus;
use shared::components::PowerEntry;
use shared::protocol::{ActionBarSnapshot, SpellCooldownUpdate};

#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod layout_test_support;

const CRUSADER_STRIKE: u32 = 35395;
const JUDGMENT: u32 = 20271;
const RAGE_SPELL: u32 = 12294;

fn catalog_spell(id: u32, name: &str, icon_fdid: u32) -> CatalogSpell {
    CatalogSpell {
        id,
        name: name.into(),
        icon_fdid,
        range: SpellRange {
            min_yd: [0.0, 0.0],
            max_yd: [5.0, 5.0],
        },
        cooldown: game_engine::spell_catalog::SpellCooldown {
            recovery_ms: 6000,
            category_recovery_ms: 0,
            gcd_ms: 1500,
        },
        ..Default::default()
    }
}

fn catalog() -> SpellCatalog {
    let mut judgment = catalog_spell(JUDGMENT, "Judgment", 135959);
    judgment.range.max_yd = [30.0, 30.0];
    let mut mortal_strike = catalog_spell(RAGE_SPELL, "Mortal Strike", 132355);
    mortal_strike.powers = vec![SpellPowerCost {
        power_type: 1,
        flat: 300,
        pct: 0.0,
        required_aura_spell_id: 0,
    }]
    .into();
    let tabs = game_engine::spell_catalog::SpellbookTabIndex {
        class_names: [(2, "Paladin".to_string())].into(),
        class_spells: [(CRUSADER_STRIKE, 2), (JUDGMENT, 2)].into(),
        ..Default::default()
    };
    SpellCatalog::ready(SpellCatalogData::from_parts(
        vec![
            catalog_spell(CRUSADER_STRIKE, "Crusader Strike", 135891),
            judgment,
            mortal_strike,
        ],
        tabs,
    ))
}

struct Fixture {
    app: App,
    window: Entity,
    commands: Receiver<NetworkCommand>,
}

fn fixture() -> Fixture {
    let mut app = App::new();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let bars = super::super::action_bar::create_action_bars(&mut registry);
    layout_test_support::compute_layout(&mut registry);
    app.insert_resource(UiState {
        registry,
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(bars);
    app.insert_resource(catalog());
    app.init_resource::<Time>();
    app.add_plugins(PlayerSpellsPlugin);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<InputBindings>();
    app.init_resource::<crate::ui_input_mode::UiInputMode>();
    app.init_resource::<CurrentTarget>();
    app.init_resource::<ReplicationMirrorMap>();
    app.init_resource::<Inbox<ActionBarSnapshot>>();
    app.init_resource::<Inbox<SpellCooldownUpdate>>();
    let (sender, commands) = std::sync::mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let mut window = Window::default();
    window.resolution.set(1920.0, 1080.0);
    let window = app
        .world_mut()
        .spawn((window, bevy::window::PrimaryWindow))
        .id();
    app.add_systems(
        Update,
        (
            receive_action_bar_snapshot,
            receive_spell_cooldowns,
            resolve_slot_widgets,
            handle_action_slot_keys,
            handle_action_slot_pointer,
            sync_action_slot_views,
            sync_drag_icon,
        )
            .chain()
            .after(tick_spell_timers),
    );
    app.update();
    Fixture {
        app,
        window,
        commands,
    }
}

impl Fixture {
    fn deliver_bar(&mut self, slots: Vec<(u8, ActionRef)>) {
        self.app
            .insert_resource(Inbox::new(vec![ActionBarSnapshot { slots }]));
        self.app.update();
    }

    fn registry(&self) -> &FrameRegistry {
        &self.app.world().resource::<UiState>().registry
    }

    fn frame(&self, name: &str) -> &game_engine::ui::frame::Frame {
        let registry = self.registry();
        registry
            .get(registry.get_by_name(name).expect(name))
            .expect(name)
    }

    fn icon(&self, name: &str) -> Option<(u32, [f32; 4])> {
        let frame = self.frame(name);
        if frame.hidden {
            return None;
        }
        match &frame.widget_data {
            Some(WidgetData::Texture(TextureData {
                source: TextureSource::FileDataId(fdid),
                vertex_color,
                ..
            })) => Some((*fdid, *vertex_color)),
            _ => None,
        }
    }

    fn text(&self, name: &str) -> String {
        match &self.frame(name).widget_data {
            Some(WidgetData::FontString(data)) => data.text.clone(),
            _ => panic!("{name} is not a font string"),
        }
    }

    fn point_at(&mut self, name: &str) {
        let rect = self.frame(name).layout_rect.clone().expect(name);
        let cursor = Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        self.point(cursor);
    }

    fn point(&mut self, cursor: Vec2) {
        self.app
            .world_mut()
            .get_mut::<Window>(self.window)
            .unwrap()
            .set_cursor_position(Some(cursor));
    }

    fn cursor(&self) -> Vec2 {
        self.app
            .world()
            .get::<Window>(self.window)
            .unwrap()
            .cursor_position()
            .unwrap()
    }

    fn mouse(&mut self, pressed: bool) {
        let mut mouse = self
            .app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        if pressed {
            mouse.press(MouseButton::Left);
        } else {
            mouse.release(MouseButton::Left);
        }
        self.app.update();
    }

    /// Press on `from`, move past the drag threshold, release on `to`.
    /// Hover first so a hover rebuild of the source (spellbook rows) is laid out again.
    fn drag(&mut self, from: &str, to: Option<&str>) {
        self.point_at(from);
        self.app.update();
        self.relayout();
        self.point_at(from);
        self.mouse(true);
        let moved = self.cursor() + Vec2::new(20.0, 0.0);
        self.point(moved);
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        self.app.update();
        match to {
            Some(name) => self.point_at(name),
            None => self.point(Vec2::new(960.0, 300.0)),
        }
        self.mouse(false);
    }

    fn set_shift(&mut self, held: bool) {
        let mut keys = self.app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if held {
            keys.press(KeyCode::ShiftLeft);
        } else {
            keys.release(KeyCode::ShiftLeft);
        }
    }

    fn slot(&self, slot: usize) -> Option<ActionRef> {
        self.app.world().resource::<ActionBarSlots>().get(slot)
    }

    fn spawn_player(&mut self, powers: Vec<PowerEntry>) -> Entity {
        self.app
            .world_mut()
            .spawn((
                LocalPlayer,
                Transform::default(),
                UnitPowers {
                    entries: powers,
                    charged_points: Vec::new(),
                },
            ))
            .id()
    }

    fn target_at(&mut self, distance: f32) -> (Entity, Entity) {
        let main = self
            .app
            .world_mut()
            .spawn((
                Transform::from_xyz(distance, 0.0, 0.0),
                Npc {
                    template_id: 1,
                    name: "Training Dummy".into(),
                },
            ))
            .id();
        let server = self.app.world_mut().spawn_empty().id();
        self.app
            .world_mut()
            .resource_mut::<ReplicationMirrorMap>()
            .insert(server, main);
        self.app.world_mut().resource_mut::<CurrentTarget>().0 = Some(main);
        (main, server)
    }

    fn relayout(&mut self) {
        let mut ui = self.app.world_mut().resource_mut::<UiState>();
        let mut registry = std::mem::replace(&mut ui.registry, FrameRegistry::new(1920.0, 1080.0));
        layout_test_support::compute_layout(&mut registry);
        ui.registry = registry;
    }

    fn sent(&self) -> (Vec<SpellCastIntent>, Vec<SetActionButton>) {
        let commands: Vec<_> = self.commands.try_iter().collect();
        loopback_sent(commands)
    }
}

/// Runs queued worker commands against a loopback transport and returns what it serialized.
fn loopback_sent(commands: Vec<NetworkCommand>) -> (Vec<SpellCastIntent>, Vec<SetActionButton>) {
    use lightyear::prelude::client::ClientPlugins;
    use lightyear::prelude::{
        ChannelRegistry, Connected, Link, Linked, MessageReceiver, MessageSender, PeerId, RemoteId,
        Transport,
    };
    let mut worker = App::new();
    worker.add_plugins(bevy::state::app::StatesPlugin);
    worker.add_plugins(ClientPlugins::default());
    worker.add_plugins(shared::ProtocolPlugin);
    worker.finish();
    worker.cleanup();
    let registry = worker.world().resource::<ChannelRegistry>();
    let mut transport = Transport::default();
    transport.add_sender_from_registry::<CombatChannel>(registry);
    transport.add_receiver_from_registry::<CombatChannel>(registry);
    transport.add_sender_from_registry::<TalentChannel>(registry);
    transport.add_receiver_from_registry::<TalentChannel>(registry);
    let peer = worker
        .world_mut()
        .spawn((
            Link::default(),
            transport,
            Linked,
            Connected,
            RemoteId(PeerId::Local(0)),
            MessageSender::<SpellCastIntent>::default(),
            MessageReceiver::<SpellCastIntent>::default(),
            MessageSender::<SetActionButton>::default(),
            MessageReceiver::<SetActionButton>::default(),
        ))
        .id();
    for command in commands {
        let NetworkCommand::Apply(apply) = command else {
            panic!("expected a message, not worker shutdown");
        };
        apply(worker.world_mut());
    }
    worker.world_mut().run_schedule(PostUpdate);
    {
        let mut entity = worker.world_mut().entity_mut(peer);
        let mut link = entity.get_mut::<Link>().unwrap();
        let packets: Vec<_> = link.send.drain().collect();
        for packet in packets {
            link.recv.push_raw(packet);
        }
    }
    worker.world_mut().run_schedule(PreUpdate);
    let mut entity = worker.world_mut().entity_mut(peer);
    let casts = entity
        .get_mut::<MessageReceiver<SpellCastIntent>>()
        .unwrap()
        .receive()
        .collect();
    let buttons = entity
        .get_mut::<MessageReceiver<SetActionButton>>()
        .unwrap()
        .receive()
        .collect();
    (casts, buttons)
}

#[test]
fn snapshot_slot_shows_spell_icon() {
    let mut f = fixture();
    f.deliver_bar(vec![
        (0, ActionRef::Spell(CRUSADER_STRIKE)),
        (13, ActionRef::Spell(JUDGMENT)),
    ]);
    assert_eq!(f.icon("ActionButton1_1Icon"), Some((135891, TINT_NORMAL)));
    assert_eq!(f.icon("ActionButton2_2Icon"), Some((135959, TINT_NORMAL)));
    assert_eq!(f.icon("ActionButton1_2Icon"), None);
}

#[test]
fn key_one_casts_slot_spell_at_current_target_server_entity() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    let (_, server) = f.target_at(3.0);
    f.app
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Digit1);
    f.app.update();
    let (casts, _) = f.sent();
    assert_eq!(casts.len(), 1);
    assert_eq!(casts[0].spell_id, Some(CRUSADER_STRIKE));
    assert_eq!(casts[0].target_entity, Some(server.to_bits()));
}

#[test]
fn key_one_does_nothing_outside_world_mode() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    *f.app
        .world_mut()
        .resource_mut::<crate::ui_input_mode::UiInputMode>() =
        crate::ui_input_mode::UiInputMode::Text;
    f.app
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Digit1);
    f.app.update();
    assert!(f.commands.try_recv().is_err());
}

#[test]
fn clicking_a_slot_casts_its_spell() {
    let mut f = fixture();
    f.deliver_bar(vec![(1, ActionRef::Spell(JUDGMENT))]);
    f.point_at("ActionButton1_2");
    f.mouse(true);
    f.mouse(false);
    let (casts, _) = f.sent();
    assert_eq!(casts.len(), 1);
    assert_eq!(casts[0].spell_id, Some(JUDGMENT));
    assert_eq!(casts[0].target_entity, None);
}

#[test]
fn cooldown_shows_on_slot_and_clears_after_it_elapses() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    f.app.insert_resource(Inbox::new(vec![SpellCooldownUpdate {
        spell_id: CRUSADER_STRIKE,
        category: 0,
        duration_ms: 6000,
        remaining_ms: 6000,
        is_gcd: false,
    }]));
    f.app.update();
    assert_eq!(f.text("ActionButton1_1CooldownText"), "6");
    assert!(!f.frame("ActionButton1_1Cooldown").hidden);
    f.app
        .world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(3500));
    f.app.update();
    assert_eq!(f.text("ActionButton1_1CooldownText"), "3");
    assert_eq!(
        f.frame("ActionButton1_1Cooldown").height,
        Dimension::Fixed(19.0)
    );
    f.app
        .world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(2600));
    f.app.update();
    assert_eq!(f.text("ActionButton1_1CooldownText"), "");
    assert!(f.frame("ActionButton1_1Cooldown").hidden);
}

#[test]
fn gcd_wipes_slots_without_countdown_text() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    f.app.insert_resource(Inbox::new(vec![SpellCooldownUpdate {
        spell_id: JUDGMENT,
        category: 133,
        duration_ms: 1500,
        remaining_ms: 1500,
        is_gcd: true,
    }]));
    f.app.update();
    assert!(!f.frame("ActionButton1_1Cooldown").hidden);
    assert_eq!(f.text("ActionButton1_1CooldownText"), "");
}

#[test]
fn slot_dims_when_cost_exceeds_current_power() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(RAGE_SPELL))]);
    let player = f.spawn_player(vec![PowerEntry {
        power: PowerType::Rage,
        current: 200,
        max: 1000,
        partial: 0,
        regen_per_sec: 0.0,
    }]);
    f.app.update();
    assert_eq!(f.icon("ActionButton1_1Icon"), Some((132355, TINT_NO_POWER)));
    f.app
        .world_mut()
        .get_mut::<UnitPowers>(player)
        .unwrap()
        .entries[0]
        .current = 300;
    f.app.update();
    assert_eq!(f.icon("ActionButton1_1Icon"), Some((132355, TINT_NORMAL)));
}

#[test]
fn slot_tints_red_when_target_is_beyond_max_range() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(JUDGMENT))]);
    f.spawn_player(Vec::new());
    let (target, _) = f.target_at(40.0);
    f.app.update();
    assert_eq!(
        f.icon("ActionButton1_1Icon"),
        Some((135959, TINT_OUT_OF_RANGE))
    );
    f.app
        .world_mut()
        .get_mut::<Transform>(target)
        .unwrap()
        .translation
        .x = 25.0;
    f.app.update();
    assert_eq!(f.icon("ActionButton1_1Icon"), Some((135959, TINT_NORMAL)));
}

#[test]
fn spellbook_drop_on_slot_two_sets_action_button() {
    let mut f = fixture();
    f.app.world_mut().resource_mut::<ActionDrag>().0 = Some(DraggedAction {
        action: ActionRef::Spell(JUDGMENT),
        source: DragSource::Spellbook,
    });
    f.point_at("ActionButton1_2");
    f.mouse(true);
    f.mouse(false);
    let (casts, buttons) = f.sent();
    assert!(casts.is_empty(), "a drop must not cast");
    assert_eq!(
        buttons,
        [SetActionButton {
            slot: 1,
            action: Some(ActionRef::Spell(JUDGMENT)),
        }]
    );
    assert_eq!(f.slot(1), Some(ActionRef::Spell(JUDGMENT)));
    assert_eq!(f.app.world().resource::<ActionDrag>().0, None);
    assert_eq!(f.icon("ActionButton1_2Icon"), Some((135959, TINT_NORMAL)));
}

#[test]
fn shift_drag_between_slots_swaps_them() {
    let mut f = fixture();
    f.deliver_bar(vec![
        (0, ActionRef::Spell(CRUSADER_STRIKE)),
        (2, ActionRef::Spell(JUDGMENT)),
    ]);
    f.set_shift(true);
    f.drag("ActionButton1_1", Some("ActionButton1_3"));
    let (casts, buttons) = f.sent();
    assert!(casts.is_empty(), "a drag must not cast");
    assert_eq!(
        buttons,
        [
            SetActionButton {
                slot: 2,
                action: Some(ActionRef::Spell(CRUSADER_STRIKE)),
            },
            SetActionButton {
                slot: 0,
                action: Some(ActionRef::Spell(JUDGMENT)),
            },
        ]
    );
    assert_eq!(f.slot(0), Some(ActionRef::Spell(JUDGMENT)));
    assert_eq!(f.slot(2), Some(ActionRef::Spell(CRUSADER_STRIKE)));
}

#[test]
fn shift_drag_off_the_bar_clears_the_slot() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    f.set_shift(true);
    f.drag("ActionButton1_1", None);
    let (_, buttons) = f.sent();
    assert_eq!(
        buttons,
        [SetActionButton {
            slot: 0,
            action: None,
        }]
    );
    assert_eq!(f.slot(0), None);
    assert_eq!(f.icon("ActionButton1_1Icon"), None);
}

#[test]
fn drag_without_shift_leaves_locked_bar_unchanged() {
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    f.drag("ActionButton1_1", None);
    let (casts, buttons) = f.sent();
    assert!(casts.is_empty() && buttons.is_empty());
    assert_eq!(f.slot(0), Some(ActionRef::Spell(CRUSADER_STRIKE)));
}

#[test]
fn drag_icon_follows_cursor_while_dragging() {
    let mut f = fixture();
    f.app.world_mut().resource_mut::<ActionDrag>().0 = Some(DraggedAction {
        action: ActionRef::Spell(JUDGMENT),
        source: DragSource::Spellbook,
    });
    f.point(Vec2::new(500.0, 400.0));
    f.app.update();
    assert_eq!(f.icon(DRAG_ICON_NAME), Some((135959, TINT_NORMAL)));
    assert!(!f.frame(DRAG_ICON_NAME).mouse_enabled);
    f.app.world_mut().resource_mut::<ActionDrag>().0 = None;
    f.app.update();
    assert!(f.frame(DRAG_ICON_NAME).hidden);
}

#[test]
fn unchanged_slots_do_not_dirty_the_ui() {
    #[derive(Resource, Default)]
    struct UiChanged(usize);
    fn record(ui: Res<UiState>, mut changed: ResMut<UiChanged>) {
        if ui.is_changed() {
            changed.0 += 1;
        }
    }
    let mut f = fixture();
    f.deliver_bar(vec![(0, ActionRef::Spell(CRUSADER_STRIKE))]);
    f.app.init_resource::<UiChanged>();
    f.app.add_systems(Last, record);
    f.point(Vec2::new(960.0, 300.0));
    f.app.update();
    f.app.update();
    let before = f.app.world().resource::<UiChanged>().0;
    for _ in 0..5 {
        f.app.update();
    }
    assert_eq!(f.app.world().resource::<UiChanged>().0, before);
}

#[test]
fn dragging_spellbook_entry_onto_slot_two_sends_set_action_button() {
    use game_engine::player_spells::KnownSpells;
    use game_engine::ui::game_plugin::{register_spellbook_frame_systems, set_spellbook_open};
    use game_engine::ui::spellbook_runtime::SpellbookUiRuntime;
    let mut f = fixture();
    f.app.add_message::<bevy::input::keyboard::KeyboardInput>();
    register_spellbook_frame_systems(&mut f.app);
    f.app
        .insert_resource(KnownSpells::new(vec![CRUSADER_STRIKE, JUDGMENT]));
    f.app
        .world_mut()
        .resource_scope(|world, mut state: Mut<UiState>| {
            let mut runtime = world.non_send_mut::<SpellbookUiRuntime>();
            set_spellbook_open(&mut state, &mut runtime, true);
        });
    f.app.update();
    f.relayout();
    assert_eq!(f.text("SpellBookSpellName2"), "Judgment");
    f.drag("SpellBookSpellName2", Some("ActionButton1_2"));
    let (casts, buttons) = f.sent();
    assert!(
        casts.is_empty(),
        "dragging from the spellbook must not cast"
    );
    assert_eq!(
        buttons,
        [SetActionButton {
            slot: 1,
            action: Some(ActionRef::Spell(JUDGMENT)),
        }]
    );
    assert_eq!(f.icon("ActionButton1_2Icon"), Some((135959, TINT_NORMAL)));
}
