//! Choose and reorder the active spec's authored cooldown entries.
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

struct DynName(String);
pub const SETTINGS_PAGE_SIZE: usize = 10;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CooldownSettingEntry {
    pub id: u32,
    pub category: u32,
    pub label: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CooldownSettingsView {
    pub open: bool,
    pub category: usize,
    pub page: usize,
    pub entries: Vec<CooldownSettingEntry>,
    pub status: String,
}

pub fn settings_panel(state: &CooldownSettingsView) -> Element {
    if !state.open {
        return Vec::new();
    }
    let rows = settings_rows(state);
    let tabs = category_tabs();
    let close = panel_button(
        "CooldownSettingsClose".into(),
        "Close",
        "cooldown_close".into(),
        [480.0, 492.0, 90.0, 28.0],
    );
    let previous = panel_button(
        "CooldownSettingsPrevious".into(),
        "Previous",
        "cooldown_previous".into(),
        [16.0, 492.0, 100.0, 28.0],
    );
    let next = panel_button(
        "CooldownSettingsNext".into(),
        "Next",
        "cooldown_next".into(),
        [120.0, 492.0, 100.0, 28.0],
    );
    rsx! {
        r#frame {
            name: {FrameName("CooldownViewerSettings")}, width: 590.0, height: 540.0,
            strata: FrameStrata::Dialog, background_color: "0.08,0.06,0.04,0.98",
            pos_type: "absolute", left: "50%", top: "50%", margin_left: -295.0, margin_top: -270.0,
            fontstring {
                name: {FrameName("CooldownSettingsTitle")}, width: 550.0, height: 30.0,
                text: "Cooldown Manager", font: GameFont::FrizQuadrata, font_size: 18.0,
                font_color: "1.0,0.82,0.0,1.0", pos_type: "absolute", pos_x: 16.0, pos_y: 8.0,
            }
            {tabs}
            {rows}
            fontstring {
                name: {FrameName("CooldownSettingsStatus")}, width: 550.0, height: 44.0,
                text: {state.status.as_str()}, font: GameFont::FrizQuadrata, font_size: 11.0,
                font_color: "1.0,0.82,0.0,1.0", pos_type: "absolute", pos_x: 16.0, pos_y: 438.0,
            }
            {previous}
            {next}
            {close}
        }
    }
}

fn category_tabs() -> Element {
    ["Essential", "Utility", "Tracked Buffs", "Tracked Bars"]
        .into_iter()
        .enumerate()
        .flat_map(|(index, label)| {
            panel_button(
                format!("CooldownSettingsCategory{index}"),
                label,
                format!("cooldown_category:{index}"),
                [16.0 + index as f32 * 140.0, 44.0, 136.0, 28.0],
            )
        })
        .collect()
}

fn settings_rows(state: &CooldownSettingsView) -> Element {
    state
        .entries
        .iter()
        .filter(|entry| entry.category as usize == state.category)
        .skip(state.page * SETTINGS_PAGE_SIZE)
        .take(SETTINGS_PAGE_SIZE)
        .enumerate()
        .flat_map(|(index, entry)| setting_row(index, entry))
        .collect()
}

fn setting_row(index: usize, entry: &CooldownSettingEntry) -> Element {
    let label = &entry.label;
    let y = 84.0 + index as f32 * 34.0;
    let select = crate::selection_list::text_row(
        format!("CooldownSettingsSelect{}", entry.id),
        label,
        &format!("cooldown_select:{}", entry.id),
        (true, entry.selected),
        (16.0, y, 454.0, 30.0),
        &crate::selection_list::SEARCH,
    );
    let up = panel_button(
        format!("CooldownSettingsUp{}", entry.id),
        "Up",
        format!("cooldown_up:{}", entry.id),
        [476.0, y, 46.0, 30.0],
    );
    let down = panel_button(
        format!("CooldownSettingsDown{}", entry.id),
        "Down",
        format!("cooldown_down:{}", entry.id),
        [526.0, y, 46.0, 30.0],
    );
    [select, up, down].into_iter().flatten().collect()
}

fn panel_button(name: String, label: &str, action: String, rect: [f32; 4]) -> Element {
    rsx! {
        button {
            name: {DynName(name)}, width: {rect[2]}, height: {rect[3]}, text: label,
            font: GameFont::FrizQuadrata, font_size: 12.0,
            onclick: {action.as_str()}, background_color: "0.18,0.13,0.07,1.0",
            pos_type: "absolute", pos_x: {rect[0]}, pos_y: {rect[1]},
        }
    }
}
