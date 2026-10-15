//! Retail four-viewer inventory and default anchors (CooldownViewer.xml and
//! EditModePresetLayouts.lua). Presentation consumes the current projected samples.

use crate::cooldown_assist::{CooldownIconView, CooldownViewerView};
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

struct DynName(String);

pub const VIEWER_NAMES: [&str; 4] = [
    "EssentialCooldownViewer",
    "UtilityCooldownViewer",
    "BuffIconCooldownViewer",
    "BuffBarCooldownViewer",
];

// CooldownViewerBuffBarItemTemplate, CooldownViewer.xml:209-277.
const BUFF_ICON_SIZE: f32 = 30.0;
const BUFF_ITEM_WIDTH: f32 = 220.0;
const BUFF_BAR_HEIGHT: f32 = 19.0;
const BUFF_BAR_LEFT: f32 = BUFF_ICON_SIZE + 2.0;
const BUFF_BAR_WIDTH: f32 = BUFF_ITEM_WIDTH - BUFF_BAR_LEFT;

pub fn cooldown_viewer_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let state = ctx
        .get::<CooldownViewerView>()
        .expect("CooldownViewerView in SharedContext");
    let mut rows: Element = state
        .categories
        .iter()
        .enumerate()
        .flat_map(|(category, icons)| viewer(category, icons, state.enabled))
        .collect();
    rows.extend(crate::cooldown_settings_component::settings_panel(
        ctx,
        &state.settings,
    ));
    rows
}

fn viewer(category: usize, icons: &[CooldownIconView], enabled: bool) -> Element {
    let hidden = !enabled;
    // XML item dimensions; Modern authored IconLimit and padding.
    let (side, columns, padding, bottom, offset_x) = match category {
        0 => (50.0, 12, 2.0, 310.0, 0.0),
        1 => (30.0, 7, 2.0, 240.0, 0.0),
        2 => (40.0, 1, 5.0, 370.0, 0.0),
        _ => (BUFF_ICON_SIZE, 1, 5.0, 430.0, 420.0),
    };
    let bar = category == 3;
    let item_width = if bar { BUFF_ITEM_WIDTH } else { side };
    let width = icons.len().min(columns).max(1) as f32 * (item_width + padding) - padding;
    let height = icons.len().div_ceil(columns).max(1) as f32 * (side + padding) - padding;
    let bottom = bottom.to_string();
    let items: Element = icons
        .iter()
        .enumerate()
        .flat_map(|(index, icon)| {
            let origin = (
                (index % columns) as f32 * (item_width + padding),
                (index / columns) as f32 * (side + padding),
            );
            viewer_item(icon, origin, (item_width, side), bar)
        })
        .collect();
    rsx! {
        r#frame {
            name: {FrameName(VIEWER_NAMES[category])},
            width, height, hidden,
            strata: FrameStrata::Medium,
            pos_type: "absolute",
            left: "50%",
            bottom: {bottom.as_str()},
            margin_left: {-width * 0.5 + offset_x},
            {items}
        }
    }
}

fn viewer_item(
    icon: &CooldownIconView,
    origin: (f32, f32),
    size: (f32, f32),
    bar: bool,
) -> Element {
    let name = format!("CooldownViewer{}", icon.cooldown_id);
    let content = viewer_item_content(&name, icon, size, bar);
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: {size.0}, height: {size.1},
            pos_type: "absolute", pos_x: {origin.0}, pos_y: {origin.1},
            {content}
        }
    }
}

fn viewer_item_content(
    name: &str,
    icon: &CooldownIconView,
    size: (f32, f32),
    bar: bool,
) -> Element {
    if bar {
        return buff_bar_content(name, icon);
    }
    let timer = remaining_text(icon.remaining);
    let charges = icon
        .charges
        .map(|count| count.to_string())
        .unwrap_or_default();
    let hide_glow = !icon.assisted_highlight;
    let progress_width = size.1;
    let progress_height = size.1;
    let label = "";
    rsx! {
        texture {
            name: {DynName(format!("{name}Icon"))},
            width: {size.1}, height: {size.1},
            texture_fdid: {icon.icon_fdid},
            draw_layer: "BACKGROUND", pos_type: "absolute", pos_x: 0.0, pos_y: 0.0,
        }
        r#frame {
            name: {DynName(format!("{name}Cooldown"))},
            width: progress_width, height: progress_height,
            background_color: "0.0,0.0,0.0,0.0",
            pos_type: "absolute", pos_x: 0.0,
            pos_y: {size.1 - progress_height},
        }
        {crate::assisted_highlight::highlight(format!("{name}AssistedHighlight"), size.1, hide_glow)}
        fontstring {
            name: {DynName(format!("{name}Name"))},
            width: {size.0 - size.1}, height: {size.1}, text: label,
            font: GameFont::FrizQuadrata, font_size: 12.0, font_color: "1.0,1.0,1.0,1.0",
            pos_type: "absolute", pos_x: {size.1 + 4.0}, pos_y: 0.0,
        }
        fontstring {
            name: {DynName(format!("{name}Timer"))},
            width: {size.1}, height: {size.1}, text: {timer.as_str()},
            font: GameFont::FrizQuadrata, font_size: 14.0, font_color: "1.0,1.0,1.0,1.0", outline: "OUTLINE",
            justify_h: "CENTER", pos_type: "absolute", pos_x: 0.0, pos_y: 0.0,
        }
        fontstring {
            name: {DynName(format!("{name}Charges"))},
            width: {size.1}, height: 12.0, text: {charges.as_str()},
            font: GameFont::FrizQuadrata, font_size: 12.0, font_color: "1.0,1.0,1.0,1.0", outline: "OUTLINE",
            justify_h: "RIGHT", pos_type: "absolute", pos_x: 0.0, pos_y: {size.1 - 12.0},
        }
    }
}

