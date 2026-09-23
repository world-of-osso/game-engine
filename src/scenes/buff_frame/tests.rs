use bevy::ecs::message::Messages;
use game_engine::buff_data::{AuraInstance, DebuffType};
use game_engine::ui::event::EventBus;
use ui_toolkit::layout::LayoutRect;

use super::native_layout_support::compute_layout;
use super::*;

fn aura(spell_id: u32, is_debuff: bool, remaining: f32) -> AuraInstance {
    AuraInstance {
        instance_id: spell_id,
        spell_id,
        name: format!("Spell {spell_id}"),
        description: String::new(),
        icon_fdid: 135987,
        source: String::new(),
        from_local_player: true,
        duration: 600.0,
        remaining,
        stacks: 1,
        is_debuff,
        debuff_type: if is_debuff {
            DebuffType::Curse
        } else {
            DebuffType::None
        },
    }
}

fn app(auras: Vec<AuraInstance>) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorld);
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(AuraState { auras });
    app.init_resource::<ButtonInput<MouseButton>>();
    app.add_message::<CancelAuraRequest>();
    app.add_plugins(BuffFramePlugin);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.update();
    layout(&mut app);
    (app, window)
}

fn layout(app: &mut App) {
    compute_layout(&mut app.world_mut().resource_mut::<UiState>().registry);
}

fn rect(app: &App, name: &str) -> LayoutRect {
    let registry = &app.world().resource::<UiState>().registry;
    let id = registry.get_by_name(name).expect(name);
    registry.get(id).unwrap().layout_rect.clone().unwrap()
}

fn text(app: &App, name: &str) -> String {
    let registry = &app.world().resource::<UiState>().registry;
    let id = registry.get_by_name(name).expect(name);
    match &registry.get(id).unwrap().widget_data {
        Some(ui_toolkit::frame::WidgetData::FontString(fs)) => fs.text.clone(),
        other => panic!("{name} is not a fontstring: {other:?}"),
    }
}

fn right_click(app: &mut App, window: Entity, name: &str) -> Vec<CancelAuraRequest> {
    let target = rect(app, name);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(Vec2::new(
            target.x + target.width / 2.0,
            target.y + target.height / 2.0,
        )));
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release_all();
    mouse.clear();
    mouse.press(MouseButton::Right);
    app.update();
    app.world_mut()
        .resource_mut::<Messages<CancelAuraRequest>>()
        .drain()
        .collect()
}

#[test]
fn mounts_buffs_and_debuffs_from_aura_state() {
    let (app, _) = app(vec![aura(21562, false, 300.0), aura(980, true, 20.0)]);
    let registry = &app.world().resource::<UiState>().registry;
    assert!(registry.get_by_name("BuffFrame").is_some());
    assert!(registry.get_by_name("BuffButton0").is_some());
    assert!(registry.get_by_name("BuffButton1").is_none());
    assert!(registry.get_by_name("DebuffButton0").is_some());
    let border = registry.get_by_name("DebuffButton0Border").unwrap();
    assert_eq!(
        registry.get(border).unwrap().background_color,
        Some([0.6, 0.0, 1.0, 1.0])
    );
    assert_eq!(text(&app, "BuffButton0Duration"), "5 m");
}

#[test]
fn duration_text_follows_the_ticking_aura_state() {
    let (mut app, _) = app(vec![aura(21562, false, 61.0)]);
    assert_eq!(text(&app, "BuffButton0Duration"), "2 m");
    app.world_mut().resource_mut::<AuraState>().tick(2.0);
    app.update();
    assert_eq!(text(&app, "BuffButton0Duration"), "59 s");
}

#[test]
fn right_click_on_a_buff_requests_cancel_and_debuffs_do_not() {
    let (mut app, window) = app(vec![
        aura(21562, false, 300.0),
        aura(1126, false, 300.0),
        aura(980, true, 20.0),
    ]);
    assert_eq!(
        right_click(&mut app, window, "BuffButton1Icon"),
        [CancelAuraRequest { spell_id: 1126 }]
    );
    assert_eq!(right_click(&mut app, window, "DebuffButton0Icon"), []);
}
