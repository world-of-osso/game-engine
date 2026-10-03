//! Retail PetFrame (Blizzard_UnitFrame/Mainline/PetFrame.xml) under the PlayerFrame.

use game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_art::FOCUS_BAR_TOT;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PET_PORTRAIT, PetFrameState, PowerBarState, UnitFrameMenuState,
    UnitFrameState, inworld_unit_frames_screen,
};
use shared::components::PowerType;
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn frames(pet: Option<PetFrameState>) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Fbpetbar"),
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

fn wolf() -> PetFrameState {
    PetFrameState {
        name: "Wolf".into(),
        health_fraction: 0.5,
        health_text: Default::default(),
        power: Some(PowerBarState {
            power: PowerType::Focus,
            current: 100,
            max: 100,
        }),
        power_text: Default::default(),
    }
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
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

/// PetFrame 120×49 is the first child of `PlayerBottomManagedFrameContainer`, whose TOP
/// sits at PlayerFrame's BOTTOM + (30, 25); `align` center with `leftPadding` 15 puts the
/// pet's TOP at the container TOP + (7.5, 0). PlayerFrame is 232×100 at BOTTOMRIGHT
/// (-300, 250), so PetFrame's left edge is 438.5 left of the screen centre and its bottom
/// 226 above the screen bottom.
#[test]
fn pet_frame_hangs_under_the_player_frame() {
    let registry = frames(Some(wolf()));
    let pet = frame(&registry, "PetFrame");
    assert!(!pet.hidden);
    assert_eq!(pet.width, Dimension::Fixed(120.0));
    assert_eq!(pet.height, Dimension::Fixed(49.0));
    assert_eq!(pet.position.left, Val::Percent(50.0));
    assert_eq!(pet.margin.left, Val::Px(-438.5));
    assert_eq!(pet.position.bottom, Val::Px(226.0));
}

/// `PetPortrait` 37×37 TOPLEFT (5, -5); `PetName` 68×10 at its TOPRIGHT + (2, 0);
/// `PetFrameHealthBar` 70×10 BOTTOMLEFT at its RIGHT + (2, -3.5); `PetFrameManaBar` 74×7
/// TOPLEFT at the health bar's BOTTOMLEFT + (-4, -1); the 120×49 TargetofTarget art.
#[test]
fn pet_frame_parts_use_retail_rects_and_focus_bar_art() {
    let registry = frames(Some(wolf()));
    assert_eq!(rect(&registry, "PetPortrait"), px(5.0, 5.0, 37.0, 37.0));
    assert_eq!(PET_PORTRAIT.rect, (5.0, 5.0, 37.0, 37.0));
    assert_eq!(PET_PORTRAIT.mask_fdid, 3_528_314);
    assert_eq!(rect(&registry, "PetName"), px(44.0, 5.0, 68.0, 10.0));
    match frame(&registry, "PetName").widget_data.as_ref() {
        Some(WidgetData::FontString(font)) => assert_eq!(font.text, "Wolf"),
        other => panic!("PetName is not a FontString: {other:?}"),
    }
    assert_eq!(
        rect(&registry, "PetFrameHealthBar"),
        px(44.0, 17.0, 70.0, 10.0)
    );
    assert_eq!(
        rect(&registry, "PetFrameManaBar"),
        px(40.0, 28.0, 74.0, 7.0)
    );
    assert_eq!(rect(&registry, "PetFrameArt"), px(0.0, 0.0, 120.0, 49.0));
    match frame(&registry, "PetFrameManaBarFill").widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => {
            assert_eq!(
                texture.source,
                TextureSource::FileDataId(FOCUS_BAR_TOT.fdid)
            )
        }
        other => panic!("PetFrameManaBarFill is not a Texture: {other:?}"),
    }
    assert_eq!(FOCUS_BAR_TOT.rect, (884.0, 958.0, 77.0, 84.0));
    assert_eq!(
        frame(&registry, "PetFrameHealthBarFill").width,
        Dimension::Fixed(35.0)
    );
    // Retail PetFrame has no level text.
    assert!(registry.get_by_name("PetLevelText").is_none());
}

#[test]
fn no_pet_frame_without_a_pet() {
    let registry = frames(None);
    assert!(frame(&registry, "PetFrame").hidden);
}
