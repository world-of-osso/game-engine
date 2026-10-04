//! Options > Nameplates: every `NameplateStyle` value except the HUD page's Thin/Thick presets.
use super::*;
use crate::nameplate_style::{NameplateStyle, StyleColor, StyleSlider};

const CELL_GAP: f32 = 16.0;
const CELL_W: f32 = (OPTIONS_ROW_W - CELL_GAP) / 2.0;
const CELL_H: f32 = 36.0;
const CELL_LABEL_W: f32 = 124.0;
const SIZE_TRACK_W: f32 = 160.0;
const SWATCH: f32 = 18.0;
const CHANNEL_TRACK_W: f32 = 56.0;
const CHANNEL_GAP: f32 = 6.0;
const CHANNEL_FILLS: [&str; 3] = [
    "0.72,0.16,0.12,0.92",
    "0.20,0.58,0.16,0.92",
    "0.18,0.34,0.76,0.92",
];

pub(super) fn nameplates_body(hud: &HudOptionsView) -> OptionsPage {
    let style = &hud.nameplate_style;
    let sizes = StyleSlider::SIZES.map(|slider| size_cell(slider, style));
    let [health_w, health_h, cast_w, cast_h, name_font, cast_font] = sizes;
    let colors = StyleColor::ALL.map(|color| color_cell(color, style));
    let [hostile, neutral, friendly, cast, channel, uninterruptible] = colors;
    content_stack(
        [
            cell_row("NameplateHealthSize", health_w, health_h),
            cell_row("NameplateCastSize", cast_w, cast_h),
            cell_row("NameplateFonts", name_font, cast_font),
            cell_row(
                "NameplateToggles",
                toggle_cell("nameplate_show_border", "Border", style.show_border),
                toggle_cell(
                    "nameplate_class_colors",
                    "Class Colors",
                    style.class_colored_players,
                ),
            ),
            // `NameplateStyle.show_health_value`: "425 K  100%" instead of "100%" in the
            // Thick bar; off by default (user 2026-10-04).
            toggle_row(
                "nameplate_show_health_value",
                "Show Health Value",
                style.show_health_value,
            ),
            cell_row("NameplateReactionColors", hostile, neutral),
            cell_row("NameplateFriendlyCastColors", friendly, cast),
            cell_row("NameplateChannelColors", channel, uninterruptible),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub(super) fn cell_row(name: &str, left: Element, right: Element) -> Element {
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: {OPTIONS_ROW_W},
            height: {CELL_H},
            layout: "flex-row",
            gap: {CELL_GAP},
            {left}
            {right}
        }
    }
}

pub(super) fn cell(key: &str, label: &str, content: Element) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("NameplateCell{key}"))},
            width: {CELL_W},
            height: {CELL_H},
            {cell_label(key, label)}
            {content}
        }
    }
}

fn cell_label(key: &str, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("NameplateLabel{key}"))},
            width: {CELL_LABEL_W},
            height: 20.0,
            text: {text},
            font_size: 14.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: 0.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn size_cell(slider: StyleSlider, style: &NameplateStyle) -> Element {
    let value = slider.get(style);
    slider_cell(
        &slider.key(),
        slider.label(),
        (value, slider.bounds()),
        &format!("{value:.0}"),
    )
}

/// A labelled half-row slider with its value printed at the cell's right edge.
pub(super) fn slider_cell(
    key: &str,
    label: &str,
    (value, (min, max)): (f32, (f32, f32)),
    value_text: &str,
) -> Element {
    let track = compact_slider(
        key,
        value,
        min,
        max,
        SIZE_TRACK_W,
        CELL_LABEL_W,
        OPTIONS_TRACK_FILL,
    );
    let value_text = rsx! {
        fontstring {
            name: {DynName(format!("SliderValue{key}"))},
            width: 44.0,
            height: 20.0,
            text: {value_text},
            font_size: 14.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 4.0,
            top: "50%",
            translate_y: "-50%",
        }
    };
    cell(
        key,
        label,
        [track, value_text].into_iter().flatten().collect(),
    )
}

fn color_cell(color: StyleColor, style: &NameplateStyle) -> Element {
    let rgb = color.rgb(style);
    let key = StyleSlider::Channel(color, 0).key();
    let swatch_color = format!("{},{},{},1.0", rgb[0], rgb[1], rgb[2]);
    let swatch = rsx! {
        r#frame {
            name: {DynName(format!("NameplateSwatch{key}"))},
            width: {SWATCH},
            height: {SWATCH},
            background_color: {swatch_color},
            pos_type: "absolute",
            left: {CELL_LABEL_W},
            top: "50%",
            translate_y: "-50%",
        }
    };
    let channels = (0..3).flat_map(|channel| {
        let slider = StyleSlider::Channel(color, channel);
        let x =
            CELL_LABEL_W + SWATCH + CHANNEL_GAP + channel as f32 * (CHANNEL_TRACK_W + CHANNEL_GAP);
        compact_slider(
            &slider.key(),
            rgb[channel],
            0.0,
            1.0,
            CHANNEL_TRACK_W,
            x,
            CHANNEL_FILLS[channel],
        )
    });
    cell(
        &key,
        color.label(),
        swatch.into_iter().chain(channels).collect(),
    )
}

fn toggle_cell(key: &str, label: &str, enabled: bool) -> Element {
    cell(key, label, segmented_toggle(key, enabled))
}

fn compact_slider(
    key: &str,
    value: f32,
    min: f32,
    max: f32,
    width: f32,
    x: f32,
    fill: &str,
) -> Element {
    let action = slider_action(key);
    let name = format!("Slider{key}");
    let x = x.to_string();
    slider_widget(SliderWidget {
        name: &name,
        action: &action,
        value,
        min,
        max,
        width,
        interactive_height: 28.0,
        track_height: OPTIONS_TRACK_H,
        thumb_width: 14.0,
        thumb_height: 18.0,
        thumb_texture: None,
        track_color: OPTIONS_TRACK_BG,
        fill_color: fill,
        x: &x,
    })
}
