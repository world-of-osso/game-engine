//! Retail Personal Resource Display (Blizzard_PersonalResourceDisplay) at the Modern Edit
//! Mode preset, enabled by the `nameplateShowSelf` option.

use game_engine_ui_model::inworld_unit_frames_component::class_bars::{ClassBarView, settled_view};
use game_engine_ui_model::inworld_unit_frames_component::personal_resource_display::PersonalResourceDisplayState;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
};
use game_engine_ui_model::status::{ClassBarPlayer, ClassBarResource};
use shared::components::{PowerEntry, PowerType, UnitPowers};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::font_string::FontStringData;
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

const ROGUE: u8 = 4;
const PALADIN: u8 = 2;
const MAGE: u8 = 8;
const PRIEST: u8 = 5;
const FROST: u32 = 64;
const SHADOW: u32 = 258;
/// UiTextureAtlas 3147 (FDID 6704514): `UI-HUD-CoolDownManager-Bar` and `-Bar-BG`.
const COOLDOWN_MANAGER_ATLAS: u32 = 6_704_514;

fn entry(power: PowerType, current: i32, max: i32) -> PowerEntry {
    PowerEntry {
        power,
        current,
        max,
        partial: 0,
        regen_per_sec: 0.0,
    }
}

fn player(class: u8, spec: Option<u32>) -> ClassBarPlayer {
    ClassBarPlayer {
        class,
        spec,
        level: 20,
        in_combat: true,
    }
}

/// What the client builds each frame: the PRD's own class frame, settled.
fn display(
    enabled: bool,
    player: &ClassBarPlayer,
    health_fraction: f32,
    powers: &UnitPowers,
) -> Option<PersonalResourceDisplayState> {
    hovering(enabled, player, health_fraction, powers, None)
}

/// The display with the pointer over the bar named `hovered`.
fn hovering(
    enabled: bool,
    player: &ClassBarPlayer,
    health_fraction: f32,
    powers: &UnitPowers,
    hovered: Option<&str>,
) -> Option<PersonalResourceDisplayState> {
    let class_bar: Option<ClassBarView> =
        ClassBarResource::for_player(powers, None, player).and_then(|r| settled_view(&r));
    let health = (health_fraction * 1000.0, 1000.0);
    PersonalResourceDisplayState::for_player(enabled, player, health, powers, class_bar, hovered)
}

fn frames(display: Option<PersonalResourceDisplayState>) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Fbprd"),
        target: None,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: display,
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

fn shown(registry: &FrameRegistry, name: &str) -> bool {
    registry
        .get_by_name(name)
        .and_then(|id| registry.get(id))
        .is_some_and(|frame| !frame.hidden)
}

fn texture<'a>(registry: &'a FrameRegistry, name: &str) -> &'a TextureData {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => texture,
        other => panic!("{name} is not a texture: {other:?}"),
    }
}

fn rect(registry: &FrameRegistry, name: &str) -> (Val, Val, Dimension, Dimension) {
    let frame = frame(registry, name);
    (
        frame.position.left,
        frame.position.top,
        frame.width,
        frame.height,
    )
}

fn px(x: f32, y: f32, width: f32, height: f32) -> (Val, Val, Dimension, Dimension) {
    (
        Val::Px(x),
        Val::Px(y),
        Dimension::Fixed(width),
        Dimension::Fixed(height),
    )
}

/// Class frame pips the display draws, by index.
fn class_pips(registry: &FrameRegistry) -> Vec<usize> {
    let prefix = "PersonalResourceDisplayPlayerSecondaryResourcePip";
    let mut pips: Vec<usize> = registry
        .frames_iter()
        .filter_map(|frame| frame.name.as_deref()?.strip_prefix(prefix))
        .filter_map(|rest| rest[..1].parse().ok())
        .collect();
    pips.sort_unstable();
    pips.dedup();
    pips
}

fn assert_color(registry: &FrameRegistry, name: &str, rgb: [f32; 3]) {
    let fill = texture(registry, name);
    assert_eq!(
        fill.source,
        TextureSource::FileDataId(COOLDOWN_MANAGER_ATLAS)
    );
    assert_eq!(fill.vertex_color, [rgb[0], rgb[1], rgb[2], 1.0], "{name}");
}

/// The Modern preset anchors the 200-wide frame's BOTTOM to UIParent BOTTOM at (-410, 380)
/// (EditModePresetLayouts.lua:773-779), so it grows upward from 380.
fn assert_anchored(registry: &FrameRegistry, height: f32) {
    let root = frame(registry, "PersonalResourceDisplayFrame");
    assert!(!root.hidden);
    assert_eq!(root.position.left, Val::Percent(50.0));
    assert_eq!(root.margin.left, Val::Px(-410.0 - 100.0));
    assert_eq!(root.position.bottom, Val::Px(380.0));
    assert_eq!(root.width, Dimension::Fixed(200.0));
    assert_eq!(root.height, Dimension::Fixed(height));
}

