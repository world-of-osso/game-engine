use game_engine_ui_model::bag_data::{InventorySlot, ItemQuality};
use game_engine_ui_model::item_tooltip::item_tooltip;
use game_engine_ui_model::tooltip_presentation::{
    GRAY_FONT_COLOR, ItemMark, TOOLTIP_W, TooltipLineState, TooltipPresentation, append_item_id,
    tooltip_frame_screen,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn linen_hover_projects_original_title_coins_and_id() {
    data_root();
    let slot = InventorySlot {
        item_id: 2589,
        name: "Linen Cloth".into(),
        count: 20,
        quality: ItemQuality::Common,
        ..Default::default()
    };
    let mut state = item_tooltip(&slot, Some(1));
    assert!(state.visible);
    assert_eq!(state.title, "Linen Cloth");
    assert_eq!(state.title_color, [1.0; 4]);
    assert_eq!(state.lines.len(), 1);
    assert_eq!(state.lines[0].money, Some(260));
    append_item_id(&mut state, slot.item_id);
    assert_eq!(state.lines[1].left_text, "Item ID: 2589");
    assert_eq!(state.lines[1].left_color, GRAY_FONT_COLOR);
    state.x = 300.0;
    state.y = 200.0;
    assert_eq!(state.height(), 60.0);
    let registry = render(state);
    assert!(registry.get_by_name("TooltipFrame").is_some());
    assert_eq!(TOOLTIP_W, 260.0);
    assert_eq!(text(&registry, "TooltipTitle"), "Linen Cloth");
    assert_eq!(text(&registry, "TooltipLine0MoneyAmount0"), "2");
    assert_eq!(text(&registry, "TooltipLine0MoneyAmount1"), "60");
    assert_eq!(text(&registry, "TooltipLine1Left"), "Item ID: 2589");
}

#[test]
fn authored_collection_marks_keep_original_atlas_and_omit_unmarked_art() {
    data_root();
    let mut lines = Vec::new();
    for mark in [
        ItemMark::Collected,
        ItemMark::Uncollected,
        ItemMark::Unmarked,
    ] {
        let mut line = TooltipLineState::new("Appearance");
        line.item_mark = Some(mark);
        lines.push(line);
    }
    let registry = render(TooltipPresentation {
        visible: true,
        lines,
        ..TooltipPresentation::hidden()
    });
    for name in ["TooltipLine0Mark", "TooltipLine1Mark"] {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("{name} must be a texture");
        };
        assert!(
            matches!(
                texture.source,
                ui_toolkit::widgets::texture::TextureSource::FileDataId(fdid) if fdid != 0
            ),
            "{name} has no mark art"
        );
    }
    assert!(registry.get_by_name("TooltipLine2Mark").is_none());
    assert_eq!(text(&registry, "TooltipLine2Left"), "Appearance");
}

fn data_root() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
}

fn render(state: TooltipPresentation) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(tooltip_frame_screen).sync(&shared, &mut registry);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match &frame.widget_data {
        Some(WidgetData::FontString(font)) => font.text.clone(),
        _ => panic!("{name} must be a font string"),
    }
}
