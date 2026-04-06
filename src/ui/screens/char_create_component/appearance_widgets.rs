use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{GameFont, JustifyH};

use super::{
    AppearanceField, COLOR_SELECTED, COLOR_SUBTITLE, COLOR_WHITE, CharCreateAction, DynName,
};

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

const STEPPER_SIZE: f32 = 38.0;
const DEC_RIGHT_INSET: f32 = 130.0;
const INC_RIGHT_INSET: f32 = 10.0;
const SWATCH_PREVIEW_WIDTH: f32 = 84.0;
const SWATCH_PREVIEW_HEIGHT: f32 = 20.0;
const SWATCH_PREVIEW_AREA_HEIGHT: f32 = 40.0;
const SWATCH_DROPDOWN_WIDTH: f32 = 40.0;
const SWATCH_DROPDOWN_HEIGHT: f32 = 20.0;
const SWATCH_DROPDOWN_CHOICE_WIDTH: f32 = 44.0;
const SWATCH_DROPDOWN_CHOICE_HEIGHT: f32 = 28.0;
const SWATCH_DROPDOWN_SELECTION_WIDTH: f32 = 48.0;
const SWATCH_DROPDOWN_SELECTION_HEIGHT: f32 = 28.0;
const NUMBER_DROPDOWN_CHOICE_WIDTH: f32 = 28.0;
const NUMBER_DROPDOWN_CHOICE_HEIGHT: f32 = 22.0;
const DROPDOWN_WIDTH: f32 = 282.0;
const DROPDOWN_GAP: f32 = 2.0;
const DROPDOWN_PADDING: f32 = 4.0;
const SELECTION_DROPDOWN_OFFSET_X: f32 = -4.0;

fn right_inset_x(inset: f32) -> String {
    format!("-{inset}")
}

fn x_offset(x: f32) -> String {
    format!("{x}")
}

fn swatch_gap_center_x() -> String {
    let inset = (DEC_RIGHT_INSET + INC_RIGHT_INSET + STEPPER_SIZE) * 0.5;
    right_inset_x(inset)
}

fn stepper_dec_button(field: AppearanceField) -> Element {
    let x = right_inset_x(DEC_RIGHT_INSET);
    rsx! {
        button {
            name: dyn_name(format!("AppDec_{}", field.as_str())),
            width: STEPPER_SIZE,
            height: STEPPER_SIZE,
            text: "",
            onclick: CharCreateAction::AppearanceDec(field),
            button_atlas_up: "charactercreate-customize-backbutton",
            button_atlas_pressed: "charactercreate-customize-backbutton-down",
            button_atlas_highlight: "charactercreate-customize-backbutton",
            button_atlas_disabled: "charactercreate-customize-backbutton-disabled",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x,
            }
        }
    }
}

fn stepper_inc_button(field: AppearanceField) -> Element {
    let x = right_inset_x(INC_RIGHT_INSET);
    rsx! {
        button {
            name: dyn_name(format!("AppInc_{}", field.as_str())),
            width: STEPPER_SIZE,
            height: STEPPER_SIZE,
            text: "",
            onclick: CharCreateAction::AppearanceInc(field),
            button_atlas_up: "charactercreate-customize-nextbutton",
            button_atlas_pressed: "charactercreate-customize-nextbutton-down",
            button_atlas_highlight: "charactercreate-customize-nextbutton",
            button_atlas_disabled: "charactercreate-customize-nextbutton-disabled",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x,
            }
        }
    }
}

fn appearance_row_label(field: AppearanceField, label: &str) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("AppLabel_{}", field.as_str())),
            width: 120.0,
            height: 24.0,
            text: label,
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
                x: "10",
            }
        }
    }
}

fn appearance_row_value(field: AppearanceField, value: u8, label: &str) -> Element {
    let val_text = if label.is_empty() {
        format!("{}", value + 1)
    } else {
        label.to_string()
    };
    let x = swatch_gap_center_x();
    rsx! {
        fontstring {
            name: dyn_name(format!("AppVal_{}", field.as_str())),
            width: SWATCH_PREVIEW_WIDTH,
            height: 24.0,
            text: val_text,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: COLOR_WHITE,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Right,
                x,
            }
        }
    }
}

fn rgb_to_vertex_color(color: [u8; 3]) -> String {
    format!(
        "{},{},{},1.0",
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0
    )
}

