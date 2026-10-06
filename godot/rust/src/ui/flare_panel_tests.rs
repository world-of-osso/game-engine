//! Under the Forever skin the chat frame and damage meter draw FlareUI's `flare_bronze`
//! panel (Blizzard's `UI-Tooltip-Border` over `UI-DialogBox-Background-Dark`) in FlareUI's
//! colours, tooltips keep Blizzard's nine-slice in FlareUI's border and background colours,
//! and a skin switch re-skins the live canvases.

use game_engine_ui_model::chat_frame_component::{CHAT_BACKGROUND, chat_frame_screen};
use game_engine_ui_model::damage_meter_component::{DAMAGE_METER_ROOT, damage_meter_screen};
use game_engine_ui_model::flare_panel::{FLARE_BRONZE_PANEL_STYLE, FLARE_SHEET};
use game_engine_ui_model::game_tooltip::game_tooltip_screen;
use game_engine_ui_model::tooltip_presentation::TooltipBorder;
use shared::faction_reaction::Reaction;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{NineSlice, WidgetData};
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::TextureSource;

use super::modern_panel_snapshot_tests::{chat_view, meter_view, tooltip_view};
use super::parts::project_images;
use super::ui_parent::UiParent;
use super::{RegistryModel, ScreenPostsetup, register_flare_bronze_style};

const BRONZE: [f32; 4] = [0.65, 0.49, 0.27, 1.0];
const TOOLTIP_DEFAULT: [f32; 4] = [0.80, 0.60, 0.34, 1.0];
const TOOLTIP_BG: [f32; 4] = [0.05, 0.05, 0.06, 0.9];

/// `data/textures/{fdid}.blp` decoded, without the Godot resource paths.
fn load_texture(fdid: u32) -> Result<(Vec<u8>, u32), String> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../data/textures/{fdid}.blp"));
    let bytes = std::fs::read(&path).map_err(|error| format!("{path:?}: {error}"))?;
    let image = game_engine_core::blp::decode_rgba(&bytes).map_err(|error| error.to_string())?;
    Ok((image.pixels, image.width))
}

fn set_data_root() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

/// A HUD canvas as `show_viewport_screen` builds it, synced to `skin`.
fn canvas<T: 'static>(
    state: T,
    build: fn(&SharedContext) -> Element,
    skin: ActiveSkin,
) -> RegistryModel {
    set_data_root();
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut registry = UiParent::for_viewport(2732.0, 1536.0).registry();
    register_flare_bronze_style(&mut registry, load_texture).unwrap();
    let mut model = RegistryModel {
        screen: Screen::new(build),
        shared,
        registry,
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    model.sync_skin(skin);
    model
}

fn slice<'a>(model: &'a RegistryModel, name: &str) -> &'a NineSlice {
    let id = model
        .registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("no {name}"));
    let frame = model.registry.get(id).unwrap();
    assert_eq!(frame.panel_style.as_deref(), Some(FLARE_BRONZE_PANEL_STYLE));
    frame
        .nine_slice
        .as_ref()
        .unwrap_or_else(|| panic!("{name} has no nine-slice"))
}

fn has(model: &RegistryModel, name: &str) -> bool {
    model.registry.get_by_name(name).is_some()
}

#[test]
fn forever_chat_and_meter_draw_the_bronze_panel_at_flareui_opacity() {
    let chat = canvas(chat_view(), chat_frame_screen, ActiveSkin::Forever);
    let skin = slice(&chat, "ChatFrame1FlareSkin");
    assert_eq!(skin.border_color, BRONZE);
    assert_eq!(skin.bg_color, [1.0, 1.0, 1.0, 0.6]);
    assert_eq!((skin.edge_size, skin.center_inset), (16.0, Some(3.0)));
    let frame = chat
        .registry
        .get(chat.registry.get_by_name("ChatFrame1FlareSkin").unwrap())
        .unwrap();
    let parts = project_images(frame, 450.0, 214.0);
    assert_eq!(parts.len(), 9, "no opaque fill behind the backdrop");
    assert_eq!(parts[0].color, [1.0, 1.0, 1.0, 0.6]);
    assert!(parts[1..].iter().all(|part| part.color == BRONZE));
    assert!(matches!(skin.texture, Some(TextureSource::Dynamic(_))));
    assert!(
        !has(&chat, CHAT_BACKGROUND.0),
        "FlareUI strips the chat background"
    );

    let meter = canvas(meter_view(), damage_meter_screen, ActiveSkin::Forever);
    let skin = slice(&meter, "DamageMeterFlareSkin");
    assert_eq!(
        (skin.border_color, skin.bg_color),
        (BRONZE, [1.0, 1.0, 1.0, 0.6])
    );
    assert!(!has(&meter, "DamageMeterBackground"));
    assert!(!has(&meter, "DamageMeterHeader"));
    assert!(has(&meter, DAMAGE_METER_ROOT.0));
}

