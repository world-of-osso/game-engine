//! Campsite selector UI for the char select screen.
//! Grid panel with scene preview cards, opened from the top navigation CAMPSITES tab.
//!
//! Retail's GlueWarbandSceneJournal keeps a fixed-size frame and pages its card grid
//! (`PagedNaturalSizeGridContentFrameTemplate` + `PagingControlsHorizontalTemplate`:
//! page text, prev, next) instead of growing past the screen; this panel mirrors that.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::char_select_component::{CampsitePreview, CampsiteState, CharSelectAction};

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);

const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_WIDTH: f32 = 215.0;
const CARD_HEIGHT: f32 = 173.0;
const CARD_PREVIEW_WIDTH: f32 = 206.0;
const CARD_PREVIEW_HEIGHT: f32 = 165.0;
const CARD_LABEL_WIDTH: f32 = 192.0;
const CARD_LABEL_HEIGHT: f32 = 34.0;
const CARD_LABEL_TEXT_WIDTH: f32 = 176.0;
const CARD_LABEL_TEXT_HEIGHT: f32 = 28.0;
const PANEL_GAP: f32 = 10.0;
const PANEL_PADDING: f32 = 15.0;
const PANEL_WIDTH: f32 = 470.0;
pub const CAMPSITE_PANEL_WIDTH: f32 = PANEL_WIDTH;
pub const CAMPSITE_PANEL_TOP_OFFSET: f32 = 58.0;
/// Two rows of two cards: the panel ends well above the bottom of a 720px window.
pub const CAMPSITES_PER_PAGE: usize = 4;
const PAGE_ROWS: usize = CAMPSITES_PER_PAGE / 2;
const PAGING_HEIGHT: f32 = 32.0;
const PAGE_BUTTON_SIZE: f32 = 32.0;
const PAGE_TEXT_WIDTH: f32 = 100.0;
const PAGING_SPACING: f32 = 5.0;
// interface/buttons/ui-spellbookicon-{prev,next}page-{up,disabled}.blp
const TEX_PREV_PAGE_UP: &str = "data/textures/130869.blp";
const TEX_PREV_PAGE_DISABLED: &str = "data/textures/130867.blp";
const TEX_NEXT_PAGE_UP: &str = "data/textures/130866.blp";
const TEX_NEXT_PAGE_DISABLED: &str = "data/textures/130864.blp";

struct DynName(String);

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

fn card_backdrop(id: u32, preview_image: Option<CampsitePreview>) -> Element {
    if let Some(preview) = preview_image {
        let [left, right, top, bottom] = preview.tex_coords;
        let tex_coords = format!("{left},{right},{top},{bottom}");
        rsx! {
            texture {
                name: dyn_name(format!("CampsiteCard_{id}")),
                width: CARD_PREVIEW_WIDTH,
                height: CARD_PREVIEW_HEIGHT,
                texture_fdid: {preview.fdid},
                tex_coords: {tex_coords.as_str()},
                pos_type: "absolute",
                left: "50%",
                translate_x: "-50%",
                top: 4.0,
            }
        }
    } else {
        rsx! {
            texture {
                name: dyn_name(format!("CampsiteCard_{id}")),
                width: CARD_PREVIEW_WIDTH,
                height: 80.0,
                texture_atlas: CARD_BACKDROP_ATLAS,
                pos_type: "absolute",
                left: "50%",
                translate_x: "-50%",
                top: "50%",
                translate_y: "-50%",
            }
        }
    }
}

fn card_label_bar(id: u32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(format!("CampsiteLabelBar_{id}")),
            width: CARD_LABEL_WIDTH,
            height: CARD_LABEL_HEIGHT,
            background_color: "0.05,0.04,0.03,0.88",
            border: "1px solid 0.36,0.28,0.08,0.75",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 4.0,
        }
    }
}

fn card_label(id: u32, name: &str, is_selected: bool) -> Element {
    let color = if is_selected {
        COLOR_GOLD
    } else {
        COLOR_SUBTITLE
    };
    rsx! {
        fontstring {
            name: dyn_name(format!("CampsiteLabel_{id}")),
            width: CARD_LABEL_TEXT_WIDTH,
            height: CARD_LABEL_TEXT_HEIGHT,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: color,
            justify_h: JustifyH::Center,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 6.0,
        }
    }
}

fn campsite_card(
    id: u32,
    name: &str,
    preview_image: Option<CampsitePreview>,
    is_selected: bool,
) -> Element {
    let border = if is_selected {
        "2px solid 0.95,0.78,0.14,0.95"
    } else {
        "1px solid 0.30,0.24,0.09,0.70"
    };
    let background = if is_selected {
        "0.13,0.10,0.03,0.96"
    } else {
        "0.03,0.03,0.02,0.94"
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("CampsiteScene_{id}")),
            width: CARD_WIDTH,
            height: CARD_HEIGHT,
            background_color: background,
            border,
            onclick: CharSelectAction::SelectCampsite(id),
            {card_backdrop(id, preview_image)}
            {card_label_bar(id)}
            {card_label(id, name, is_selected)}
        }
    }
}

pub fn campsite_panel(state: &CampsiteState) -> Element {
    campsite_panel_at_top(state, CAMPSITE_PANEL_TOP_OFFSET, PANEL_WIDTH)
}