fn swatch_color_preview(field: AppearanceField, vc: &str) -> Element {
    rsx! {
        texture {
            name: dyn_name(format!("AppSwatch_{}", field.as_str())),
            width: SWATCH_PREVIEW_WIDTH,
            height: SWATCH_PREVIEW_HEIGHT,
            texture_atlas: "charactercreate-customize-palette",
            vertex_color: vc,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn swatch_texture(field: AppearanceField, color: [u8; 3]) -> Element {
    let (vc, x) = (rgb_to_vertex_color(color), swatch_gap_center_x());
    rsx! {
        r#frame {
            name: dyn_name(format!("AppSwatchArea_{}", field.as_str())),
            width: SWATCH_PREVIEW_WIDTH,
            height: SWATCH_PREVIEW_AREA_HEIGHT,
            onclick: CharCreateAction::ToggleDropdown(field),
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Right,
                x,
            }
            {swatch_color_preview(field, &vc)}
        }
    }
}

fn dropdown_color_choice(
    field: AppearanceField,
    i: usize,
    color: [u8; 3],
    selected: bool,
) -> Element {
    let vc = rgb_to_vertex_color(color);
    let sel_hidden = !selected;
    let idx = i as u8;
    let selection_x = x_offset(SELECTION_DROPDOWN_OFFSET_X);
    rsx! {
        r#frame {
            name: dyn_name(format!("DropChoice_{}_{i}", field.as_str())),
            width: SWATCH_DROPDOWN_CHOICE_WIDTH,
            height: SWATCH_DROPDOWN_CHOICE_HEIGHT,
            onclick: CharCreateAction::SelectChoice(field, idx),
            texture {
                name: dyn_name(format!("DropSwatch_{}_{i}", field.as_str())),
                width: SWATCH_DROPDOWN_WIDTH,
                height: SWATCH_DROPDOWN_HEIGHT,
                texture_atlas: "charactercreate-customize-palette",
                vertex_color: vc,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
            }
            texture {
                name: dyn_name(format!("DropSel_{}_{i}", field.as_str())),
                width: SWATCH_DROPDOWN_SELECTION_WIDTH,
                height: SWATCH_DROPDOWN_SELECTION_HEIGHT,
                hidden: sel_hidden,
                texture_atlas: "charactercreate-customize-palette-selected",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                    x: selection_x,
                }
            }
        }
    }
}

fn dropdown_number_choice(field: AppearanceField, i: usize, selected: bool) -> Element {
    let idx = i as u8;
    let val_text = format!("{}", idx + 1);
    let color = if selected {
        COLOR_SELECTED
    } else {
        COLOR_WHITE
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("DropChoice_{}_{i}", field.as_str())),
            width: NUMBER_DROPDOWN_CHOICE_WIDTH,
            height: NUMBER_DROPDOWN_CHOICE_HEIGHT,
            onclick: CharCreateAction::SelectChoice(field, idx),
            background_color: "0.15,0.12,0.08,0.9",
            border: "1px solid 0.4,0.35,0.2,0.6",
            fontstring {
                name: dyn_name(format!("DropVal_{}_{i}", field.as_str())),
                width: NUMBER_DROPDOWN_CHOICE_WIDTH,
                height: NUMBER_DROPDOWN_CHOICE_HEIGHT,
                text: val_text,
                font: GameFont::FrizQuadrata,
                font_size: 11.0,
                font_color: color,
            }
        }
    }
}

fn build_dropdown_choices(
    field: AppearanceField,
    swatches: &[Option<[u8; 3]>],
    selected: u8,
) -> Element {
    swatches
        .iter()
        .enumerate()
        .flat_map(|(i, swatch)| {
            let is_sel = i as u8 == selected;
            match swatch {
                Some(c) => dropdown_color_choice(field, i, *c, is_sel),
                None => dropdown_number_choice(field, i, is_sel),
            }
        })
        .collect()
}

pub(super) fn customization_row(
    label: &str,
    value: u8,
    value_label: &str,
    swatches: &[Option<[u8; 3]>],
    field: AppearanceField,
) -> Element {
    let current_swatch = swatches.get(value as usize).copied().flatten();
    let center = match current_swatch {
        Some(color) => swatch_texture(field, color),
        None => appearance_row_value(field, value, value_label),
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("Appearance_{}", field.as_str())),
            width: 280.0,
            height: 44.0,
            {appearance_row_label(field, label)}
            {stepper_dec_button(field)}
            {center}
            {stepper_inc_button(field)}
        }
    }
}

pub(super) fn dropdown_panel(
    field: AppearanceField,
    swatches: &[Option<[u8; 3]>],
    selected_idx: u8,
    y_offset: f32,
) -> Element {
    let choices = build_dropdown_choices(field, swatches, selected_idx);
    let y_str = format!("{y_offset}");
    rsx! {
        r#frame {
            name: dyn_name(format!("Dropdown_{}", field.as_str())),
            width: DROPDOWN_WIDTH,
            height: 0.0,
            strata: FrameStrata::Dialog,
            background_color: "0.05,0.05,0.05,1.0",
            border: "1px solid 0.4,0.35,0.2,0.8",
            layout: "flex-row-wrap",
            gap: DROPDOWN_GAP,
            padding: DROPDOWN_PADDING,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: y_str,
            }
            {choices}
        }
    }
}
