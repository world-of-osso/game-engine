//! Retail ReputationBar.lua and the two StatusTrackingBarManager containers.
use game_engine_ui_model::reputation_status_bar::{ReputationBarState, reputation_bar_screen};
use shared::protocol_snapshots::WatchedReputationSnapshot;
use shared::reputation::Standing;
use ui_toolkit::{
    atlas::ActiveSkin,
    frame::{Dimension, Frame, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::texture::TextureSource,
};

fn watched() -> WatchedReputationSnapshot {
    WatchedReputationSnapshot {
        faction_id: 2503,
        name: "Maruuk Centaur".into(),
        value: 1000,
        maximum: 2500,
        capped: false,
        is_major_faction: true,
        reaction: Standing::Neutral,
    }
}
fn render(
    skin: ActiveSkin,
    watched: Option<WatchedReputationSnapshot>,
    xp_visible: bool,
    hovered: bool,
) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    ui_toolkit::atlas::set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(ReputationBarState {
        watched,
        xp_visible,
        hovered,
    });
    let mut registry = FrameRegistry::new(1920., 1080.);
    Screen::new(reputation_bar_screen).sync(&shared, &mut registry);
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}
fn width(registry: &FrameRegistry, name: &str) -> f32 {
    let Dimension::Fixed(width) = frame(registry, name).width else {
        panic!("fixed width")
    };
    width
}
fn text(registry: &FrameRegistry) -> String {
    let Some(WidgetData::FontString(font)) =
        &frame(registry, "ReputationStatusBarText").widget_data
    else {
        panic!("text")
    };
    font.text.clone()
}
#[test]
fn reputation_bar_is_secondary_to_xp_and_promotes_after_xp_cap_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let secondary = render(skin, Some(watched()), true, false);
        assert_eq!(width(&secondary, "ReputationStatusBar"), 571.);
        assert_eq!(
            frame(&secondary, "ReputationStatusBar").position.bottom,
            Val::Px(17.)
        );
        assert_eq!(width(&secondary, "ReputationStatusBarBackground"), 565.);
        assert_eq!(width(&secondary, "ReputationStatusBarFill"), 226.);
        let main = render(skin, Some(watched()), false, false);
        assert_eq!(
            frame(&main, "ReputationStatusBar").position.bottom,
            Val::Px(0.)
        );
        assert_eq!(width(&main, "ReputationStatusBarFill"), 226.);
    }
}
#[test]
fn reputation_bar_hover_shows_authoritative_name_and_normalized_progress_then_hides() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = render(skin, Some(watched()), false, true);
        assert_eq!(text(&r), "Maruuk Centaur 1000 / 2500");
        assert!(!frame(&r, "ReputationStatusBarText").hidden);
        let r = render(skin, Some(watched()), false, false);
        assert!(frame(&r, "ReputationStatusBarText").hidden);
        let mut capped = watched();
        capped.capped = true;
        capped.value = 0;
        let r = render(skin, Some(capped), false, true);
        assert_eq!(text(&r), "Maruuk Centaur");
        assert_eq!(width(&r, "ReputationStatusBarFill"), 0.);
    }
}
#[test]
fn reputation_bar_missing_cleared_or_empty_watch_has_no_visible_bar() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let r = render(skin, None, false, false);
        assert!(r.get_by_name("ReputationStatusBar").is_none());
        let mut empty = watched();
        empty.name.clear();
        assert!(
            render(skin, Some(empty), false, false)
                .get_by_name("ReputationStatusBar")
                .is_none()
        );
    }
}
#[test]
fn reputation_bar_uses_blue_renown_and_reaction_colored_retail_atlases() {
    let r = render(ActiveSkin::Modern, Some(watched()), false, false);
    let Some(WidgetData::Texture(fill)) = &frame(&r, "ReputationStatusBarFill").widget_data else {
        panic!("texture")
    };
    assert!(
        matches!(&fill.source,TextureSource::Atlas(name) if name=="UI-HUD-ExperienceBar-Fill-Reputation-Faction-Blue")
    );
    let mut standard = watched();
    standard.is_major_faction = false;
    standard.reaction = Standing::Hostile;
    standard.name = "Orgrimmar".into();
    let r = render(ActiveSkin::Modern, Some(standard), false, false);
    let Some(WidgetData::Texture(fill)) = &frame(&r, "ReputationStatusBarFill").widget_data else {
        panic!("texture")
    };
    assert!(
        matches!(&fill.source,TextureSource::Atlas(name) if name=="UI-HUD-ExperienceBar-Fill-Reputation-Faction-Red")
    );
}
