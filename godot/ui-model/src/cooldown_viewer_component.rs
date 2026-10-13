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
        _ => (30.0, 1, 5.0, 430.0, 420.0),
    };
    let bar = category == 3;
    let item_width = if bar { 220.0 } else { side };
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
    let timer = if icon.remaining > 0.0 {
        format!("{:.0}", icon.remaining.ceil())
    } else {
        String::new()
    };
    let charges = icon
        .charges
        .map(|count| count.to_string())
        .unwrap_or_default();
    let hide_glow = !icon.assisted_highlight;
    let progress_width = if bar {
        (size.0 - size.1) * icon.fraction
    } else {
        size.1
    };
    let progress_height = size.1;
    let label = if bar { icon.name.as_str() } else { "" };
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
            background_color: {if bar { "0.7,0.5,0.15,0.85" } else { "0.0,0.0,0.0,0.0" }},
            pos_type: "absolute", pos_x: {if bar { size.1 } else { 0.0 }},
            pos_y: {if bar { 0.0 } else { size.1 - progress_height }},
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
            justify_h: "CENTER", pos_type: "absolute", pos_x: {if bar { size.0 - size.1 } else { 0.0 }}, pos_y: 0.0,
        }
        fontstring {
            name: {DynName(format!("{name}Charges"))},
            width: {size.1}, height: 12.0, text: {charges.as_str()},
            font: GameFont::FrizQuadrata, font_size: 12.0, font_color: "1.0,1.0,1.0,1.0", outline: "OUTLINE",
            justify_h: "RIGHT", pos_type: "absolute", pos_x: 0.0, pos_y: {size.1 - 12.0},
        }
    }
}
