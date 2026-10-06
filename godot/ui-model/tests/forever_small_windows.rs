//! Shared merchant/mail metal chrome: base Modern trees and atlas-sheet composition.
#[path = "fixtures/modern_small_window_trees.rs"]
mod fixture;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use game_engine_ui_model::character_frame::{
    CharacterFrameView, PaperDollSlotView, character_frame_screen,
};
use game_engine_ui_model::quest_log_frame_component::{QuestLogFrameState, quest_log_frame_screen};

use game_engine_ui_model::mail_frame_component::{
    MailFrameState, MailFrameTab, OpenMailView, mail_frame_screen,
};
use game_engine_ui_model::merchant_frame_component::{MerchantFrameState, merchant_frame_screen};
use game_engine_ui_model::panel_style_data::{
    MetalGeometry, MetalTopLeft, compose_metal_sheet, metal_frame_style,
};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, set_active_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

const METAL_CORNERS: u32 = 2_406_979;
const METAL_SIDE_EDGES: u32 = 2_406_984;
const METAL_TOP_BOTTOM_EDGES: u32 = 2_406_987;

fn source(fdid: u32) -> Result<(Vec<u8>, u32), String> {
    let (width, height) = match fdid {
        METAL_CORNERS => (512, 512),
        METAL_SIDE_EDGES => (512, 32),
        METAL_TOP_BOTTOM_EDGES => (64, 256),
        8_069_116 => (1024, 512),
        8_069_114 => (512, 256),
        8_069_118 => (256, 512),
        _ => return Err(format!("unexpected sheet {fdid}")),
    };
    let pixels = (0..height)
        .flat_map(|y| (0..width).flat_map(move |x| [x as u8, y as u8, (fdid % 251) as u8, 255]))
        .collect();
    Ok((pixels, width))
}

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

fn dump<T: 'static>(state: T, screen: fn(&SharedContext) -> Element, root: &str) -> String {
    let mut ctx = SharedContext::new();
    ctx.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    registry.register_panel_style(
        MetalTopLeft::Portrait.style_name(),
        metal_frame_style(
            TextureSource::Dynamic(DynamicTextureId(1)),
            MetalGeometry::active().unwrap(),
        ),
    );
    Screen::new(screen).sync(&ctx, &mut registry);
    game_engine_ui_model::mail_frame_component::apply_mail_body_postsetup(&mut registry);
    let mut out = String::new();
    dump_frame(&registry, registry.get_by_name(root).unwrap(), &mut out);
    out
}

