//! Options owns popup state; shared menu_primitives owns every dropdown/radio/backdrop.
use super::menu_primitives::{self, ContextMenu, ContextMenuItem, MenuElementKind};
use ui_toolkit::widget_def::{Element, WidgetChild};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionsDropdownState {
    pub menu: Option<OptionsDropdownMenu>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct OptionsDropdownMenu {
    pub key: String,
    /// Anchor below its button, relative to OptionsRoot in logical UI units.
    pub position: [f32; 2],
}
impl OptionsDropdownState {
    pub fn toggle(&mut self, key: &str, position: [f32; 2]) {
        self.menu = if self.menu.as_ref().is_some_and(|menu| menu.key == key) {
            None
        } else {
            Some(OptionsDropdownMenu {
                key: key.into(),
                position,
            })
        };
    }
    pub fn close(&mut self) -> bool {
        self.menu.take().is_some()
    }
}

pub fn radio_popup(
    key: &str,
    label: &str,
    choices: &[(u8, &str)],
    selected: u8,
    position: [f32; 2],
) -> Element {
    // ContextMenu geometry is owned by the shared helper. Replace only row kinds with
    // its Radio element; keep the helper's frame, backdrop, title and divider unchanged.
    let names: Vec<_> = choices
        .iter()
        .map(|(value, _)| format!("Choice{key}{value}Hit"))
        .collect();
    let actions: Vec<_> = choices
        .iter()
        .map(|(value, _)| format!("options_toggle:{key}:{value}"))
        .collect();
    let items: Vec<_> = choices
        .iter()
        .enumerate()
        .map(|(index, (_, text))| ContextMenuItem {
            name: &names[index],
            label: text,
            action: &actions[index],
        })
        .collect();
    let width = choices
        .iter()
        .map(|(_, text)| {
            ui_toolkit::text_measure::measure_text(
                text,
                ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
                12.0,
            )
            .expect("Options radio font loaded")
            .0 + 32.0
        })
        .fold(220.0, f32::max);
    let rows: Element = choices
        .iter()
        .enumerate()
        .flat_map(|(index, (value, _))| {
            menu_primitives::menu_element(
                items[index],
                MenuElementKind::Radio {
                    checked: *value == selected,
                },
                true,
                menu_primitives::menu_row_rect(width, index),
            )
        })
        .collect();
    menu_primitives::context_menu_with_rows(
        ContextMenu {
            frame_name: &format!("OptionsChoicePopup{key}"),
            title_name: &format!("OptionsChoiceTitle{key}"),
            divider_name: &format!("OptionsChoiceDivider{key}"),
            hidden: false,
            title: label,
            width,
            x: position[0],
            y: position[1],
            items: &items,
        },
        rows,
    )
}

/// Popup children must be attached to OptionsRoot, not clipped by a Settings ScrollBox.
pub fn detach_popup(elements: &mut Element) -> Element {
    let mut popups = Vec::new();
    let mut index = 0;
    while index < elements.len() {
        let is_popup = matches!(&elements[index], WidgetChild::Widget(widget) if widget.name.as_deref().is_some_and(|name| name.starts_with("OptionsChoicePopup")));
        if is_popup {
            popups.push(elements.remove(index));
            continue;
        }
        match &mut elements[index] {
            WidgetChild::Widget(widget) => popups.extend(detach_popup(&mut widget.children)),
            WidgetChild::Fragment(children) => popups.extend(detach_popup(children)),
            WidgetChild::Dynamic => {}
        }
        index += 1;
    }
    popups
}