#[test]
fn forever_skin_panels_sit_where_flareui_anchors_them() {
    use std::collections::HashMap;
    let rect = |model: &RegistryModel, name: &str| {
        let bounds =
            crate::ui::layout::compute_layout_with_intrinsics(&model.registry, &HashMap::new())
                .unwrap();
        let r = &bounds[&model.registry.get_by_name(name).unwrap()];
        (r.x, r.y, r.width, r.height)
    };
    let chat = canvas(chat_view(), chat_frame_screen, ActiveSkin::Forever);
    let (frame_x, frame_y, _, _) = rect(&chat, "ChatFrame1Messages");
    let (x, y, w, h) = rect(&chat, "ChatFrame1FlareSkin");
    // textPadding 10 around the messages, headerHeight 24 above them.
    assert_eq!((x, y), (frame_x - 10.0, frame_y - 34.0));
    let (_, _, msg_w, msg_h) = rect(&chat, "ChatFrame1Messages");
    assert_eq!((w, h), (msg_w + 20.0, msg_h + 44.0));

    let meter = canvas(meter_view(), damage_meter_screen, ActiveSkin::Forever);
    let (mx, my, mw, mh) = rect(&meter, DAMAGE_METER_ROOT.0);
    assert_eq!(
        rect(&meter, "DamageMeterFlareSkin"),
        (mx - 2.0, my - 2.0, mw + 4.0, mh + 4.0)
    );
}

#[test]
fn backdrop_centre_spans_under_the_edges_from_the_inset() {
    let chat = canvas(chat_view(), chat_frame_screen, ActiveSkin::Forever);
    let id = chat.registry.get_by_name("ChatFrame1FlareSkin").unwrap();
    let frame = chat.registry.get(id).unwrap();
    let parts = project_images(frame, 481.0, 259.0);
    assert_eq!(parts.len(), 9);
    // The centre goes first, inset 3, in the background colour; the edges follow in bronze.
    assert_eq!(parts[0].rect, [3.0, 3.0, 475.0, 253.0]);
    assert_eq!(parts[0].color, [1.0, 1.0, 1.0, 0.6]);
    assert_eq!(parts[1].rect, [0.0, 0.0, 16.0, 16.0]);
    assert!(parts[1..].iter().all(|part| part.color == BRONZE));
}

#[test]
fn bronze_sheet_turns_the_top_and_bottom_edges_to_run_along_the_frame() {
    set_data_root();
    let pixels =
        game_engine_ui_model::flare_panel::compose_flare_bronze_sheet(load_texture).unwrap();
    let alpha = |x: u32, y: u32| pixels[((y * FLARE_SHEET.0 + x) * 4 + 3) as usize];
    // Left edge cell (0, 14): the border line runs down its left side.
    assert!(alpha(2, 21) > 200 && alpha(10, 21) == 0);
    // Top edge cell (14, 0): the same line, turned to run along its top.
    assert!(alpha(21, 2) > 200 && alpha(21, 10) == 0);
    // Bottom edge cell (14, 28): along its bottom.
    assert!(alpha(21, 39) > 200 && alpha(21, 31) == 0);
    // Centre: the dark dialog background, black at alpha 221.
    assert_eq!(
        &pixels[((21 * FLARE_SHEET.0 + 21) * 4) as usize..][..4],
        &[0, 0, 0, 221]
    );
}

/// `(fdid, vertex colour)` of the texture `name`.
fn piece(model: &RegistryModel, name: &str) -> (u32, [f32; 4]) {
    let id = model
        .registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("no {name}"));
    match &model.registry.get(id).unwrap().widget_data {
        Some(WidgetData::Texture(texture)) => match texture.source {
            TextureSource::FileDataId(fdid) => (fdid, texture.vertex_color),
            ref other => panic!("{name} draws {other:?}"),
        },
        other => panic!("{name} is not a texture: {other:?}"),
    }
}

/// `TooltipDefaultLayout` border pieces.
const TOOLTIP_BORDER_PIECES: [&str; 8] = [
    "TopLeftCorner",
    "TopRightCorner",
    "BottomLeftCorner",
    "BottomRightCorner",
    "TopEdge",
    "BottomEdge",
    "LeftEdge",
    "RightEdge",
];

/// The art the tooltip nine-slice's centre and border pieces draw.
fn tooltip_art(tooltip: &RegistryModel) -> Vec<u32> {
    std::iter::once("Center")
        .chain(TOOLTIP_BORDER_PIECES)
        .map(|part| piece(tooltip, &format!("TooltipNineSlice{part}")).0)
        .collect()
}

