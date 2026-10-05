use game_engine_ui_model::main_action_bar_component::{
    ActionBar, ActionButtonView, MAIN_ACTION_BAR, MainActionBarState, main_action_bar_screen,
    parse_action_button,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

const SLAM_ICON: u32 = 132340;

fn build(state: MainActionBarState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(state);
    Screen::new(main_action_bar_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

fn source(registry: &FrameRegistry, name: &str) -> TextureSource {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => texture.source.clone(),
        other => panic!("{name} is not a Texture: {other:?}"),
    }
}

/// Modern preset: 12 × 45 px buttons 2 px apart, BOTTOM y 45, keys 1..=.
#[test]
fn main_bar_uses_retail_geometry_and_default_keys() {
    let mut state = MainActionBarState::default();
    state.set_hotkeys(&game_engine_ui_model::input_bindings::InputBindingsData::default());
    let registry = build(state);
    let bar = frame(&registry, MAIN_ACTION_BAR.0);
    assert_eq!(bar.width, Dimension::Fixed(562.0));
    assert_eq!(bar.height, Dimension::Fixed(45.0));
    assert_eq!(bar.position.bottom, Val::Px(45.0));
    assert_eq!(bar.position.left, Val::Percent(50.0));
    assert_eq!(
        frame(&registry, "ActionButton1").position.left,
        Val::Px(0.0)
    );
    assert_eq!(
        frame(&registry, "ActionButton12").position.left,
        Val::Px(517.0)
    );
    assert_eq!(text(&registry, "ActionButton1HotKey"), "1");
    assert_eq!(text(&registry, "ActionButton10HotKey"), "0");
    assert_eq!(text(&registry, "ActionButton11HotKey"), "-");
    assert_eq!(text(&registry, "ActionButton12HotKey"), "=");
    assert!(matches!(
        source(&registry, "ActionButton1NormalTexture"),
        TextureSource::Atlas(name) if !name.is_empty()
    ));
    assert!(
        registry.get_by_name("ActionButton1Icon").is_none(),
        "empty slot"
    );
    assert!(frame(&registry, "ActionButton1PushedTexture").hidden);
}

#[test]
fn spell_button_shows_icon_cooldown_swipe_and_countdown() {
    let mut state = MainActionBarState::default();
    state.buttons[0] = ActionButtonView {
        icon_fdid: SLAM_ICON,
        cooldown_fraction: 0.5,
        cooldown_text: "3".into(),
        pushed: true,
        hovered: false,
        ..Default::default()
    };
    let registry = build(state);
    assert!(!frame(&registry, "ActionButton1Icon").hidden);
    assert_eq!(
        source(&registry, "ActionButton1Icon"),
        TextureSource::FileDataId(SLAM_ICON)
    );
    // The swipe covers half of the 39 px cooldown square, from the bottom.
    let swipe = frame(&registry, "ActionButton1Cooldown");
    assert!(!swipe.hidden);
    assert_eq!(swipe.height, Dimension::Fixed(20.0));
    assert_eq!(swipe.position.top, Val::Px(22.0));
    assert_eq!(text(&registry, "ActionButton1CooldownText"), "3");
    assert!(!frame(&registry, "ActionButton1PushedTexture").hidden);
    assert!(frame(&registry, "ActionButton1NormalTexture").hidden);
    assert!(frame(&registry, "ActionButton2Cooldown").hidden);
}

#[test]
fn empty_action_slots_have_no_texture_source() {
    use ui_toolkit::atlas::ActiveSkin;

    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = MainActionBarState::default();
        state.buttons[0].icon_fdid = SLAM_ICON;
        shared.insert(skin);
        shared.insert(state);
        let mut screen = Screen::new(main_action_bar_screen);
        screen.sync(&shared, &mut registry);
        assert_eq!(
            source(&registry, "ActionButton1Icon"),
            TextureSource::FileDataId(SLAM_ICON)
        );
        for index in 2..=12 {
            let name = format!("ActionButton{index}Icon");
            assert!(registry.get_by_name(&name).is_none(), "{name} is empty");
        }

        // A previously occupied slot must also clear its source when emptied.
        shared.insert(MainActionBarState::default());
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("ActionButton1Icon").is_none());
        assert!(registry.frames_iter().all(|frame| {
            !matches!(
                frame.widget_data.as_ref(),
                Some(WidgetData::Texture(texture))
                    if texture.source == TextureSource::FileDataId(0)
            )
        }));
    }
}

#[test]
fn clicks_name_their_button() {
    assert_eq!(
        parse_action_button("action_button:0"),
        Some((ActionBar::Main, 0))
    );
    assert_eq!(
        parse_action_button("action_button:11"),
        Some((ActionBar::Main, 11))
    );
    assert_eq!(parse_action_button("action_button:12"), None);
    assert_eq!(parse_action_button("spellbook_tab:1"), None);
}
