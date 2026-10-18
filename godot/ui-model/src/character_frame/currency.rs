//! Bounded PvP TokenFrame: Retail Blizzard_TokenUI.xml/.lua and ListTemplates.xml.
//! Amounts come only from CurrencyStateUpdate. Source70391 metadata covers1792/1602;
//! transfer, watched/unused preferences and account-wide filters are not fabricated.
use super::{FRAME_H, text};
use crate::bank_art::{WHITE, label};
use crate::damage_meter_data::break_up_large_number;
use crate::game_tooltip::GameTooltip;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture};
use crate::tooltip_presentation::{TooltipLineState, TooltipPresentation};
use shared::protocol::CurrencyEntrySnapshot;
use ui_toolkit::{rsx, widget_def::Element};

pub const TOKEN_SCROLL: &str = "TokenFrameScrollBox";
pub const ACTION_CURRENCY_COLLAPSE: &str = "currency:collapse";
pub const WIDTH: f32 = 400.0;
const ENTRY_HEIGHT: f32 = 22.0;
const HEADER_HEIGHT: f32 = 26.0;
const SPACING: f32 = 2.0;

#[derive(Clone, Debug, PartialEq)]
pub struct CurrencyRow {
    pub id: u32,
    pub name: String,
    pub quantity: u64,
    pub icon: u32,
    pub description: &'static str,
    pub maximum: Option<u64>,
}

/// CurrencyTypes70391 rows1792/1602; Conquest31270 is a dynamic cap, not static zero.
pub fn currency_rows(entries: &[CurrencyEntrySnapshot]) -> Result<Vec<CurrencyRow>, String> {
    entries
        .iter()
        .map(|entry| {
            let (icon, description, maximum) = match entry.id {
                1792 => (
                    1455894,
                    "Used to purchase Unrated PvP equipment.",
                    Some(15000),
                ),
                1602 => (
                    1523630,
                    "Earned from Rated PvP activities. Used to purchase Rated PvP equipment.",
                    None,
                ),
                id => {
                    return Err(format!(
                        "Currency{id} has no native source metadata in the PvP wallet slice"
                    ));
                }
            };
            Ok(CurrencyRow {
                id: entry.id,
                name: entry.name.clone(),
                quantity: entry.amount,
                icon,
                description,
                maximum,
            })
        })
        .collect()
}

pub fn currency_tooltip(row: &CurrencyRow) -> GameTooltip {
    let total = break_up_large_number(row.quantity);
    let count = match row.maximum {
        Some(maximum) => format!("Total Maximum: {total}/{}", break_up_large_number(maximum)),
        None => format!("Total: {total}"),
    };
    GameTooltip::new(
        TooltipPresentation {
            title: row.name.clone(),
            lines: vec![
                TooltipLineState::new(row.description),
                TooltipLineState::new(count),
            ],
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

/// ScrollBoxListView uses the first element's extent: this list starts with a header.
pub fn token_pan_extent() -> usize {
    (HEADER_HEIGHT + SPACING) as usize
}

pub(super) fn backgrounds() -> Element {
    named_atlas_texture(
        "CharacterFrameBackground".into(),
        "character-panel-background",
        (4.0, 60.0, WIDTH - 10.0, FRAME_H - 64.0),
    )
}

pub(super) fn entries(rows: &[CurrencyRow], collapsed: bool, offset: usize) -> Element {
    // TokenFrame.ScrollBox: CharacterFrame.Inset +4,-4 / -22,+2.
    let (width, height) = (WIDTH - 36.0, FRAME_H - 70.0);
    let row_count = if collapsed { 0 } else { rows.len() };
    let content_height = 20.0 + HEADER_HEIGHT + row_count as f32 * (ENTRY_HEIGHT + SPACING);
    let geometry = pixel_geometry(height, content_height, height - 6.0);
    let offset = geometry.clamp(offset);
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: TOKEN_SCROLL,
        left: 8.0 + width + 5.0,
        top: 66.0,
        height: height - 6.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let mut children = if rows.is_empty() {
        vec![]
    } else {
        header(width - 20.0, collapsed, 10.0 - offset as f32)
    };
    if !collapsed {
        for (index, row) in rows.iter().enumerate() {
            let top = 10.0 + HEADER_HEIGHT + SPACING + index as f32 * (ENTRY_HEIGHT + SPACING)
                - offset as f32;
            children.extend(entry(row, (12.0, top, width - 22.0)));
        }
    }
    rsx! { r#frame { name: "TokenFrame", width: WIDTH, height: FRAME_H, pos_type: "absolute", left: 0.0, top: 0.0,
        r#frame { name: {DynName(TOKEN_SCROLL.into())}, width, height, left: 8.0, top: 64.0,
            pos_type: "absolute", mouse_enabled: true, scroll_list: config,
            r#frame { name: "TokenFrameScrollChild", width, height: content_height,
                pos_type: "absolute", {children} }
        }
        {bar.element()}
    } }
}