fn tooltip(border: TooltipBorder, skin: ActiveSkin) -> RegistryModel {
    let mut view = tooltip_view();
    view.main.border = border;
    canvas(view, game_tooltip_screen, skin)
}

/// FlareUI keeps Blizzard's tooltip nine-slice: the same art as Modern, every border piece
/// tinted one colour (`NineSlice:SetBorderColor`) and the centre at 0.05/0.05/0.06 alpha 0.9
/// (`SetCenterColor`, Tooltips.lua:138-146). Returns that border colour.
fn forever_tooltip_border(subject: TooltipBorder) -> [f32; 4] {
    let forever = tooltip(subject, ActiveSkin::Forever);
    assert!(!has(&forever, "TooltipFlareBackdrop"), "no bronze panel");
    assert_eq!(
        tooltip_art(&forever),
        tooltip_art(&tooltip(subject, ActiveSkin::Modern)),
        "Blizzard's nine-slice art"
    );
    assert_eq!(piece(&forever, "TooltipNineSliceCenter").1, TOOLTIP_BG);
    let colors: Vec<[f32; 4]> = TOOLTIP_BORDER_PIECES
        .iter()
        .map(|part| piece(&forever, &format!("TooltipNineSlice{part}")).1)
        .collect();
    assert!(colors.iter().all(|color| *color == colors[0]));
    colors[0]
}

#[test]
fn forever_tooltip_tints_blizzards_nine_slice_by_class_reaction_and_quality() {
    assert_eq!(
        forever_tooltip_border(TooltipBorder::Default),
        TOOLTIP_DEFAULT
    );
    assert_eq!(
        forever_tooltip_border(TooltipBorder::Reaction(Reaction::Hostile)),
        [0.78, 0.28, 0.24, 1.0]
    );
    assert_eq!(
        forever_tooltip_border(TooltipBorder::Reaction(Reaction::Friendly)),
        [0.35, 0.65, 0.38, 1.0]
    );
    // Mage RAID_CLASS_COLORS, unmuted.
    assert_eq!(
        forever_tooltip_border(TooltipBorder::Class(8)),
        [0.25, 0.78, 0.92, 1.0]
    );
    // Rare 0.0, 0.44, 0.87 muted by 0.85.
    let [r, g, b, a] = forever_tooltip_border(TooltipBorder::Quality(3));
    assert_eq!([r, a], [0.0, 1.0]);
    assert!((g - 0.374).abs() < 1e-5 && (b - 0.7395).abs() < 1e-5);
}

#[test]
fn modern_tooltip_keeps_the_white_border_and_retail_centre() {
    let tooltip = tooltip(
        TooltipBorder::Reaction(Reaction::Hostile),
        ActiveSkin::Modern,
    );
    assert_eq!(
        piece(&tooltip, "TooltipNineSliceCenter").1,
        [0.09, 0.09, 0.188, 1.0]
    );
    for part in TOOLTIP_BORDER_PIECES {
        let name = format!("TooltipNineSlice{part}");
        assert_eq!(piece(&tooltip, &name).1, [1.0; 4], "{part}");
    }
}

#[test]
fn forever_reskins_and_modern_restores_live_canvases() {
    let mut chat = canvas(chat_view(), chat_frame_screen, ActiveSkin::Modern);
    let mut tooltip = canvas(tooltip_view(), game_tooltip_screen, ActiveSkin::Modern);
    let center = |model: &RegistryModel| piece(model, "TooltipNineSliceCenter").1;
    assert!(has(&chat, CHAT_BACKGROUND.0) && !has(&chat, "ChatFrame1FlareSkin"));
    assert_eq!(center(&tooltip), [0.09, 0.09, 0.188, 1.0]);

    chat.sync_skin(ActiveSkin::Forever);
    tooltip.sync_skin(ActiveSkin::Forever);
    assert_eq!(slice(&chat, "ChatFrame1FlareSkin").border_color, BRONZE);
    assert_eq!(center(&tooltip), TOOLTIP_BG);
    assert_eq!(
        piece(&tooltip, "TooltipNineSliceTopEdge").1,
        TOOLTIP_DEFAULT
    );
    assert!(!has(&chat, CHAT_BACKGROUND.0));

    chat.sync_skin(ActiveSkin::Modern);
    tooltip.sync_skin(ActiveSkin::Modern);
    assert!(has(&chat, CHAT_BACKGROUND.0) && !has(&chat, "ChatFrame1FlareSkin"));
    assert_eq!(center(&tooltip), [0.09, 0.09, 0.188, 1.0]);
    assert_eq!(piece(&tooltip, "TooltipNineSliceTopEdge").1, [1.0; 4]);
}
