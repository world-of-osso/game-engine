//! The container window draws as a Retail ContainerFrame under both skins: frame chrome,
//! title, a close button that closes that bag, and one slot background per slot.
use std::path::PathBuf;

use game_engine_ui_model::bag_frame_component::{
    BACKPACK_PORTRAIT, BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
    parse_bag_close_action,
};
use game_engine_ui_model::panel_style_data::{MetalGeometry, MetalTopLeft, metal_frame_style};
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

const SLOTS: usize = 16;

fn backpack() -> BagFrameState {
    BagFrameState {
        bags: vec![BagContainerState {
            bag_index: 0,
            title: "Backpack".into(),
            portrait_fdid: BACKPACK_PORTRAIT,
            visible: true,
            slots: (0..SLOTS)
                .map(|i| BagSlotState {
                    icon_fdid: if i == 0 { 133784 } else { 0 },
                    count: if i == 0 { 20 } else { 0 },
                    quality_border: String::new(),
                    locked: false,
                })
                .collect(),
        }],
    }
}

fn mount() -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(backpack());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    registry.register_panel_style(
        MetalTopLeft::Portrait.style_name(),
        metal_frame_style(
            TextureSource::Dynamic(DynamicTextureId(1)),
            MetalGeometry::active().unwrap(),
        ),
    );
    Screen::new(bag_frame_screen).sync(&ctx, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("no {name}"));
    registry.get(id).unwrap()
}

/// `(x, y, w, h)` of an absolutely placed frame inside its parent.
fn rect(frame: &Frame) -> [f32; 4] {
    let (Val::Px(x), Val::Px(y)) = (frame.position.left, frame.position.top) else {
        panic!("{:?} is not placed in px", frame.name)
    };
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (frame.width, frame.height) else {
        panic!("{:?} has no fixed size", frame.name)
    };
    [x, y, w, h]
}

fn inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
    inner[0] >= 0.0
        && inner[1] >= 0.0
        && inner[0] + inner[2] <= outer[2]
        && inner[1] + inner[3] <= outer[3]
}

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

fn texture_source(frame: &Frame) -> TextureSource {
    let Some(WidgetData::Texture(texture)) = frame.widget_data.as_ref() else {
        panic!("{:?} is not a texture", frame.name)
    };
    texture.source.clone()
}

/// Asserts the open backpack's window parts and returns its slot background sources.
fn assert_backpack_window(skin: ActiveSkin) -> Vec<TextureSource> {
    set_active_skin(skin);
    let registry = mount();
    let window = frame(&registry, "ContainerFrame0");
    assert!(!window.hidden, "{skin:?}: backpack open");
    let bounds = rect(window);

    let border = frame(&registry, "ContainerFrame0NineSlice");
    assert_eq!(border.parent_id, registry.get_by_name("ContainerFrame0"));
    assert!(border.panel_style.is_some(), "{skin:?}: frame border drawn");
    let border_rect = rect(border);
    assert!(
        border_rect[0] < 0.0 && border_rect[2] > bounds[2],
        "{skin:?}: border surrounds the window"
    );

    // The skin's border pieces fit the window: corners and edges meet without overlap.
    let [left, top, right, bottom] = MetalGeometry::active().unwrap().edge_sizes();
    assert!(
        left + right <= border_rect[2] && top + bottom <= border_rect[3],
        "{skin:?}: border pieces fit {border_rect:?}"
    );

    let portrait = frame(&registry, "ContainerFrame0Portrait");
    assert!(
        matches!(texture_source(portrait), TextureSource::FileDataId(fdid) if fdid != 0),
        "{skin:?}: portrait shows the bag icon"
    );
    let portrait_rect = rect(portrait);
    assert!(
        portrait_rect[0] <= 0.0 && portrait_rect[1] <= 0.0,
        "{skin:?}: portrait in the top-left ring"
    );

    let title = frame(&registry, "ContainerFrame0TitleText");
    let Some(WidgetData::FontString(text)) = title.widget_data.as_ref() else {
        panic!("title is not text")
    };
    assert_eq!(text.text, "Backpack");
    let title_rect = rect(title);
    assert!(
        inside(title_rect, bounds) && title_rect[1] < 20.0,
        "{skin:?}: title in the title bar"
    );

    let close = frame(&registry, "ContainerFrame0CloseButton");
    assert_eq!(
        close.onclick.as_deref().and_then(parse_bag_close_action),
        Some(0),
        "{skin:?}: close button closes the backpack"
    );
    let close_rect = rect(close);
    assert!(
        close_rect[1] <= 1.0 && close_rect[0] > bounds[2] / 2.0,
        "{skin:?}: close top-right"
    );

    let mut sources = Vec::new();
    let mut slot_rects: Vec<[f32; 4]> = Vec::new();
    for i in 0..SLOTS {
        let slot = frame(&registry, &format!("ContainerFrame0Slot{i}"));
        let slot_rect = rect(slot);
        assert!(
            inside(slot_rect, bounds),
            "{skin:?}: slot {i} inside the window"
        );
        assert!(
            slot_rect[1] >= 20.0,
            "{skin:?}: slot {i} below the title bar"
        );
        assert!(
            slot_rects.iter().all(|other| !overlaps(*other, slot_rect)),
            "{skin:?}: slot {i} overlaps another"
        );
        slot_rects.push(slot_rect);
        let background = frame(&registry, &format!("ContainerFrame0Slot{i}Background"));
        assert_eq!(
            background.parent_id,
            registry.get_by_name(&format!("ContainerFrame0Slot{i}"))
        );
        let source = texture_source(background);
        assert!(
            matches!(source, TextureSource::FileDataId(fdid) if fdid != 0),
            "{skin:?}: slot {i} background has art"
        );
        sources.push(source);
    }
    // The item still draws over its slot background.
    assert_eq!(
        texture_source(frame(&registry, "ContainerFrame0Slot0Icon")),
        TextureSource::FileDataId(133784)
    );
    sources
}

#[test]
fn open_backpack_draws_a_container_window_under_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let modern = assert_backpack_window(ActiveSkin::Modern);
    let forever = assert_backpack_window(ActiveSkin::Forever);
    assert_ne!(modern, forever, "Forever draws its own slot art");
    set_active_skin(ActiveSkin::Modern);
}
