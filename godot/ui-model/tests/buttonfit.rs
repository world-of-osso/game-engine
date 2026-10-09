//! Concrete Retail button-fit regressions through production screen builders.
use game_engine_ui_model::{
    auction::{apply_auction_paging_postsetup, native_auction_screen, preview::preview_view},
    professions_frame::{ProfessionView, professions_screen},
};
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
        for count in [0, 7, 100, 9999] {
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

#[test]
fn buttonfit_auction_pagers_use_retail_32px_arrow_buttons_not_overflowing_captions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for view in ["browse", "item", "sell", "bids", "owned"] {
            let mut state = preview_view(view).unwrap();
            state.row_page = 1;
            state.row_pages = 3;
            state.search_page = 1;
            state.search_pages = 3;
            let mut registry = render(state, native_auction_screen, skin);
            apply_auction_paging_postsetup(&mut registry);
            for name in ["AuctionPagePrev", "AuctionPageNext"] {
                let button = frame(&registry, name);
                let Some(WidgetData::Button(text)) = &button.widget_data else {
                    panic!("paging button")
                };
                let (text_width, _) =
                    measure_text(&text.text, GameFont::FrizQuadrata, text.font_size).unwrap();
                let Dimension::Fixed(width) = button.width else {
                    panic!("fixed paging width")
                };
                println!("{skin:?} {view} {name} text={text_width} button={width}");
                assert!(text_width <= width, "{name}: {text_width} exceeds {width}");
                assert_eq!(button.width, Dimension::Fixed(32.0));
                assert_eq!(button.height, Dimension::Fixed(32.0));
                assert!(
                    text.text.is_empty(),
                    "Retail pager is an arrow, not a text panel button"
                );
                assert!(text.enabled);
                assert!(text.normal_texture.is_some());
                assert!(text.pushed_texture.is_some());
                assert!(text.disabled_texture.is_some());
            }
            for name in ["AuctionRowsLabel", "AuctionPageLabel"] {
                let caption = frame(&registry, name);
                let Some(WidgetData::FontString(text)) = &caption.widget_data else {
                    panic!("page summary")
                };
                let (width, _) = measure_text(&text.text, text.font, text.font_size).unwrap();
                let Dimension::Fixed(available) = caption.width else {
                    panic!("fixed summary width")
                };
                assert!(width <= available, "{name}: {width} exceeds {available}");
            }
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
