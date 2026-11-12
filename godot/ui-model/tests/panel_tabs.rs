use std::path::PathBuf;

use game_engine_ui_model::merchant_frame_component::{
    PANEL_TAB_SIDE_WIDTHS, PANEL_TAB_SIDES_PADDING, PanelTab, PanelTabResize, panel_tabs,
    resize_panel_tab,
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widgets::button::ButtonState;

const GROUP_FINDER_FRAME_WIDTH: f32 = 563.0;
const GROUP_FINDER_TAB_LEFT: f32 = 19.0;
const GROUP_FINDER_TAB_TOP: f32 = 426.0;
const PANEL_TAB_ANCHOR_GAP: f32 = 3.0;

fn tab_screen(_: &SharedContext) -> ui_toolkit::widget_def::Element {
    let tabs = [
        PanelTab::new("PanelTab1", "All", "tabs:all", true),
        PanelTab::new("PanelTab2", "Player vs. Player", "tabs:pvp", false),
        PanelTab::new("PanelTab3", "Mythic+ Dungeons", "tabs:mythic", false),
    ];
    let row = panel_tabs(
        &tabs,
        (GROUP_FINDER_TAB_LEFT, GROUP_FINDER_TAB_TOP),
        PanelTabResize {
            max_width: Some((GROUP_FINDER_FRAME_WIDTH - GROUP_FINDER_TAB_LEFT) / tabs.len() as f32),
            ..PanelTabResize::default()
        },
        ui_toolkit::tab_art::TabLayout::Panel,
    );
    rsx! {
        r#frame {
            name: "PanelTabsTestRoot",
            width: 1280.0,
            height: 720.0,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {row}
        }
    }
}

fn registry() -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(ActiveSkin::Forever);
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(tab_screen).sync(&SharedContext::new(), &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .expect(name)
}

fn rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let frame = frame(registry, name);
    let (Val::Px(left), Val::Px(top)) = (frame.position.left, frame.position.top) else {
        panic!("{name} has non-pixel position: {:?}", frame.position);
    };
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (frame.width, frame.height) else {
        panic!(
            "{name} has non-fixed size: {:?} {:?}",
            frame.width, frame.height
        );
    };
    (left, top, width, height)
}

#[test]
fn panel_tab_row_uses_caption_width_anchors_without_overlap_and_draws_selected_last() {
    let registry = registry();
    let labels = ["All", "Player vs. Player", "Mythic+ Dungeons"];
    let child_names: Vec<_> = frame(&registry, "PanelTabsTestRoot")
        .children
        .iter()
        .map(|id| registry.get(*id).unwrap().name.clone().unwrap())
        .collect();
    assert_eq!(child_names, vec!["PanelTab2", "PanelTab3", "PanelTab1"]);
    let Some(WidgetData::Button(selected)) = &frame(&registry, "PanelTab1").widget_data else {
        panic!("selected panel tab is not a button");
    };
    assert_eq!(selected.state, ButtonState::Disabled);
    let resize = PanelTabResize {
        max_width: Some((GROUP_FINDER_FRAME_WIDTH - GROUP_FINDER_TAB_LEFT) / labels.len() as f32),
        ..PanelTabResize::default()
    };
    let rectangles: Vec<_> = (1..=labels.len())
        .map(|index| rect(&registry, &format!("PanelTab{index}")))
        .collect();

    assert_eq!(rectangles[0].0, GROUP_FINDER_TAB_LEFT);
    for index in 0..labels.len() {
        let expected = resize_panel_tab(labels[index], resize);
        let (left, top, width, height) = rectangles[index];
        assert_eq!(top, GROUP_FINDER_TAB_TOP);
        assert_eq!(height, 32.0);
        assert_eq!(width, expected.width, "{} width", labels[index]);

        let text_rect = rect(&registry, &format!("PanelTab{}Text", index + 1));
        assert!(
            text_rect.0 >= 0.0,
            "{} text starts outside its tab",
            labels[index]
        );
        assert!(
            text_rect.0 + text_rect.2 <= width,
            "{} text ends outside its tab",
            labels[index]
        );
        if index > 0 {
            let previous = rectangles[index - 1];
            assert_eq!(left, previous.0 + previous.2 + PANEL_TAB_ANCHOR_GAP);
            assert!(left >= previous.0 + previous.2, "tabs overlap");
        }
    }
    assert_eq!(
        rectangles[0].2, PANEL_TAB_SIDE_WIDTHS,
        "default side caps set the minimum"
    );
    assert!(
        rectangles[2].2 > rectangles[0].2,
        "caption length affects width"
    );
}

#[test]
fn panel_tab_resize_applies_padding_minimum_maximum_and_absolute_width() {
    let font = ui_toolkit::font_object::resolve_font_object("GameFontNormalSmall");
    let (long_text_width, _) = measure_text("Mythic+ Dungeons", font.font, font.font_size).unwrap();
    let natural_width = long_text_width + PANEL_TAB_SIDES_PADDING;
    let natural = resize_panel_tab("Mythic+ Dungeons", PanelTabResize::default());
    assert!((natural.width - natural_width).abs() < 0.01);
    assert_eq!(natural.text_width, long_text_width);

    let padded = resize_panel_tab(
        "Mythic+ Dungeons",
        PanelTabResize {
            padding: 12.0,
            ..PanelTabResize::default()
        },
    );
    assert!((padded.width - natural_width - 12.0).abs() < 0.01);

    let minimum = resize_panel_tab(
        "All",
        PanelTabResize {
            min_width: Some(84.0),
            ..PanelTabResize::default()
        },
    );
    assert_eq!(
        (minimum.width, minimum.text_width),
        (84.0, 84.0 - PANEL_TAB_SIDES_PADDING)
    );

    let maximum = resize_panel_tab(
        "Mythic+ Dungeons",
        PanelTabResize {
            max_width: Some(90.0),
            ..PanelTabResize::default()
        },
    );
    assert_eq!(
        (maximum.width, maximum.text_width),
        (90.0, 90.0 - PANEL_TAB_SIDES_PADDING)
    );

    let absolute = resize_panel_tab(
        "Mythic+ Dungeons",
        PanelTabResize {
            absolute_width: Some(96.0),
            ..PanelTabResize::default()
        },
    );
    assert_eq!(
        (absolute.width, absolute.text_width),
        (96.0, 96.0 - PANEL_TAB_SIDES_PADDING)
    );

    let max_before_min = resize_panel_tab(
        "Mythic+ Dungeons",
        PanelTabResize {
            min_width: Some(100.0),
            max_width: Some(90.0),
            ..PanelTabResize::default()
        },
    );
    assert_eq!(max_before_min.width, 90.0);
}
