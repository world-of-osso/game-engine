//! Existing Professions button-fit case, retaining its original module/test name.
use game_engine_ui_professions::professions_frame::{ProfessionView, professions_screen};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, Frame, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    text_measure::measure_text,
    widget_def::Element,
    widgets::font_string::GameFont,
};

fn render<T: 'static>(
    state: T,
    build: fn(&SharedContext) -> Element,
    skin: ActiveSkin,
) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(build).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

#[test]
fn buttonfit_create_all_keeps_retail_padding_and_right_anchor_for_concrete_counts() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (count, expected_width) in [(0, 114.0), (7, 114.0), (100, 129.0), (9999, 137.0)] {
            let mut view = ProfessionView {
                craftable: count,
                can_all: count > 0,
                ..Default::default()
            };
            view.book.visible = true;
            let registry = render(view, professions_screen, skin);
            let button = frame(&registry, "ProfessionsCreateAll");
            let Some(WidgetData::Button(text)) = &button.widget_data else {
                panic!("Create All button")
            };
            assert_eq!(text.text, format!("Create All [{count}]"));
            assert_eq!(text.font_size, 12.0);
            let (width, _) =
                measure_text(&text.text, GameFont::FrizQuadrata, text.font_size).unwrap();
            let Dimension::Fixed(button_width) = button.width else {
                panic!("fixed fitted width")
            };
            println!("{skin:?} count={count} text={width} button={button_width}");
            assert_eq!(button_width, expected_width);
            assert!(
                width <= button_width - 40.0,
                "{skin:?} {count}: text {width} exceeds {button_width}-40"
            );
            let Val::Px(left) = button.position.left else {
                panic!("absolute Retail anchor")
            };
            assert_eq!(left + button_width, 762.0);
            assert_eq!(button.height, Dimension::Fixed(22.0));
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
