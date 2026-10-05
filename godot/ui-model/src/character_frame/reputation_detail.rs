//! Retail RF.xml:270-354, RF.lua:720-747; Forever Camelot RF.xml:251-286,
//! CharacterFrame.xml:317-388, RF.lua:735-745,756-792.

use ui_toolkit::atlas::{ActiveSkin, active_skin};
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use super::art::{WHITE, atlas, resolve_art, texture};
use super::reputation::{ReputationRow, reputation_bar};
use super::text;
use crate::minimal_scroll_bar::{
    BAR_W, MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
};
use crate::quest_art::{
    DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, line_height, wrapped_text_height,
};

pub const ACTION_REPUTATION_DETAIL_CLOSE: &str = "reputation_detail:close";
pub const REPUTATION_DESCRIPTION_SCROLL: &str = "ReputationDescriptionScrollBox";

/// ScrollTemplates.lua:144: a ScrollingFont's pan extent is its font height;
/// ScrollController.lua:152-154 doubles it for the wheel.
pub fn reputation_description_pan_extent() -> usize {
    line_height(12.0).ceil() as usize
}

/// RF.lua:284-288 toggles the clicked faction. Closing clears selection (750-753).
pub fn reputation_selection(
    action: &str,
    rows: &[ReputationRow],
    selected: Option<u32>,
) -> Option<u32> {
    if action == ACTION_REPUTATION_DETAIL_CLOSE {
        return None;
    }
    let Some(id) = action
        .strip_prefix("reputation_faction:")
        .and_then(|id| id.parse::<u32>().ok())
    else {
        return selected;
    };
    if rows.iter().any(|row| row.faction_id == id) {
        if selected == Some(id) { None } else { Some(id) }
    } else {
        selected
    }
}

pub(super) fn detail(row: Option<&ReputationRow>, ctx: &SharedContext) -> Element {
    let Some(row) = row else {
        return Element::default();
    };
    let forever = active_skin() == ActiveSkin::Forever;
    // Modern RF.xml:271-282; Forever CF.xml:318-329 and pane hosts :474-482.
    let (left, top, width, height, title_x, title_y, title_w, font) = if forever {
        (
            398.0 + 16.0,
            20.0 + 14.0,
            233.0 - 16.0 - 12.0,
            464.0 - 28.0,
            5.0,
            0.0,
            195.0,
            16.0,
        )
    } else {
        (400.0, 28.0, 212.0, 203.0, 20.0, 21.0, 160.0, 12.0)
    };
    let title_h = wrapped_text_height(&row.name, title_w, font);
    let standing_y = title_y + title_h + if forever { 3.0 } else { 2.0 };
    let mut children = Element::default();
    if !forever {
        // DialogTemplates.xml:64-79, the existing Dialog nine-slice border plus normal Bg.
        children.extend(texture(
            "ReputationDetailFrameBackground".into(),
            131_071,
            (7.0, 7.0, width - 14.0, height - 14.0),
            "0,1,0,1",
            WHITE,
        ));
        children.extend(texture(
            "ReputationDetailFrameArtwork".into(),
            136_565,
            (11.0, 11.0, 260.0, 128.0),
            "0,1,0,1",
            WHITE,
        ));
    }
    children.extend(text(
        "ReputationDetailFrameTitle".into(),
        &row.name,
        (title_x, title_y, title_w, line_height(font)),
        (font, NORMAL_FONT_COLOR),
        if forever { "CENTER" } else { "LEFT" },
    ));
    // The requested standing uses Camelot's Subtitle (CF.xml:330-335), within Retail's
    // scrolling description column; it is not an invented current at-war state.
    children.extend(text(
        "ReputationDetailFrameStanding".into(),
        row.standing,
        (title_x, standing_y, title_w, line_height(12.0)),
        (12.0, HIGHLIGHT_FONT_COLOR),
        if forever { "CENTER" } else { "LEFT" },
    ));
    let (description_x, description_y, description_w, description_h, war_x, war_y) = if forever {
        let divider = resolve_art("UI-Character-Info-ScrollLine");
        let divider_h = divider.rect.3 - divider.rect.2;
        let divider_w = divider.rect.1 - divider.rect.0;
        let divider_y = standing_y + line_height(12.0) + 4.0;
        children.extend(atlas(
            "ReputationDetailFrameDivider".into(),
            &divider,
            ((width - divider_w) / 2.0, divider_y, divider_w, divider_h),
            WHITE,
        ));
        let bar_y = divider_y + divider_h + 6.0;
        children.extend(reputation_bar(
            "ReputationDetailStanding",
            row,
            ((width - 180.0) / 2.0, bar_y, 180.0, 29.0),
        ));
        // CRF.lua:741-745: description 8 below StandingBar; height 200 (:1), footer 108 (:2).
        (
            0.0,
            bar_y + 29.0 + 8.0,
            width - 14.0,
            200.0,
            -4.0,
            height - 108.0,
        )
    } else {
        children.extend(texture(
            "ReputationDetailFrameDivider".into(),
            131_074,
            (9.0, 131.0, 256.0, 32.0),
            "0,1,0,1",
            WHITE,
        ));
        (
            20.0,
            standing_y + line_height(12.0) + 2.0,
            160.0,
            129.0 - standing_y - line_height(12.0) - 2.0,
            14.0,
            143.0,
        )
    };
    children.extend(description(
        row,
        ctx,
        (description_x, description_y, description_w, description_h),
    ));
    if row.allows_at_war {
        children.extend(at_war(forever, war_x, war_y));
    }

    if !forever {
        // RF.xml:321-324: UIPanelCloseButton at TOPRIGHT -2,-2.
        children.extend(rsx! {
            button {
                name: "ReputationDetailFrameCloseButton", width: 24.0, height: 24.0,
                onclick: ACTION_REPUTATION_DETAIL_CLOSE, pos_type: "absolute", right: 2.0, top: 2.0,
                texture { width: 24.0, height: 24.0, texture_atlas: "common-icon-redx", }
            }
        });
    }
    let mut element = rsx! {
        r#frame {
            name: "ReputationDetailFrame", width, height,
            frame_level: 100.0, mouse_enabled: true, pos_type: "absolute", left, top,
            {children}
        }
    };
    // An absent style is not an empty style name: Forever uses its existing side pane.
    if !forever {
        let ui_toolkit::widget_def::WidgetChild::Widget(frame) = &mut element[0] else {
            unreachable!("detail emits one frame");
        };
        frame.attrs.push(ui_toolkit::widget_def::Attr::new_static(
            "style",
            crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE.into(),
        ));
    }
    element
}

