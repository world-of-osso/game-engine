//! docs/specs/xp-bar.md: geometry, fill, rested overlay, level cap, hover text and each
//! preset's art.

use game_engine_ui_model::xp_bar_component::{XpBarState, xp_bar_screen};
use std::path::PathBuf;
use ui_toolkit::atlas::{ActiveSkin, resolve_region};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

/// 250 of 1000 XP with a rested pool of 400: fill 25%, pool ending at 65%.
fn state() -> XpBarState {
    XpBarState {
        xp: 250,
        next_level_xp: 1000,
        rested_xp: 400,
        hovered: false,
    }
}

fn setup(skin: ActiveSkin, state: XpBarState) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(xp_bar_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn size(frame: &Frame) -> (f32, f32) {
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (frame.width, frame.height) else {
        panic!("fixed size")
    };
    (w, h)
}

fn origin(frame: &Frame) -> (Val, Val) {
    (frame.position.left, frame.position.top)
}

fn texture<'a>(
    registry: &'a FrameRegistry,
    name: &str,
) -> &'a ui_toolkit::widgets::texture::TextureData {
    let Some(WidgetData::Texture(data)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} is not a texture")
    };
    data
}

/// `name` draws Blizzard's atlas `atlas`, which the Modern skin resolves.
fn assert_atlas(registry: &FrameRegistry, name: &str, atlas: &str) {
    assert_eq!(
        texture(registry, name).source,
        TextureSource::Atlas(atlas.into())
    );
    assert!(
        resolve_region(atlas, ActiveSkin::Modern).is_some(),
        "{atlas}"
    );
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let Some(WidgetData::FontString(text)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} is not a font string")
    };
    text.text.clone()
}

#[test]
fn modern_is_the_retail_container_at_the_bottom_centre_in_blizzard_atlases() {
    let r = setup(ActiveSkin::Modern, state());
    let root = frame(&r, "ExperienceBar");
    assert_eq!(size(root), (571.0, 17.0));
    assert_eq!(root.position.left, Val::Percent(50.0));
    assert_eq!(root.margin.left, Val::Px(-285.5));
    assert_eq!(root.position.bottom, Val::Px(0.0));
    assert!(!root.hidden);
    let px = (Val::Px(1.0), Val::Px(1.0));
    assert_eq!(size(frame(&r, "ExperienceBarBackground")), (565.0, 11.0));
    assert_eq!(origin(frame(&r, "ExperienceBarBackground")), px);
    assert_eq!(size(frame(&r, "ExperienceBarFill")), (141.25, 11.0));
    assert_eq!(origin(frame(&r, "ExperienceBarFill")), px);
    assert_eq!(size(frame(&r, "ExperienceBarFrame")), (571.0, 17.0));
    assert_atlas(
        &r,
        "ExperienceBarBackground",
        "UI-HUD-ExperienceBar-Background",
    );
    assert_atlas(&r, "ExperienceBarFill", "UI-HUD-ExperienceBar-Fill-Rested");
    assert_atlas(&r, "ExperienceBarFrame", "UI-HUD-ExperienceBar-Frame");
    assert_atlas(
        &r,
        "ExperienceBarPrediction",
        "UI-HUD-ExperienceBar-Fill-Prediction",
    );
    assert_atlas(&r, "ExperienceBarTick", "UI-HUD-ExperienceBar-Frame-Pip");
}

#[test]
fn rested_overlay_ends_at_the_pool_and_hides_past_the_level() {
    let r = setup(ActiveSkin::Modern, state());
    // (250 + 400) / 1000 of 565.
    assert_eq!(size(frame(&r, "ExperienceBarPrediction")), (367.25, 11.0));
    assert!(!frame(&r, "ExperienceBarPrediction").hidden);
    let tick = frame(&r, "ExperienceBarTick");
    assert!(!tick.hidden);
    assert_eq!(size(tick), (10.0, 14.0));
    // Centred on the pool's end, 2 above the bar's centre.
    assert_eq!(origin(tick), (Val::Px(363.25), Val::Px(-2.5)));

    let overflowing = XpBarState {
        rested_xp: 800,
        ..state()
    };
    let r = setup(ActiveSkin::Modern, overflowing);
    assert!(frame(&r, "ExperienceBarPrediction").hidden);
    assert!(frame(&r, "ExperienceBarTick").hidden);

    // The pool ends at 99.5%: the overlay stays, the pip hides at the bar's edge.
    let at_edge = XpBarState {
        rested_xp: 745,
        ..state()
    };
    let r = setup(ActiveSkin::Modern, at_edge);
    assert!(!frame(&r, "ExperienceBarPrediction").hidden);
    assert!(frame(&r, "ExperienceBarTick").hidden);
}

