//! Character/paperdoll first slice: Modern base bytes and Camelot art/layout.
#[path = "fixtures/modern_character_trees.rs"]
mod fixture;
use game_engine_ui_model::character_frame::{
    CharacterFrameView, LevelLine, PaperDollSlotView, StatLine, apply_character_frame_postsetup,
    character_frame_screen,
};
use std::fmt::Write;
use std::path::PathBuf;
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn dump_frame(registry: &FrameRegistry, id: u64, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(out, "{:?} {:?} size={:?},{:?} pos={:?},{:?} anchor={:?} translate={:?} margin={:?} layout={:?} visibility={},{} alpha={},{} scale={},{} strata={:?} level={} raise={} layer={:?},{} input={},{},{:?} bg={:?} backdrop={:?} nine={:?} three={:?} border={:?} style={:?},{:?} behavior={},{},{} click={:?} flex={:?} data={:?}",
        f.name, f.widget_type, f.width, f.height, f.position, f.position_type, f.anchor,
        f.translation, f.margin, f.layout_rect, f.hidden, f.visible, f.alpha, f.effective_alpha,
        f.scale, f.effective_scale, f.strata, f.frame_level, f.raise_order, f.draw_layer,
        f.draw_sub_layer, f.mouse_enabled, f.keyboard_enabled, f.hit_rect_insets,
        f.background_color, f.backdrop, f.nine_slice, f.three_slice, f.border,
        f.panel_style, f.three_slice_style, f.clamped_to_screen, f.movable, f.resizable,
        f.onclick, f.flex_layout, f.widget_data).unwrap();
    for child in &f.children {
        dump_frame(registry, *child, out);
    }
}

fn view(populated: bool) -> CharacterFrameView {
    let mut view = CharacterFrameView {
        visible: populated,
        title: "Azerothia".into(),
        level: LevelLine {
            level: "Level 60 ".into(),
            class_text: "Protection Warrior".into(),
            class_color: "0.78,0.61,0.43,1.0".into(),
        },
        slots: vec![PaperDollSlotView::default(); 18],
        item_level: populated.then(|| "42".into()),
        attributes: Vec::new(),
        enhancements: Vec::new(),
        race_id: 1,
        class_id: 1,
        ..CharacterFrameView::default()
    };
    if populated {
        view.slots[0] = PaperDollSlotView {
            icon_fdid: 135274,
            quality_border: "0.0,1.0,0.0,1.0",
            count: 2,
            locked: true,
        };
        view.attributes = vec![
            StatLine {
                label: "Strength:",
                value: "125".into(),
            },
            StatLine {
                label: "Stamina:",
                value: "98".into(),
            },
            StatLine {
                label: "Armor:",
                value: "2,450".into(),
            },
        ];
        view.enhancements = vec![
            StatLine {
                label: "Critical Strike:",
                value: "12%".into(),
            },
            StatLine {
                label: "Haste:",
                value: "5%".into(),
            },
        ];
    }
    view
}

fn build(view: CharacterFrameView) -> FrameRegistry {
    let mut shared = SharedContext::new();
    let tab = view.tab;
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry, tab);
    registry
}

fn modern_trees() -> String {
    let mut out = String::new();
    for populated in [false, true] {
        let registry = build(view(populated));
        dump_frame(
            &registry,
            registry.get_by_name("CharacterFrame").unwrap(),
            &mut out,
        );
    }
    out
}

use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

fn frame<'a>(registry: &'a FrameRegistry, node: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(node)
                .unwrap_or_else(|| panic!("missing {node}")),
        )
        .unwrap()
}

/// The art `node` draws: a source and a non-empty crop of it.
fn art(registry: &FrameRegistry, node: &str) -> String {
    let Some(WidgetData::Texture(TextureData {
        source, tex_coords, ..
    })) = &frame(registry, node).widget_data
    else {
        panic!("not a texture: {node}")
    };
    assert_ne!(*source, TextureSource::None, "{node} draws nothing");
    let [left, right, top, bottom] = *tex_coords;
    assert!(
        left != right && top != bottom,
        "{node}: empty crop {tex_coords:?}"
    );
    format!("{source:?} {tex_coords:?}")
}

