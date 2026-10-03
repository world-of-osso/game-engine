//! Shared merchant/mail metal chrome: base Modern trees and atlas-sheet composition.
#[path = "fixtures/modern_small_window_trees.rs"]
mod fixture;
use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::mail_frame_component::{
    MailFrameState, MailFrameTab, OpenMailView, mail_frame_screen,
};
use game_engine_ui_model::merchant_frame_component::{MerchantFrameState, merchant_frame_screen};
use game_engine_ui_model::panel_style_data::{
    MetalTopLeft, compose_metal_sheet, metal_frame_style,
};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, set_active_skin};
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
        metal_frame_style(TextureSource::Dynamic(DynamicTextureId(1))),
    );
    Screen::new(screen).sync(&ctx, &mut registry);
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
const PIECES: [(&str, [u32; 4], [u32; 4], [u32; 4]); 9] = [
    (
        "UI-Frame-PortraitMetal-CornerTopLeft",
        [1, 153, 150, 150],
        [385, 193, 190, 190],
        [0, 0, 150, 150],
    ),
    (
        "UI-Frame-Metal-CornerTopLeft",
        [1, 1, 150, 150],
        [193, 1, 190, 190],
        [0, 0, 150, 150],
    ),
    (
        "UI-Frame-Metal-CornerTopRight",
        [153, 1, 150, 150],
        [193, 193, 190, 190],
        [214, 0, 150, 150],
    ),
    (
        "UI-Frame-Metal-CornerBottomLeft",
        [153, 153, 64, 64],
        [1, 1, 190, 200],
        [0, 182, 64, 64],
    ),
    (
        "UI-Frame-Metal-CornerBottomRight",
        [219, 153, 64, 64],
        [1, 203, 190, 200],
        [300, 182, 64, 64],
    ),
    (
        "_UI-Frame-Metal-EdgeTop",
        [0, 1, 64, 150],
        [0, 203, 256, 190],
        [150, 0, 64, 150],
    ),
    (
        "_UI-Frame-Metal-EdgeBottom",
        [0, 153, 32, 64],
        [0, 1, 256, 200],
        [150, 182, 32, 64],
    ),
    (
        "!UI-Frame-Metal-EdgeLeft",
        [1, 0, 150, 32],
        [1, 0, 190, 256],
        [0, 150, 150, 32],
    ),
    (
        "!UI-Frame-Metal-EdgeRight",
        [153, 0, 150, 32],
        [193, 0, 190, 256],
        [214, 150, 150, 32],
    ),
];

fn sheet_for(name: &str, skin: ActiveSkin) -> u32 {
    match (
        name.contains("EdgeLeft") || name.contains("EdgeRight"),
        name.contains("Edge"),
        skin,
    ) {
        (true, _, ActiveSkin::Modern) => METAL_SIDE_EDGES,
        (true, _, ActiveSkin::Forever) => 8_069_114,
        (_, true, ActiveSkin::Modern) => METAL_TOP_BOTTOM_EDGES,
        (_, true, ActiveSkin::Forever) => 8_069_118,
        (_, _, ActiveSkin::Modern) => METAL_CORNERS,
        (_, _, ActiveSkin::Forever) => 8_069_116,
    }
}

fn assert_atlas_regions(skin: ActiveSkin) {
    for (name, modern, forever, _) in PIECES {
        let fdid = sheet_for(name, skin);
        let (pixels, width) = source(fdid).unwrap();
        let height = pixels.len() as u32 / (width * 4);
        let region = resolve_region(name, skin).unwrap();
        assert_eq!(
            region.source,
            AtlasSource::FileDataId(fdid),
            "{name} {skin:?}"
        );
        let rect = region.rect_pixels(width, height);
        let [x, y, w, h] = if skin == ActiveSkin::Modern {
            modern
        } else {
            forever
        };
        assert_eq!(rect.min, [x as f32, y as f32], "{name} {skin:?}");
        assert_eq!(
            rect.max,
            [(x + w) as f32, (y + h) as f32],
            "{name} {skin:?}"
        );
    }
}

fn assert_composed_members(corner: MetalTopLeft, skin: ActiveSkin) {
    let sheet = compose_metal_sheet(corner, source).unwrap();
    for (name, modern, forever, [dx, dy, dw, dh]) in PIECES {
        if name == "UI-Frame-PortraitMetal-CornerTopLeft" && corner == MetalTopLeft::Plain {
            continue;
        }
        if name == "UI-Frame-Metal-CornerTopLeft" && corner == MetalTopLeft::Portrait {
            continue;
        }
        let fdid = sheet_for(name, skin);
        let [sx, sy, sw, sh] = if skin == ActiveSkin::Modern {
            modern
        } else {
            forever
        };
        for (x, y) in [(0, 0), (dw / 2, dh / 2), (dw - 1, dh - 1)] {
            // The bottom edge is tiled from x=64 in fixed 32-pixel spans.
            let tile_x = if name == "_UI-Frame-Metal-EdgeBottom" {
                (dx + x - 64) % 32
            } else {
                x
            };
            let expected = [
                (sx + tile_x * sw / dw) as u8,
                (sy + y * sh / dh) as u8,
                (fdid % 251) as u8,
                255,
            ];
            let offset = (((dy + y) * 364 + dx + x) * 4) as usize;
            assert_eq!(
                &sheet[offset..offset + 4],
                &expected,
                "{name} {skin:?} at {x},{y}"
            );
        }
    }
}

#[test]
fn small_window_chrome_preserves_modern_and_draws_forever_members() {
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
            fingerprint(&compose_metal_sheet(corner, source).unwrap()),
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