fn description(
    row: &ReputationRow,
    ctx: &SharedContext,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    let content_h = wrapped_text_height(&row.description, width, 12.0);
    let geometry = pixel_geometry(height, content_h, height);
    let offset = geometry.clamp(ctx.scroll_first_row(REPUTATION_DESCRIPTION_SCROLL));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: REPUTATION_DESCRIPTION_SCROLL,
        left: width + 5.0,
        top: 1.0,
        height: height - 2.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideBar,
    };
    let content = text(
        "ReputationDetailFrameDescription".into(),
        &row.description,
        (0.0, -(offset as f32), width, line_height(12.0)),
        (12.0, HIGHLIGHT_FONT_COLOR),
        "LEFT",
    );
    rsx! {
        r#frame {
            name: {DynName(REPUTATION_DESCRIPTION_SCROLL.into())}, width: {width + 5.0 + BAR_W}, height,
            scroll_list: config, mouse_enabled: true, pos_type: "absolute", left, top,
            {content}
            {bar.element()}
        }
    }
}

/// User requirement: omit ineligible war controls; keep eligible controls disabled until
/// shared-protocol reports atWarWith/canToggleAtWar and accepts a mutation.
fn at_war(forever: bool, left: f32, top: f32) -> Element {
    let mut art = if forever {
        atlas(
            "ReputationDetailFrameAtWarArt".into(),
            &resolve_art("checkbox-minimal"),
            (0.0, 0.0, 26.0, 26.0),
            "0.5,0.5,0.5,1",
        )
    } else {
        texture(
            "ReputationDetailFrameAtWarArt".into(),
            130_755,
            (0.0, 0.0, 26.0, 26.0),
            "0,1,0,1",
            "0.5,0.5,0.5,1",
        )
    };
    let font = if forever { 12.0 } else { 10.0 };
    let label_w = if forever {
        158.0
    } else {
        ui_toolkit::text_measure::measure_text(
            "At War",
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            font,
        )
        .expect("At War font measurement")
        .0
    };
    art.extend(text(
        "ReputationDetailFrameAtWarLabel".into(),
        "At War",
        (
            if forever { 28.0 } else { 24.0 },
            (26.0 - line_height(font)) / 2.0,
            label_w,
            line_height(font),
        ),
        (font, "0.5,0.5,0.5,1"),
        "LEFT",
    ));
    rsx! {
        button {
            name: "ReputationDetailFrameAtWarCheckbox", width: 26.0, height: 26.0,
            disabled: true, pos_type: "absolute", left, top,
            {art}
        }
    }
}