#[test]
fn unrested_bar_is_purple_without_overlay_and_hover_shows_the_xp_text() {
    let unrested = XpBarState {
        rested_xp: 0,
        ..state()
    };
    let r = setup(ActiveSkin::Modern, unrested);
    assert_atlas(
        &r,
        "ExperienceBarFill",
        "UI-HUD-ExperienceBar-Fill-Experience",
    );
    assert!(frame(&r, "ExperienceBarPrediction").hidden);
    assert!(frame(&r, "ExperienceBarTick").hidden);
    assert!(frame(&r, "ExperienceBarText").hidden);

    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let hovered = XpBarState {
            hovered: true,
            ..state()
        };
        let r = setup(skin, hovered);
        assert!(!frame(&r, "ExperienceBarText").hidden);
        assert_eq!(text(&r, "ExperienceBarText"), "XP: 250/1000");
    }
}

#[test]
fn level_cap_hides_the_bar_in_both_presets() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let capped = XpBarState {
            xp: 0,
            next_level_xp: 0,
            rested_xp: 400,
            hovered: false,
        };
        let r = setup(skin, capped);
        assert!(frame(&r, "ExperienceBar").hidden, "{skin:?}");
        assert_eq!(size(frame(&r, "ExperienceBarFill")).0, 0.0);
        assert!(frame(&r, "ExperienceBarPrediction").hidden);
    }
}

#[test]
fn forever_is_a_flat_bar_at_the_top_centre_under_a_bronze_border() {
    let r = setup(ActiveSkin::Forever, state());
    let root = frame(&r, "ExperienceBar");
    assert_eq!(size(root), (596.0, 17.0));
    assert_eq!(root.position.left, Val::Percent(50.0));
    assert_eq!(root.margin.left, Val::Px(-298.0));
    assert_eq!(root.position.top, Val::Px(6.0));
    let px = (Val::Px(1.0), Val::Px(1.0));
    let track = frame(&r, "ExperienceBarBackground");
    assert_eq!((size(track), origin(track)), ((593.0, 14.0), px));
    assert_eq!(track.background_color, Some([0.15, 0.15, 0.15, 0.9]));
    let fill = frame(&r, "ExperienceBarFill");
    assert_eq!((size(fill), origin(fill)), ((148.25, 14.0), px));
    assert_eq!(fill.background_color, Some([0.0, 0.39, 0.88, 1.0]));
    let prediction = frame(&r, "ExperienceBarPrediction");
    assert_eq!(size(prediction), (593.0 * (650.0 / 1000.0), 14.0));
    assert_eq!(prediction.background_color, Some([0.0, 0.39, 0.88, 0.4]));

    // The border sits 4 outside the fill, built after it, in XPBar.lua's bronze.
    let border = frame(&r, "ExperienceBarBorder");
    assert_eq!(size(border), (601.0, 22.0));
    assert_eq!(origin(border), (Val::Px(-3.0), Val::Px(-3.0)));
    let order = |name: &str| {
        let id = r.get_by_name(name).unwrap();
        root.children.iter().position(|child| *child == id).unwrap()
    };
    assert!(order("ExperienceBarBorder") > order("ExperienceBarFill"));
    assert!(order("ExperienceBarOverlay") > order("ExperienceBarBorder"));
    let corner = texture(&r, "ExperienceBarBorderTopLeft");
    assert_eq!(corner.vertex_color, [0.8, 0.6, 0.34, 1.0]);
    assert_eq!(corner.source, TextureSource::FileDataId(137_057));
    assert_eq!(size(frame(&r, "ExperienceBarBorderTopLeft")), (16.0, 16.0));
    // No Blizzard frame art or pip.
    assert!(r.get_by_name("ExperienceBarFrame").is_none());
    assert!(r.get_by_name("ExperienceBarTick").is_none());

    let unrested = XpBarState {
        rested_xp: 0,
        ..state()
    };
    let r = setup(ActiveSkin::Forever, unrested);
    assert_eq!(
        frame(&r, "ExperienceBarFill").background_color,
        Some([0.58, 0.0, 0.55, 1.0])
    );
    assert!(frame(&r, "ExperienceBarPrediction").hidden);
}