/// `(left, top)` of a frame placed with pixel offsets.
fn pos(registry: &FrameRegistry, node: &str) -> (f32, f32) {
    let f = frame(registry, node);
    let (Val::Px(x), Val::Px(y)) = (f.position.left, f.position.top) else {
        panic!("{node} not placed in pixels: {:?}", f.position)
    };
    (x, y)
}

/// `(width, height)` of a fixed-size frame.
fn size(registry: &FrameRegistry, node: &str) -> (f32, f32) {
    let f = frame(registry, node);
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (f.width, f.height) else {
        panic!("{node} not fixed: {:?} {:?}", f.width, f.height)
    };
    (w, h)
}

/// `(left, top, width, height)` of a frame placed with fixed pixels.
fn rect(registry: &FrameRegistry, node: &str) -> (f32, f32, f32, f32) {
    let ((x, y), (w, h)) = (pos(registry, node), size(registry, node));
    (x, y, w, h)
}

/// The tests switch the process-wide skin.
static SKIN: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn character_frame_skin_art_and_layout_preserve_modern_bytes() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees().as_bytes(), fixture::TREES.as_bytes());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let registry = build(view(true));
        for node in [
            "CharacterStatsPaneItemLevelCategoryBackground",
            "CharacterStatsPaneStat2Background",
            "CharacterStatsPaneItemLevelFrameBackground",
            "CharacterFrameCloseButtonNormal",
            "CharacterFrameBackground",
        ] {
            art(&registry, node);
        }
        // Every class draws its own stats background.
        let mut class_art: Vec<String> = (1..=12)
            .map(|class| {
                let mut data = view(true);
                data.class_id = class;
                art(&build(data), "CharacterStatsPaneClassBackground")
            })
            .collect();
        class_art.sort();
        class_art.dedup();
        assert_eq!(class_art.len(), 12, "{skin:?}: classes share art");
        if skin == ActiveSkin::Modern {
            art(&registry, "CharacterFrameTopTileStreaks");
            // The selected tab is drawn differently from the other.
            for suffix in ["Left", "Middle", "Right"] {
                assert_ne!(
                    art(&registry, &format!("CharacterFrameTab1{suffix}")),
                    art(&registry, &format!("CharacterFrameTab2{suffix}")),
                    "{suffix}"
                );
            }
            continue;
        }
        for node in [
            "CharacterFrameInsetRightBg",
            "CharacterFrameStoneBg",
            "CharacterModelFrameBackgroundOverlay",
            "CharacterFrameDivider",
            "CharacterLevelTextBackground",
        ] {
            art(&registry, node);
        }
        // Both side tabs share one background; the selected one adds its own highlight.
        let tab = art(&registry, "CharacterFrameTab1Background");
        assert_eq!(art(&registry, "CharacterFrameTab2Background"), tab);
        assert_ne!(art(&registry, "CharacterFrameTab1Selected"), tab);
        // Every paperdoll slot wears the same frame.
        let slots: Vec<String> = game_engine_ui_model::character_frame::PAPERDOLL_BUTTONS
            .iter()
            .map(|button| art(&registry, &format!("{}Frame", button.name)))
            .collect();
        assert!(slots.iter().all(|slot| *slot == slots[0]), "{slots:?}");

        // The model scene and the stats pane tile the frame side by side; the model fills
        // the frame's height below the title.
        let (frame_w, frame_h) = size(&registry, "CharacterFrame");
        let model = rect(&registry, "CharacterModelScene");
        let stats = rect(&registry, "CharacterStatsPaneClassBackground");
        assert_eq!(model.0, 0.0);
        assert_eq!(
            model.0 + model.2,
            stats.0,
            "stats pane starts where the model ends"
        );
        assert_eq!(
            stats.0 + stats.2,
            frame_w,
            "stats pane reaches the frame's right edge"
        );
        assert_eq!(
            model.1 + model.3,
            frame_h,
            "model reaches the frame's bottom"
        );
        // Side tabs hang off the frame's right edge, stacked without overlap.
        let tab1 = rect(&registry, "CharacterFrameTab1");
        let tab2 = pos(&registry, "CharacterFrameTab2");
        assert_eq!((tab1.0, tab2.0), (frame_w, frame_w));
        assert!(tab2.1 >= tab1.1 + tab1.3, "tabs overlap");
        let tab_bg = size(&registry, "CharacterFrameTab1Background");
        assert_eq!(tab_bg.0, tab1.2);
        assert!(tab_bg.1 >= tab1.3);
        // Slot columns: head and hands share a row, head and wrist a column, the weapons
        // sit side by side on the bottom row.
        let slot = |name: &str| pos(&registry, name);
        assert_eq!(slot("CharacterHeadSlot").1, slot("CharacterHandsSlot").1);
        assert!(slot("CharacterHeadSlot").0 < slot("CharacterHandsSlot").0);
        assert_eq!(slot("CharacterHeadSlot").0, slot("CharacterWristSlot").0);
        assert!(slot("CharacterWristSlot").1 > slot("CharacterHeadSlot").1);
        let (main_hand, off_hand) = (
            slot("CharacterMainHandSlot"),
            slot("CharacterSecondaryHandSlot"),
        );
        assert_eq!(main_hand.1, off_hand.1);
        assert!(main_hand.1 > slot("CharacterWristSlot").1);
        assert!(main_hand.0 < off_hand.0);
        let head_frame = size(&registry, "CharacterHeadSlotFrame");
        assert_eq!(head_frame.0, head_frame.1, "square slot frame");
        assert!(
            registry
                .get_by_name("PaperDollInnerBorderTopLeft")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("CharacterMainHandSlotFrameCap")
                .is_none()
        );
        assert!(registry.get_by_name("CharacterFrameTab1Left").is_none());
        assert!(registry.get_by_name("CharacterRangedSlot").is_none());
        assert!(registry.get_by_name("CharacterAmmoSlot").is_none());
        assert!(
            registry
                .get_by_name("CharacterFrameTopTileStreaks")
                .is_none()
        );
        for node in [
            "CharacterStatsPaneStat2Label",
            "CharacterStatsPaneStat2Value",
            "CharacterStatsPaneItemLevelCategoryTitle",
        ] {
            let Some(WidgetData::FontString(text)) = &frame(&registry, node).widget_data else {
                panic!("not text: {node}")
            };
            assert_eq!(text.font_size, 12.0, "{node}");
        }
        for node in [
            "CharacterStatsPaneStat2Background",
            "CharacterStatsPaneItemLevelFrameBackground",
        ] {
            let Some(WidgetData::Texture(texture)) = &frame(&registry, node).widget_data else {
                panic!("not a texture")
            };
            assert_eq!(texture.vertex_color, [1.0; 4], "{node}");
        }
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn forever_stats_class_art_stays_inside_the_character_frame() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Forever);
    let size = |frame: &ui_toolkit::frame::Frame| match (frame.width, frame.height) {
        (Dimension::Fixed(w), Dimension::Fixed(h)) => (w, h),
        other => panic!("{other:?}"),
    };
    for class_id in 1..=12 {
        let mut data = view(true);
        data.class_id = class_id;
        let registry = build(data);
        let get = |name: &str| registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let (frame_w, frame_h) = size(get("CharacterFrame"));
        let art = get("CharacterStatsPaneClassBackground");
        let (w, h) = size(art);
        let (Val::Px(x), Val::Px(y)) = (art.position.left, art.position.top) else {
            panic!("{:?}", art.position)
        };
        assert!(
            x >= 0.0 && y >= 0.0 && x + w <= frame_w && y + h <= frame_h,
            "class {class_id}: art {x},{y} {w}x{h} outside the {frame_w}x{frame_h} frame"
        );
    }
    set_active_skin(ActiveSkin::Modern);
}
