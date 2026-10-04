use game_engine_ui_model::xp_bar_component::{XpBarState, apply_xp_bar_postsetup, xp_bar_screen};
use std::path::PathBuf;
use ui_toolkit::atlas::{ActiveSkin, resolve_region};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn state() -> XpBarState {
    XpBarState {
        xp: 250,
        next_level_xp: 1000,
        rested_xp: 400,
        level: 12,
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
    registry.register_panel_style(
        game_engine_ui_model::flare_panel::FLARE_BRONZE_PANEL_STYLE,
        game_engine_ui_model::flare_panel::flare_bronze_style(TextureSource::SolidColor([1.0; 4])),
    );
    Screen::new(xp_bar_screen).sync(&shared, &mut registry);
    apply_xp_bar_postsetup(&mut registry);
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
fn texture<'a>(
    registry: &'a FrameRegistry,
    name: &str,
) -> &'a ui_toolkit::widgets::texture::TextureData {
    let Some(WidgetData::Texture(data)) = frame(registry, name).widget_data.as_ref() else {
        panic!("texture")
    };
    data
}
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
#[test]
fn modern_bottom_rect_and_quarter_fill_use_blizzard_atlases() {
    let r = setup(ActiveSkin::Modern, state());
    let root = frame(&r, "ExperienceBar");
    assert_eq!(size(root), (571.0, 17.0));
    assert_eq!(root.position.left, Val::Percent(50.0));
    assert_eq!(root.margin.left, Val::Px(-285.5));
    assert_eq!(root.position.bottom, Val::Px(0.0)); // rect (674.5,1063,571,17)
    assert_eq!(size(frame(&r, "ExperienceBarFill")), (141.25, 11.0));
    assert_eq!(frame(&r, "ExperienceBarFill").position.left, Val::Px(1.0));
    assert_eq!(frame(&r, "ExperienceBarFill").position.top, Val::Px(1.0));
    assert_atlas(&r, "ExperienceBarFill", "UI-HUD-ExperienceBar-Fill-Rested");
    assert_atlas(
        &r,
        "ExperienceBarBackground",
        "UI-HUD-ExperienceBar-Background",
    );
    assert_atlas(&r, "ExperienceBarFrame", "UI-HUD-ExperienceBar-Frame");
    assert_atlas(
        &r,
        "ExperienceBarPrediction",
        "UI-HUD-ExperienceBar-Fill-Prediction",
    );
    assert_atlas(&r, "ExperienceBarTick", "UI-HUD-ExperienceBar-Frame-Pip");
}
#[test]
fn rested_prediction_reaches_sixty_five_percent_and_hides_beyond_level() {
    let r = setup(ActiveSkin::Modern, state());
    assert_eq!(size(frame(&r, "ExperienceBarPrediction")), (367.25, 11.0));
    assert!(!frame(&r, "ExperienceBarPrediction").hidden);
    let tick = frame(&r, "ExperienceBarTick");
    assert!(!tick.hidden);
    assert_eq!(tick.position.left, Val::Px(363.25));
    let r = setup(
        ActiveSkin::Modern,
        XpBarState {
            rested_xp: 800,
            ..state()
        },
    );
    assert!(frame(&r, "ExperienceBarPrediction").hidden);
    assert!(frame(&r, "ExperienceBarTick").hidden);
}
#[test]
fn unrested_fill_and_hover_text_follow_retail() {
    let r = setup(
        ActiveSkin::Modern,
        XpBarState {
            rested_xp: 0,
            ..state()
        },
    );
    assert_atlas(
        &r,
        "ExperienceBarFill",
        "UI-HUD-ExperienceBar-Fill-Experience",
    );
    assert!(frame(&r, "ExperienceBarPrediction").hidden);
    assert!(frame(&r, "ExperienceBarText").hidden);
    let r = setup(
        ActiveSkin::Modern,
        XpBarState {
            hovered: true,
            ..state()
        },
    );
    let text = frame(&r, "ExperienceBarText");
    assert!(!text.hidden);
    let Some(WidgetData::FontString(text)) = text.widget_data.as_ref() else {
        panic!("font")
    };
    assert_eq!(text.text, "XP: 250/1000");
}
#[test]
fn level_cap_hides_entire_bar_in_both_presets() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = setup(
            skin,
            XpBarState {
                next_level_xp: 0,
                level: 90,
                ..state()
            },
        );
        assert!(frame(&r, "ExperienceBar").hidden);
        assert_eq!(size(frame(&r, "ExperienceBarFill")).0, 0.0);
    }
}
#[test]
fn forever_top_rect_flat_palette_and_bronze_border() {
    let r = setup(ActiveSkin::Forever, state());
    let root = frame(&r, "ExperienceBar");
    assert_eq!(size(root), (1192.0, 17.0));
    assert_eq!(root.position.left, Val::Percent(50.0));
    assert_eq!(root.margin.left, Val::Px(-596.0));
    assert_eq!(root.position.top, Val::Px(0.0)); // rect (364,0,1192,17)
    assert_eq!(size(frame(&r, "ExperienceBarFill")), (297.25, 14.0));
    assert_eq!(
        texture(&r, "ExperienceBarFill").source,
        TextureSource::SolidColor([1.0; 4])
    );
    assert_eq!(
        texture(&r, "ExperienceBarFill").vertex_color,
        [0.0, 0.39, 0.88, 1.0]
    );
    assert_eq!(
        texture(&r, "ExperienceBarPrediction").vertex_color,
        [0.0, 0.39, 0.88, 0.4]
    );
    assert_eq!(
        texture(&r, "ExperienceBarBackground").vertex_color,
        [0.15, 0.15, 0.15, 0.9]
    );
    let border = frame(&r, "ExperienceBarBorder")
        .nine_slice
        .as_ref()
        .unwrap();
    assert_eq!(border.border_color, [0.8, 0.6, 0.34, 1.0]);
    assert_eq!(border.edge_size, 16.0);
    assert_eq!(border.bg_color, [0.0; 4]);
    assert!(frame(&r, "ExperienceBarFrame").hidden);
    let r = setup(
        ActiveSkin::Forever,
        XpBarState {
            rested_xp: 0,
            ..state()
        },
    );
    assert_eq!(
        texture(&r, "ExperienceBarFill").vertex_color,
        [0.58, 0.0, 0.55, 1.0]
    );
}