/// A rogue: green health, yellow Energy, and its own combo point bar (`RogueComboPointBar
/// Template`, yOffset -10) below the power bar.
#[test]
fn rogue_shows_health_energy_and_combo_points() {
    let rogue = player(ROGUE, None);
    let powers = UnitPowers {
        entries: vec![
            entry(PowerType::Energy, 80, 100),
            entry(PowerType::ComboPoints, 3, 5),
        ],
        ..Default::default()
    };
    let registry = frames(display(true, &rogue, 0.75, &powers));
    // 15 health + 4 + 15 power + 4 padding + 10 yOffset + 15 container.
    assert_anchored(&registry, 63.0);
    assert_eq!(
        rect(&registry, "PersonalResourceDisplayHealthBar"),
        px(0.0, 0.0, 200.0, 15.0)
    );
    assert_eq!(
        frame(&registry, "PersonalResourceDisplayHealthBarFill").width,
        Dimension::Fixed(150.0)
    );
    // PERSONAL_RESOURCE_DISPLAY_DEFAULT_HEALTH_COLOR: GlobalColor 391, 0xFF00CC00.
    assert_color(
        &registry,
        "PersonalResourceDisplayHealthBarFill",
        [0.0, 0.8, 0.0],
    );
    assert_eq!(
        rect(&registry, "PersonalResourceDisplayPowerBar"),
        px(0.0, 19.0, 200.0, 15.0)
    );
    assert_eq!(
        frame(&registry, "PersonalResourceDisplayPowerBarFill").width,
        Dimension::Fixed(160.0)
    );
    assert_color(
        &registry,
        "PersonalResourceDisplayPowerBarFill",
        [1.0, 1.0, 0.0],
    );
    assert!(!shown(
        &registry,
        "PersonalResourceDisplayAlternatePowerBar"
    ));
    assert_eq!(
        rect(&registry, "PersonalResourceDisplayClassFrameContainer"),
        px(0.0, 48.0, 200.0, 15.0)
    );
    assert_eq!(class_pips(&registry), vec![0, 1, 2, 3, 4]);
}

/// A paladin: mana in the display's own blue (`MANA_BAR_COLOR`) and the Holy Power bar
/// (`PaladinPowerBarFrameTemplate`, yOffset -14).
#[test]
fn paladin_shows_mana_and_holy_power() {
    let paladin = player(PALADIN, None);
    let powers = UnitPowers {
        entries: vec![
            entry(PowerType::Mana, 500, 1000),
            entry(PowerType::HolyPower, 3, 5),
        ],
        ..Default::default()
    };
    let registry = frames(display(true, &paladin, 1.0, &powers));
    assert_anchored(&registry, 67.0);
    assert_color(
        &registry,
        "PersonalResourceDisplayPowerBarFill",
        [0.1, 0.25, 1.0],
    );
    assert!(!shown(
        &registry,
        "PersonalResourceDisplayAlternatePowerBar"
    ));
    assert_eq!(
        rect(&registry, "PersonalResourceDisplayClassFrameContainer"),
        px(0.0, 52.0, 200.0, 15.0)
    );
    assert!(shown(
        &registry,
        "PersonalResourceDisplayPlayerSecondaryResourceHolderBackground"
    ));
    assert_eq!(class_pips(&registry), vec![0, 1, 2, 3, 4]);
}

/// A Frost mage: health and mana only. Arcane Charges belong to Arcane, but the mage's
/// class container still counts toward the frame height (`HasClassInfo` has no gate).
#[test]
fn frost_mage_shows_mana_only() {
    let mage = player(MAGE, Some(FROST));
    let powers = UnitPowers {
        entries: vec![entry(PowerType::Mana, 1000, 1000)],
        ..Default::default()
    };
    let registry = frames(display(true, &mage, 1.0, &powers));
    assert_anchored(&registry, 61.0);
    assert!(shown(&registry, "PersonalResourceDisplayHealthBar"));
    assert!(shown(&registry, "PersonalResourceDisplayPowerBar"));
    assert!(!shown(
        &registry,
        "PersonalResourceDisplayAlternatePowerBar"
    ));
    assert!(class_pips(&registry).is_empty());
}

