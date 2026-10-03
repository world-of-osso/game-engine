//! TargetFrame classification art (`TargetFrameMixin:CheckClassification`,
//! Blizzard_UnitFrame/Mainline/TargetFrame.lua:436-462) around the portrait.

use game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_art::{
    AtlasArt, BOSS_GOLD, BOSS_RARE_SILVER, BOSS_RARE_STAR,
};
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
};
use shared::components::CreatureClassification;
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn target_frames(name: &str, classification: CreatureClassification) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Fbunitrank"),
        target: Some(UnitFrameState {
            level_text: "10".into(),
            classification,
            ..UnitFrameState::named(name)
        }),
        target_of_target: None,
        focus: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
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

fn texture(registry: &FrameRegistry, name: &str) -> (TextureSource, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => (texture.source.clone(), texture.tex_coords),
        other => panic!("{name} is not a Texture: {other:?}"),
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

fn atlas(art: &AtlasArt) -> (TextureSource, [f32; 4]) {
    let (left, right, top, bottom) = art.rect;
    let (width, height) = art.atlas;
    (
        TextureSource::FileDataId(art.fdid),
        [left / width, right / width, top / height, bottom / height],
    )
}

/// Timber (world.db creature_template 1132, rank 4): the rare star centred on the
/// portrait's BOTTOM, (177, 77) of the 232×100 frame, and no dragon.
#[test]
fn timber_shows_the_rare_star_without_a_dragon() {
    let registry = target_frames("Timber", CreatureClassification::Rare);
    assert!(frame(&registry, "TargetBossPortraitFrameTexture").hidden);
    assert!(!frame(&registry, "TargetBossIcon").hidden);
    assert_eq!(texture(&registry, "TargetBossIcon"), atlas(&BOSS_RARE_STAR));
    assert_eq!(
        rect(&registry, "TargetBossIcon"),
        (
            Val::Px(167.0),
            Val::Px(67.0),
            Dimension::Fixed(20.0),
            Dimension::Fixed(20.0)
        )
    );
}

/// Hogger (448, rank 1): the gold dragon TOPRIGHT (-11, -8) of the 232×100 frame, so
/// 141..221 × 8..87; no star.
#[test]
fn hogger_shows_the_gold_dragon_at_its_retail_anchor() {
    let registry = target_frames("Hogger", CreatureClassification::Elite);
    assert!(!frame(&registry, "TargetBossPortraitFrameTexture").hidden);
    assert!(frame(&registry, "TargetBossIcon").hidden);
    assert_eq!(
        texture(&registry, "TargetBossPortraitFrameTexture"),
        atlas(&BOSS_GOLD)
    );
    assert_eq!(
        rect(&registry, "TargetBossPortraitFrameTexture"),
        (
            Val::Px(141.0),
            Val::Px(8.0),
            Dimension::Fixed(80.0),
            Dimension::Fixed(79.0)
        )
    );
}

#[test]
fn rare_elites_get_the_silver_dragon_and_star_normal_units_neither() {
    // Bruegal Ironknuckle (1720, rank 2).
    let registry = target_frames("Bruegal Ironknuckle", CreatureClassification::RareElite);
    assert!(!frame(&registry, "TargetBossIcon").hidden);
    assert_eq!(
        texture(&registry, "TargetBossPortraitFrameTexture"),
        atlas(&BOSS_RARE_SILVER)
    );
    let normal = target_frames("Timber Wolf", CreatureClassification::Normal);
    assert!(frame(&normal, "TargetBossPortraitFrameTexture").hidden);
    assert!(frame(&normal, "TargetBossIcon").hidden);
}