fn remaining_text(remaining: f32) -> String {
    if remaining > 0.0 {
        format!("{:.0}", remaining.ceil())
    } else {
        String::new()
    }
}

fn buff_bar_content(name: &str, icon: &CooldownIconView) -> Element {
    let image = buff_bar_icon(name, icon);
    let art = buff_bar_art(name, icon);
    let labels = buff_bar_labels(name, icon);
    rsx! {
        {image}
        r#frame { name: {DynName(format!("{name}Cooldown"))},
            width: BUFF_BAR_WIDTH, height: BUFF_BAR_HEIGHT, frame_level: 511.0,
            pos_type: "absolute", left: BUFF_BAR_LEFT, top: {(BUFF_ICON_SIZE - BUFF_BAR_HEIGHT) / 2.0},
            {art} {labels} }
    }
}

fn buff_bar_icon(name: &str, icon: &CooldownIconView) -> Element {
    let hide_glow = !icon.assisted_highlight;
    let charges = icon
        .charges
        .map(|count| count.to_string())
        .unwrap_or_default();
    rsx! {
        r#frame { name: {DynName(format!("{name}IconFrame"))},
            width: BUFF_ICON_SIZE, height: BUFF_ICON_SIZE, frame_level: 512.0,
            pos_type: "absolute", left: 0.0, top: 0.0,
            texture { name: {DynName(format!("{name}Icon"))},
                width: BUFF_ICON_SIZE, height: BUFF_ICON_SIZE, texture_fdid: {icon.icon_fdid},
                pos_type: "absolute", left: 0.0, top: 0.0, draw_layer: "BACKGROUND" }
            {crate::assisted_highlight::highlight(format!("{name}AssistedHighlight"), BUFF_ICON_SIZE, hide_glow)}
            fontstring { name: {DynName(format!("{name}Charges"))},
                width: 32.0, height: 10.0, text: {charges.as_str()},
                font: GameFont::FrizQuadrata, font_size: 12.0, font_color: "1,1,1,1", outline: "OUTLINE",
                justify_h: "RIGHT", pos_type: "absolute", right: 5.0, bottom: 5.0 }
        }
    }
}

fn buff_bar_art(name: &str, icon: &CooldownIconView) -> Element {
    let pip = ui_toolkit::atlas::resolve_region(
        "UI-HUD-CoolDownManager-Bar-Pip",
        ui_toolkit::atlas::thread_skin(),
    )
    .expect("CooldownViewer buff pip atlas");
    let width = BUFF_BAR_WIDTH * icon.fraction;
    let coords = format!("0,{},0,1", icon.fraction);
    let hidden = icon.remaining <= 0.0;
    rsx! {
        texture { name: {DynName(format!("{name}BarBackground"))},
            texture_atlas: "UI-HUD-CoolDownManager-Bar-BG", width: {BUFF_BAR_WIDTH + 6.0},
            height: {BUFF_BAR_HEIGHT + 9.0}, pos_type: "absolute", left: -2.0, top: -2.0,
            draw_layer: "BACKGROUND" }
        texture { name: {DynName(format!("{name}BarFill"))},
            texture_atlas: "UI-HUD-CoolDownManager-Bar", width, height: BUFF_BAR_HEIGHT,
            vertex_color: "1,0.5,0.25,1", tex_coords: {coords.as_str()}, hidden,
            pos_type: "absolute", left: 0.0, top: 0.0 }
        texture { name: {DynName(format!("{name}BarPip"))},
            texture_atlas: "UI-HUD-CoolDownManager-Bar-Pip", width: pip.width, height: pip.height,
            hidden, pos_type: "absolute", left: {width - pip.width / 2.0},
            top: {(BUFF_BAR_HEIGHT - pip.height) / 2.0 + 1.0}, draw_layer: "OVERLAY" }
    }
}

fn buff_bar_labels(name: &str, icon: &CooldownIconView) -> Element {
    let timer = remaining_text(icon.remaining);
    rsx! {
        fontstring { name: {DynName(format!("{name}Name"))},
            width: {BUFF_BAR_WIDTH - 30.0}, height: BUFF_BAR_HEIGHT, text: {icon.name.as_str()},
            font: GameFont::FrizQuadrata, font_size: 12.0, font_color: "1,1,1,1", justify_h: "LEFT",
            pos_type: "absolute", left: 5.0, top: 0.0, draw_layer: "OVERLAY" }
        fontstring { name: {DynName(format!("{name}Timer"))},
            width: "auto", height: BUFF_BAR_HEIGHT, text: {timer.as_str()},
            font: GameFont::FrizQuadrata, font_size: 14.0, font_color: "1,1,1,1", outline: "OUTLINE",
            justify_h: "LEFT", pos_type: "absolute", right: 8.0,
            top: 0.0, draw_layer: "OVERLAY" }
    }
}