/// Shadow: Insanity on the power bar, mana on the alternate bar in `PowerBarColor.MANA`
/// (ManaAlternatePower.lua:6-16, PriestAlternatePowerBarMixin).
#[test]
fn shadow_priest_shows_mana_on_the_alternate_bar() {
    let priest = player(PRIEST, Some(SHADOW));
    let powers = UnitPowers {
        entries: vec![
            entry(PowerType::Insanity, 4000, 10000),
            entry(PowerType::Mana, 750, 1000),
        ],
        ..Default::default()
    };
    let registry = frames(display(true, &priest, 1.0, &powers));
    assert_anchored(&registry, 53.0);
    assert_color(
        &registry,
        "PersonalResourceDisplayPowerBarFill",
        [0.4, 0.0, 0.8],
    );
    assert_eq!(
        frame(&registry, "PersonalResourceDisplayPowerBarFill").width,
        Dimension::Fixed(80.0)
    );
    assert_eq!(
        rect(&registry, "PersonalResourceDisplayAlternatePowerBar"),
        px(0.0, 38.0, 200.0, 15.0)
    );
    assert_color(
        &registry,
        "PersonalResourceDisplayAlternatePowerBarFill",
        [0.0, 0.0, 1.0],
    );
    assert_eq!(
        frame(&registry, "PersonalResourceDisplayAlternatePowerBarFill").width,
        Dimension::Fixed(150.0)
    );
}

/// `nameplateShowSelf` 0 (the Retail default) hides the display.
#[test]
fn disabled_display_draws_nothing() {
    let rogue = player(ROGUE, None);
    let powers = UnitPowers {
        entries: vec![entry(PowerType::Energy, 80, 100)],
        ..Default::default()
    };
    assert!(display(false, &rogue, 1.0, &powers).is_none());
    let registry = frames(None);
    assert!(
        registry
            .get_by_name("PersonalResourceDisplayFrame")
            .is_none()
    );
}

fn text<'a>(registry: &'a FrameRegistry, name: &str) -> Option<&'a str> {
    let frame = frame(registry, name);
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(FontStringData { text, .. })) if !frame.hidden => {
            Some(text.as_str())
        }
        Some(WidgetData::FontString(_)) => None,
        other => panic!("{name} is not a font string: {other:?}"),
    }
}

fn bar_texts<'a>(registry: &'a FrameRegistry, bar: &str) -> [Option<&'a str>; 3] {
    ["Text", "TextLeft", "TextRight"]
        .map(|suffix| text(registry, &format!("PersonalResourceDisplay{bar}{suffix}")))
}

/// `PersonalResourceStatusBar` has no `cvar` and the preset's Show Bar Text is off, so
/// its text shows only while hovered (`lockShow`, TextStatusBar.lua:113-121,217-220);
/// `showNumeric` and `showPercentage` force Both: percentage left, value right. The
/// PowerBar has no `powerToken`, so Energy keeps its percentage too.
#[test]
fn bar_text_shows_both_only_while_hovered() {
    let rogue = player(ROGUE, None);
    let powers = UnitPowers {
        entries: vec![
            entry(PowerType::Energy, 80, 100),
            entry(PowerType::ComboPoints, 0, 5),
        ],
        ..Default::default()
    };
    let idle = frames(display(true, &rogue, 0.75, &powers));
    assert_eq!(bar_texts(&idle, "HealthBar"), [None, None, None]);
    assert_eq!(bar_texts(&idle, "PowerBar"), [None, None, None]);
    let health = frames(hovering(
        true,
        &rogue,
        0.75,
        &powers,
        Some("PersonalResourceDisplayHealthBar"),
    ));
    assert_eq!(
        bar_texts(&health, "HealthBar"),
        [None, Some("75%"), Some("750")]
    );
    assert_eq!(bar_texts(&health, "PowerBar"), [None, None, None]);
    let power = frames(hovering(
        true,
        &rogue,
        0.75,
        &powers,
        Some("PersonalResourceDisplayPowerBar"),
    ));
    assert_eq!(
        bar_texts(&power, "PowerBar"),
        [None, Some("80%"), Some("80")]
    );
}

/// The alternate bar: `showPercentage` false and `disableMaxValue` leave Numeric with the
/// value alone in its `TextString`.
#[test]
fn alternate_bar_text_is_the_value_alone() {
    let priest = player(PRIEST, Some(SHADOW));
    let powers = UnitPowers {
        entries: vec![
            entry(PowerType::Insanity, 4000, 10000),
            entry(PowerType::Mana, 12500, 50000),
        ],
        ..Default::default()
    };
    let registry = frames(hovering(
        true,
        &priest,
        1.0,
        &powers,
        Some("PersonalResourceDisplayAlternatePowerBar"),
    ));
    assert_eq!(
        bar_texts(&registry, "AlternatePowerBar"),
        [Some("12,500"), None, None]
    );
}