pub fn campsite_page_count(scene_count: usize) -> usize {
    scene_count.div_ceil(CAMPSITES_PER_PAGE).max(1)
}

/// The requested page, clamped to the pages the current scene list has.
fn visible_page(state: &CampsiteState) -> usize {
    state.page.min(campsite_page_count(state.scenes.len()) - 1)
}

fn build_campsite_cards(state: &CampsiteState) -> Element {
    state
        .scenes
        .iter()
        .skip(visible_page(state) * CAMPSITES_PER_PAGE)
        .take(CAMPSITES_PER_PAGE)
        .flat_map(|e| {
            campsite_card(
                e.id,
                &e.name,
                e.preview_image,
                state.selected_id == Some(e.id),
            )
        })
        .collect()
}

/// A disabled button (retail `SetEnabled(false)`) shows its disabled art and targets the
/// current page, so a click is a no-op.
fn page_button(name: &str, texture: &str, target: usize, left: f32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(name.to_owned()),
            width: PAGE_BUTTON_SIZE,
            height: PAGE_BUTTON_SIZE,
            onclick: CharSelectAction::CampsitePage(target),
            pos_type: "absolute",
            left,
            top: 0.0,
            texture {
                name: dyn_name(format!("{name}Icon")),
                width: PAGE_BUTTON_SIZE,
                height: PAGE_BUTTON_SIZE,
                texture_file: texture,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        }
    }
}

/// Retail horizontal paging controls: "Page N/M", then prev and next, centred under the
/// grid. Kept outside the grid so card rebuilds cannot reorder it above the cards.
fn paging_controls(state: &CampsiteState) -> Element {
    let page = visible_page(state);
    let pages = campsite_page_count(state.scenes.len());
    let row_width = PANEL_WIDTH - PANEL_PADDING * 2.0;
    let group_width = PAGE_TEXT_WIDTH + (PAGING_SPACING + PAGE_BUTTON_SIZE) * 2.0;
    let text_left = (row_width - group_width) / 2.0;
    let prev_left = text_left + PAGE_TEXT_WIDTH + PAGING_SPACING;
    let next_left = prev_left + PAGE_BUTTON_SIZE + PAGING_SPACING;
    let (prev_texture, prev_target) = if page > 0 {
        (TEX_PREV_PAGE_UP, page - 1)
    } else {
        (TEX_PREV_PAGE_DISABLED, page)
    };
    let (next_texture, next_target) = if page + 1 < pages {
        (TEX_NEXT_PAGE_UP, page + 1)
    } else {
        (TEX_NEXT_PAGE_DISABLED, page)
    };
    let text = format!("Page {}/{}", page + 1, pages);
    rsx! {
        r#frame {
            name: "CampsitePaging",
            width: row_width,
            height: PAGING_HEIGHT,
            pos_type: "absolute",
            left: PANEL_PADDING,
            bottom: PANEL_PADDING,
            fontstring {
                name: "CampsitePageText",
                width: PAGE_TEXT_WIDTH,
                height: PAGING_HEIGHT,
                text,
                font: GameFont::FrizQuadrata,
                font_size: 13.0,
                font_color: COLOR_SUBTITLE,
                justify_h: JustifyH::Right,
                pos_type: "absolute",
                left: text_left,
                top: 0.0,
            }
            {page_button("CampsitePrevPage", prev_texture, prev_target, prev_left)}
            {page_button("CampsiteNextPage", next_texture, next_target, next_left)}
        }
    }
}

fn campsite_panel_at_top(state: &CampsiteState, top: f32, width: f32) -> Element {
    let hide = !state.panel_visible;
    let cards = build_campsite_cards(state);
    let paging = paging_controls(state);
    let height = campsite_panel_height(state.scenes.len());
    let on_page =
        (state.scenes.len() - visible_page(state) * CAMPSITES_PER_PAGE).min(CAMPSITES_PER_PAGE);
    // The grid is exactly as tall as this page's rows: wrapped flex lines would otherwise
    // spread across leftover height and slide under the paging controls.
    let grid_height = card_rows_height(on_page.div_ceil(2).max(1)) + PANEL_PADDING * 2.0;
    rsx! {
        r#frame {
            name: "CampsitePanel",
            width,
            height,
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: "0.04,0.03,0.02,0.98",
            border: "1px solid 0.62,0.46,0.10,0.75",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top,
            r#frame {
                name: "CampsiteGrid",
                width,
                height: grid_height,
                layout: "flex-row-wrap",
                justify: "center",
                gap: PANEL_GAP,
                padding: PANEL_PADDING,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
                {cards}
            }
            {paging}
        }
    }
}

fn card_rows_height(rows: usize) -> f32 {
    let rows = rows as f32;
    rows * CARD_HEIGHT + (rows - 1.0) * PANEL_GAP
}

/// Fixed page height (at most `PAGE_ROWS` card rows) plus the paging row.
pub fn campsite_panel_height(scene_count: usize) -> f32 {
    let rows = scene_count.div_ceil(2).clamp(1, PAGE_ROWS);
    card_rows_height(rows) + PANEL_GAP + PAGING_HEIGHT + PANEL_PADDING * 2.0
}