fn modern_trees() -> String {
    let mut out = String::new();
    for buyback_tab in [false, true] {
        out += &dump(
            MerchantFrameState {
                visible: true,
                title: "Godric Rothgar".into(),
                buyback_tab,
                repair: Some(true),
                money: 123456,
                ..Default::default()
            },
            merchant_frame_screen,
            "MerchantFrame",
        );
    }
    for tab in [MailFrameTab::Inbox, MailFrameTab::Send] {
        out += &dump(
            MailFrameState {
                visible: true,
                tab,
                money: 123456,
                open: Some(OpenMailView {
                    sender: "Auction House".into(),
                    subject: "Linen Cloth".into(),
                    body: "Your won item is enclosed.".into(),
                    money: 1200,
                    can_delete: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            mail_frame_screen,
            "MailFrame",
        );
    }
    out
}

fn fingerprint(pixels: &[u8]) -> u64 {
    pixels.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

// Retail / Forever AddOns/Blizzard_SharedXML/Mainline/NineSliceLayouts.lua:
// portrait TL :20, plain TL :46, TR :21, BL :22, BR :23, T/B/L/R :24-27.
/// (atlas, Modern sheet cell, Forever sheet cell): every member lands at its own size, so
/// neighbouring pieces share one scale.
const PIECES: [(&str, [u32; 4], [u32; 4]); 9] = [
    (
        "UI-Frame-PortraitMetal-CornerTopLeft",
        [0, 0, 150, 150],
        [0, 0, 190, 190],
    ),
    (
        "UI-Frame-Metal-CornerTopLeft",
        [0, 0, 150, 150],
        [0, 0, 190, 190],
    ),
    (
        "UI-Frame-Metal-CornerTopRight",
        [214, 0, 150, 150],
        [446, 0, 190, 190],
    ),
    (
        "UI-Frame-Metal-CornerBottomLeft",
        [0, 182, 64, 64],
        [0, 446, 190, 200],
    ),
    (
        "UI-Frame-Metal-CornerBottomRight",
        [300, 182, 64, 64],
        [446, 446, 190, 200],
    ),
    (
        "_UI-Frame-Metal-EdgeTop",
        [150, 0, 64, 150],
        [190, 0, 256, 190],
    ),
    (
        "_UI-Frame-Metal-EdgeBottom",
        [150, 182, 32, 64],
        [190, 446, 256, 200],
    ),
    (
        "!UI-Frame-Metal-EdgeLeft",
        [0, 150, 150, 32],
        [0, 190, 190, 256],
    ),
    (
        "!UI-Frame-Metal-EdgeRight",
        [214, 150, 150, 32],
        [446, 190, 190, 256],
    ),
];

/// The atlas member under `skin`: its sheet FileDataID and pixel rect `[x, y, w, h]`.
fn member(name: &str, skin: ActiveSkin) -> (u32, [u32; 4]) {
    let region = resolve_region(name, skin).unwrap_or_else(|| panic!("{name} {skin:?}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("{name} {skin:?} is not a file: {:?}", region.source)
    };
    let (pixels, width) = source(fdid).unwrap();
    let height = pixels.len() as u32 / (width * 4);
    let rect = region.rect_pixels(width, height);
    let [x, y] = rect.min.map(|v| v as u32);
    let [right, bottom] = rect.max.map(|v| v as u32);
    (fdid, [x, y, right - x, bottom - y])
}

/// Both skins resolve every piece to a non-empty member inside its sheet.
fn assert_atlas_regions(skin: ActiveSkin) {
    for (name, _, _) in PIECES {
        let (fdid, [x, y, w, h]) = member(name, skin);
        let (pixels, width) = source(fdid).unwrap();
        let height = pixels.len() as u32 / (width * 4);
        assert!(w > 0 && h > 0, "{name} {skin:?} is empty");
        assert!(
            x + w <= width && y + h <= height,
            "{name} {skin:?} past its sheet"
        );
    }
}

fn assert_composed_members(corner: MetalTopLeft, skin: ActiveSkin) {
    let geometry = MetalGeometry::active().unwrap();
    let (sheet_w, _) = geometry.sheet_size();
    // Display edges are half the sheet cells: one scale for every piece.
    let [left, top, right, bottom] = geometry.edge_sizes();
    assert_eq!(
        [left, top, right, bottom].map(|v| (v * 2.0) as u32),
        [
            geometry.columns[0],
            geometry.rows[0],
            geometry.columns[2],
            geometry.rows[2]
        ]
    );
    let sheet = compose_metal_sheet(corner, geometry, source).unwrap();
    let cell = |modern: [u32; 4], forever: [u32; 4]| {
        if skin == ActiveSkin::Modern {
            modern
        } else {
            forever
        }
    };
    // The bottom edge tiles from the end of the bottom-left corner in member-wide spans.
    let tile_start = cell(PIECES[3].1, PIECES[3].2)[2];
    let tile = member("_UI-Frame-Metal-EdgeBottom", skin).1[2];
    for (name, modern_cell, forever_cell) in PIECES {
        if name == "UI-Frame-PortraitMetal-CornerTopLeft" && corner == MetalTopLeft::Plain {
            continue;
        }
        if name == "UI-Frame-Metal-CornerTopLeft" && corner == MetalTopLeft::Portrait {
            continue;
        }
        let (fdid, [sx, sy, sw, sh]) = member(name, skin);
        let [dx, dy, dw, dh] = cell(modern_cell, forever_cell);
        for (x, y) in [(0, 0), (dw / 2, dh / 2), (dw - 1, dh - 1)] {
            let tile_x = if name == "_UI-Frame-Metal-EdgeBottom" {
                (dx + x - tile_start) % tile
            } else {
                x
            };
            let expected = [
                (sx + tile_x * sw / dw) as u8,
                (sy + y * sh / dh) as u8,
                (fdid % 251) as u8,
                255,
            ];
            let offset = (((dy + y) * sheet_w + dx + x) * 4) as usize;
            assert_eq!(
                &sheet[offset..offset + 4],
                &expected,
                "{name} {skin:?} at {x},{y}"
            );
        }
    }
}

/// The tests switch the process-wide skin.
static SKIN: Mutex<()> = Mutex::new(());

#[test]
#[ignore = "base fixture capture only"]
fn capture_base_trees() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Modern);
    println!("BEGIN_BASE_TREES\n{}END_BASE_TREES", modern_trees());
}

#[test]
fn small_window_chrome_preserves_modern_and_draws_forever_members() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees().as_bytes(), fixture::MODERN_TREES.as_bytes());
    for (corner, hash) in [
        (MetalTopLeft::Portrait, 11562532850027720861),
        (MetalTopLeft::Plain, 10577131485223720269),
    ] {
        assert_eq!(
            fingerprint(
                &compose_metal_sheet(corner, MetalGeometry::active().unwrap(), source).unwrap()
            ),
            hash,
            "base b6e77253 {corner:?}"
        );
    }
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        assert_atlas_regions(skin);
        for corner in [MetalTopLeft::Portrait, MetalTopLeft::Plain] {
            assert_composed_members(corner, skin);
        }
    }
    set_active_skin(ActiveSkin::Modern);
}

/// The member texture from `data/textures`.
fn texture_file(fdid: u32) -> Result<(Vec<u8>, u32), String> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/textures/{fdid}.blp"));
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let image = game_engine_core::blp::decode_rgba(&bytes)?;
    Ok((image.pixels, image.width))
}