fn header(width: f32, collapsed: bool, top: f32) -> Element {
    let right = if collapsed {
        "Options_ListExpand_Right"
    } else {
        "Options_ListExpand_Right_Expanded"
    };
    let atlas = |name| {
        ui_toolkit::atlas::resolve_region(name, ui_toolkit::atlas::thread_skin())
            .expect("source token header atlas")
    };
    let sides = (atlas("Options_ListExpand_Left").width, atlas(right).width);
    let mut children = three_slice(
        "TokenHeader2",
        (
            "Options_ListExpand_Left",
            "_Options_ListExpand_Middle",
            right,
        ),
        (width, HEADER_HEIGHT),
        sides,
        ("BACKGROUND", 1.0, false),
    );
    children.extend(three_slice(
        "TokenHeader2Highlight",
        (
            "Options_ListExpand_Left",
            "_Options_ListExpand_Middle",
            right,
        ),
        (width, HEADER_HEIGHT),
        sides,
        ("HIGHLIGHT", 0.4, false),
    ));
    children.extend(label(
        "TokenHeader2Name".into(),
        "Player vs. Player",
        (10.0, 5.5, width - 40.0, 15.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    crate::selection_list::row(
        "TokenHeader2".into(),
        ACTION_CURRENCY_COLLAPSE,
        true,
        (10.0, top, width, HEADER_HEIGHT),
        (&crate::selection_list::AUTHORED, false),
        children,
    )
}

fn entry(row: &CurrencyRow, (left, top, width): (f32, f32, f32)) -> Element {
    let name = format!("TokenEntry{}", row.id);
    let count = break_up_large_number(row.quantity);
    let count_width = ui_toolkit::text_measure::measure_text(
        &count,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .expect("TokenEntry count font")
    .0;
    let color = if row.quantity > 0 {
        WHITE
    } else {
        "0.5,0.5,0.5,1"
    };
    let mut children = three_slice(
        &format!("{name}Highlight"),
        (
            "charactercreate-customize-dropdown-linemouseover-side",
            "charactercreate-customize-dropdown-linemouseover-middle",
            "charactercreate-customize-dropdown-linemouseover-side",
        ),
        (width, ENTRY_HEIGHT),
        (6.0, 6.0),
        ("HIGHLIGHT", 0.1, true),
    );
    children.extend(crate::bank_art::texture(
        format!("{name}CurrencyIcon"),
        row.icon,
        (width - 40.0, 1.0, 20.0, 20.0),
        WHITE,
    ));
    children.extend(text(
        format!("{name}Count"),
        &count,
        (width - 45.0 - count_width, 5.5, count_width, 11.0),
        (12.0, color),
        "RIGHT",
    ));
    children.extend(text(
        format!("{name}Name"),
        &row.name,
        (25.0, 5.5, width - 80.0 - count_width, 11.0),
        (12.0, color),
        "LEFT",
    ));
    crate::selection_list::row(
        name,
        "",
        true,
        (left, top, width, ENTRY_HEIGHT),
        (&crate::selection_list::AUTHORED, false),
        children,
    )
}

fn three_slice(
    name: &str,
    atlases: (&str, &str, &str),
    size: (f32, f32),
    sides: (f32, f32),
    style: (&str, f32, bool),
) -> Element {
    use ui_toolkit::atlas::{AtlasSource, resolve_region, thread_skin};
    let (width, height) = size;
    let (left_span, right_span) = sides;
    let (layer, alpha, mirror_right) = style;
    [("Left", atlases.0, 0.0, left_span),
        ("Middle", atlases.1, left_span, width - left_span - right_span),
        ("Right", atlases.2, width - right_span, right_span)]
    .into_iter().flat_map(|(part, atlas, left, width)| {
        let region = resolve_region(atlas, thread_skin()).expect("source token atlas");
        let AtlasSource::FileDataId(fdid) = region.source else { panic!("TokenFrame atlas must be a DB2 file") };
        let (u0, u1) = if part == "Right" && mirror_right { (region.right, region.left) } else { (region.left, region.right) };
        let coords = format!("{u0},{u1},{},{}", region.top, region.bottom);
        rsx! { texture { name: {DynName(format!("{name}{part}"))}, texture_fdid: fdid, width, height,
            left, top: 0.0, pos_type: "absolute", alpha, draw_layer: layer, tex_coords: {coords.as_str()} } }
    }).collect()
}
