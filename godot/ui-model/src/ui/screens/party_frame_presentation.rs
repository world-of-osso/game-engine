//! Party-only presentation; raid frames retain their existing settings.
use super::group_frames_component::GroupFramesState;
use crate::hud_layout::HudAnchor;
use game_engine_core::ui_layout_data::{PartyFrameSettings, PartySort};
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::{
    rsx,
    widget_def::{Attr, AttrValue, Element, WidgetChild, WidgetDef},
};

struct DynName(String);

pub fn role_order(role: GroupRoleSnapshot) -> u8 {
    match role {
        GroupRoleSnapshot::Tank => 0,
        GroupRoleSnapshot::Healer => 1,
        GroupRoleSnapshot::Damage => 2,
        GroupRoleSnapshot::None => 3,
    }
}

pub fn sorted_party_state(state: &GroupFramesState, sort: PartySort) -> GroupFramesState {
    let mut state = state.clone();
    match sort {
        PartySort::Group => {}
        PartySort::Alphabetical => {
            state.party.sort_by_cached_key(|member| member.name.clone());
            state
                .portrait_party
                .members
                .sort_by_cached_key(|member| member.name.clone());
        }
        PartySort::Role => {
            state
                .party
                .sort_by_cached_key(|member| (role_order(member.role), member.name.clone()));
            state
                .portrait_party
                .members
                .sort_by_cached_key(|member| (role_order(member.role), member.name.clone()));
        }
    }
    state
}

pub fn party_chrome(name: &str, width: f32, height: f32, settings: PartyFrameSettings) -> Element {
    // Explicitly opt-in: the default has no additional party container chrome.
    rsx! { r#frame {
        name: {DynName(format!("{name}SettingsBackground"))}, width, height,
        hidden: { !settings.background.unwrap_or(false) }, background_color: "0.08,0.08,0.08,0.65",
        pos_type: "absolute", left: 0.0, top: 0.0,
    }
    r#frame {
        name: {DynName(format!("{name}SettingsBorder"))}, width, height,
        hidden: { !settings.border.unwrap_or(false) }, border: "1,0.5,0.5,0.5,1",
        pos_type: "absolute", left: 0.0, top: 0.0,
    } }
}

pub fn scale_party(frame: Element, settings: PartyFrameSettings, anchor: &HudAnchor) -> Element {
    crate::unit_frame_style::styled_frame(
        frame,
        &crate::unit_frame_style::UnitFrameStyle {
            scale: settings.scale(),
            ..crate::unit_frame_style::UnitFrameStyle::AUTHORED
        },
        anchor,
    )
}

fn set_attr(widget: &mut WidgetDef, name: &'static str, value: f32) {
    if let Some(attr) = widget
        .attrs
        .iter_mut()
        .find(|attr| attr.effective_name() == name)
    {
        attr.value = AttrValue::Dynamic(value.to_string());
    } else {
        widget
            .attrs
            .push(Attr::new_dynamic(name, value.to_string()));
    }
}

pub fn place_party_member(member: &mut Element, x: f32, y: f32) {
    for child in member {
        if let WidgetChild::Widget(widget) = child {
            set_attr(widget, "pos_x", x);
            set_attr(widget, "pos_y", y);
        }
    }
}

pub fn resize_party_member(children: &mut [WidgetChild], sx: f32, sy: f32) {
    if sx == 1.0 && sy == 1.0 {
        return;
    }
    for child in children {
        match child {
            WidgetChild::Widget(widget) => {
                for attr in &mut widget.attrs {
                    let factor = match attr.effective_name() {
                        "width" | "pos_x" | "left" | "right" | "margin_left" => sx,
                        "height" | "pos_y" | "top" | "bottom" | "margin_top" => sy,
                        "font_size" => sx.min(sy),
                        _ => continue,
                    };
                    if let Ok(length) = attr.value_str().parse::<f32>() {
                        attr.value = AttrValue::Dynamic((length * factor).to_string());
                    }
                }
                resize_party_member(&mut widget.children, sx, sy);
            }
            WidgetChild::Fragment(children) => resize_party_member(children, sx, sy),
            WidgetChild::Dynamic => {}
        }
    }
}