/// Display rows, from the top of the border frame, between the two bright rules of the
/// composed top edge: the band the window title sits in.
fn title_bar_band(geometry: MetalGeometry) -> (f32, f32) {
    let sheet = compose_metal_sheet(MetalTopLeft::Portrait, geometry, texture_file).unwrap();
    let (sheet_w, _) = geometry.sheet_size();
    let x = geometry.columns[0] + geometry.columns[1] / 2;
    let bright: Vec<u32> = (0..geometry.rows[0])
        .filter(|y| {
            let at = ((y * sheet_w + x) * 4) as usize;
            let [r, g, b, a] = [0, 1, 2, 3].map(|i| u32::from(sheet[at + i]));
            a > 128 && (r + g + b) / 3 > 80
        })
        .collect();
    // Runs of consecutive bright rows: (first, last).
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for y in bright {
        match runs.last_mut() {
            Some(run) if run.1 + 1 == y => run.1 = y,
            _ => runs.push((y, y)),
        }
    }
    assert!(runs.len() >= 2, "top edge rules {runs:?}");
    let (upper, lower) = (runs[0], runs[runs.len() - 1]);
    ((upper.1 + 1) as f32 / 2.0, lower.0 as f32 / 2.0)
}

fn px(value: Val) -> f32 {
    let Val::Px(v) = value else {
        panic!("{value:?}")
    };
    v
}

fn fixed(value: Dimension) -> f32 {
    let Dimension::Fixed(v) = value else {
        panic!("{value:?}")
    };
    v
}

fn forever_window<T: 'static>(state: T, screen: fn(&SharedContext) -> Element) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(screen).sync(&ctx, &mut registry);
    game_engine_ui_model::mail_frame_component::apply_mail_body_postsetup(&mut registry);
    registry
}

#[test]
fn forever_window_titles_sit_inside_the_title_bar_band() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_active_skin(ActiveSkin::Forever);
    let (band_top, band_bottom) = title_bar_band(MetalGeometry::active().unwrap());
    let windows = [
        (
            forever_window(
                MerchantFrameState {
                    visible: true,
                    title: "Brother Danil".into(),
                    ..Default::default()
                },
                merchant_frame_screen,
            ),
            "MerchantFrame",
        ),
        (
            forever_window(
                QuestLogFrameState {
                    visible: true,
                    ..Default::default()
                },
                quest_log_frame_screen,
            ),
            "QuestLogFrame",
        ),
        (
            forever_window(
                CharacterFrameView {
                    visible: true,
                    title: "Shot".into(),
                    slots: vec![PaperDollSlotView::default(); 18],
                    race_id: 1,
                    class_id: 2,
                    ..Default::default()
                },
                character_frame_screen,
            ),
            "CharacterFrame",
        ),
    ];
    for (registry, window) in &windows {
        let frame = |suffix: &str| {
            let name = format!("{window}{suffix}");
            registry
                .get(
                    registry
                        .get_by_name(&name)
                        .unwrap_or_else(|| panic!("missing {name}")),
                )
                .unwrap()
        };
        let border_top = px(frame("NineSlice").position.top);
        let title = frame("TitleText");
        let Some(WidgetData::FontString(_)) = &title.widget_data else {
            panic!("{window}TitleText is not text")
        };
        let title_top = px(title.position.top);
        let title_bottom = title_top + fixed(title.height);
        let (top, bottom) = (border_top + band_top, border_top + band_bottom);
        assert!(
            top <= title_top && title_bottom <= bottom,
            "{window}: title {title_top}..{title_bottom} outside the title bar {top}..{bottom}"
        );
    }
    set_active_skin(ActiveSkin::Modern);
}

/// The close button's TOPRIGHT offset: Retail overhangs the window's right edge by 1 at its
/// top (+1, 0) (Mainline/SharedUIPanelTemplates.lua:139-141); Forever tucks it inside the
/// right edge and lifts it above the top (-2, +1) (Camelot/SharedUIPanelTemplates.lua:3-5).
#[test]
fn forever_close_button_sits_inside_the_right_edge_and_above_the_top() {
    let _skin = SKIN.lock().unwrap_or_else(|poison| poison.into_inner());
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let close_button = |skin: ActiveSkin| {
        set_active_skin(skin);
        let registry = forever_window(
            MerchantFrameState {
                visible: true,
                title: "Brother Danil".into(),
                ..Default::default()
            },
            merchant_frame_screen,
        );
        let frame = |name: &str| {
            registry
                .get(
                    registry
                        .get_by_name(name)
                        .unwrap_or_else(|| panic!("missing {name}")),
                )
                .unwrap()
        };
        let window_w = fixed(frame("MerchantFrame").width);
        let button = frame("MerchantFrameCloseButton");
        let right = px(button.position.left) + fixed(button.width);
        (right - window_w, px(button.position.top))
    };
    let (modern_overhang, modern_top) = close_button(ActiveSkin::Modern);
    let (forever_overhang, forever_top) = close_button(ActiveSkin::Forever);
    set_active_skin(ActiveSkin::Modern);
    assert!(
        modern_overhang > 0.0 && modern_top == 0.0,
        "Modern {modern_overhang},{modern_top}"
    );
    assert!(
        forever_overhang < 0.0,
        "Forever right edge {forever_overhang} past the window"
    );
    assert!(
        forever_top < modern_top,
        "Forever top {forever_top} not above Modern {modern_top}"
    );
}
