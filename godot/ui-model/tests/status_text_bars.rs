//! Unit frame bars draw the Status Text (`TextStatusBar`) in their `TextString`,
//! `LeftText` and `RightText`, and take the mouse for the hover-only None mode.

use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PetFrameState, PowerBarState, UnitFrameMenuState, UnitFrameState,
    inworld_unit_frames_screen,
};
use game_engine_ui_model::status_text_data::{StatusTextDisplay, TextStatusBar};
use shared::components::PowerType;
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn frames(player: UnitFrameState, pet: Option<PetFrameState>) -> FrameRegistry {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player,
        target: None,
        target_of_target: None,
        focus: None,
        pet,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

/// The shown text of a font string, `None` while hidden.
fn shown_text(registry: &FrameRegistry, name: &str) -> Option<String> {
    let frame = frame(registry, name);
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(font)) => (!frame.hidden).then(|| font.text.clone()),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

fn texts(registry: &FrameRegistry, bar: &str) -> [Option<String>; 3] {
    ["Text", "TextLeft", "TextRight"].map(|suffix| shown_text(registry, &format!("{bar}{suffix}")))
}

fn player(display: StatusTextDisplay) -> UnitFrameState {
    UnitFrameState {
        health_text: TextStatusBar::HEALTH.text(12_345, 15_000, display, false),
        power: Some(PowerBarState {
            power: PowerType::Rage,
            current: 45,
            max: 100,
        }),
        power_text: TextStatusBar::power(false).text(45, 100, display, false),
        ..UnitFrameState::named("Fbstatustext")
    }
}

#[test]
fn both_puts_percent_left_and_value_right_on_the_player_bars() {
    let registry = frames(player(StatusTextDisplay::Both), None);
    assert_eq!(
        texts(&registry, "PlayerHealthBar"),
        [None, Some("83%".into()), Some("12,345".into())]
    );
    // Rage: no percentage in Both.
    assert_eq!(
        texts(&registry, "PlayerManaBar"),
        [None, None, Some("45".into())]
    );
    // PlayerFrame LeftText LEFT (2, 0), RightText RIGHT (-2, 0) on the 124-wide bar.
    let left = frame(&registry, "PlayerHealthBarTextLeft");
    let right = frame(&registry, "PlayerHealthBarTextRight");
    assert_eq!(left.position.left, Val::Px(2.0));
    assert_eq!(right.position.left, Val::Px(-2.0));
}

#[test]
fn percentage_centres_the_rounded_up_percent() {
    let registry = frames(player(StatusTextDisplay::Percent), None);
    assert_eq!(
        texts(&registry, "PlayerHealthBar"),
        [Some("83%".into()), None, None]
    );
    assert_eq!(
        texts(&registry, "PlayerManaBar"),
        [Some("45%".into()), None, None]
    );
}

#[test]
fn none_draws_no_text_and_the_bars_take_the_mouse_for_hover() {
    let registry = frames(player(StatusTextDisplay::None), None);
    assert_eq!(texts(&registry, "PlayerHealthBar"), [None, None, None]);
    assert!(frame(&registry, "PlayerHealthBar").mouse_enabled);
    assert!(frame(&registry, "PlayerManaBar").mouse_enabled);
}

#[test]
fn pet_bars_show_their_status_text() {
    let pet = PetFrameState {
        name: "Wolf".into(),
        health_fraction: 0.5,
        reaction: None,
        health_text: TextStatusBar::HEALTH.text(
            1_234_567,
            2_469_134,
            StatusTextDisplay::Numeric,
            false,
        ),
        power: Some(PowerBarState {
            power: PowerType::Focus,
            current: 0,
            max: 100,
        }),
        power_text: TextStatusBar::power(false).text(0, 100, StatusTextDisplay::Numeric, false),
    };
    let registry = frames(player(StatusTextDisplay::Numeric), Some(pet));
    assert_eq!(
        texts(&registry, "PetFrameHealthBar"),
        [Some("1234 K / 2469 K".into()), None, None]
    );
    assert_eq!(
        texts(&registry, "PetFrameManaBar"),
        [Some("0 / 100".into()), None, None]
    );
}

/// Unit frames size their art from the atlas tables (`atlas_size`).
fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}
