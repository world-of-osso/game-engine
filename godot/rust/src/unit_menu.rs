//! TargetFrame and FocusFrame right-click menu (Retail `UnitPopup`; Bevy
//! `rendering/ui/unit_frames.rs` `UnitFrameClick`): right-clicking either frame opens the
//! authored `UnitFrameContextMenu`. It leads with Set Focus (`FocusUnit(unit)`), or Clear
//! Focus (`ClearFocus()`) when opened from FocusFrame (the `FOCUS` menu). The focus is
//! client-local, as in Retail. For a player it has the group entries and, for another player,
//! Trade (`UnitPopupTradeButtonMixin:OnClick` → `InitiateTrade(unit)`); every unit gets the
//! raid target icons (`UnitPopupRaidTargetButtonMixin`), listed inline rather than in a
//! submenu. An entry's click runs it and closes the menu; a click outside closes it.
use game_engine_ui_model::group_state::{GroupMenuEntry, group_menu_entries};
use game_engine_ui_model::inworld_unit_frames_component::{
    ACTION_UNIT_MENU_CLEAR_FOCUS, ACTION_UNIT_MENU_CLOSE, ACTION_UNIT_MENU_SET_FOCUS,
    ACTION_UNIT_MENU_TRADE, UNIT_MENU_W, UnitFrameMenuState, UnitMenuItem, focus_menu_item,
    unit_menu_height,
};
use godot::classes::{InputEvent, InputEventMouseButton};
use godot::global::MouseButton;
use godot::prelude::*;
use shared::components::Player;

use crate::GameClient;
use crate::frame_error::{FrameError, SessionError};
use crate::replicated::UnitFields;

/// `UnitPopupRaidTarget<n>ButtonMixin:OnClick` → `SetRaidTargetIcon(unit, n)`.
const ACTION_UNIT_MENU_RAID_TARGET_PREFIX: &str = "unit_menu_raid_target:";
/// `UnitPopupRaidTargetButtonMixin:GetEntries` order with `RAID_TARGET_8..1` and
/// `RAID_TARGET_NONE` (GlobalStrings).
const RAID_TARGET_ENTRIES: [(u8, &str); 9] = [
    (8, "Skull"),
    (7, "Cross"),
    (6, "Square"),
    (5, "Moon"),
    (4, "Triangle"),
    (3, "Diamond"),
    (2, "Circle"),
    (1, "Star"),
    (0, "None"),
];

#[derive(Default)]
pub(crate) struct UnitMenu {
    /// The unit the open menu acts on, and its name when it is a player.
    unit: Option<u64>,
    player: Option<String>,
    pub state: UnitFrameMenuState,
}

/// The raid target icon entries.
pub(crate) fn raid_target_items() -> Vec<UnitMenuItem> {
    RAID_TARGET_ENTRIES
        .iter()
        .map(|&(index, label)| UnitMenuItem {
            name: format!("UnitFrameContextMenuRaidTarget{index}"),
            label: label.into(),
            action: format!("{ACTION_UNIT_MENU_RAID_TARGET_PREFIX}{index}"),
        })
        .collect()
}

/// Group entries, then Trade for another player.
pub(crate) fn player_items(
    group: &game_engine_ui_model::group_state::GroupState,
    local: &str,
    unit: &str,
) -> Vec<UnitMenuItem> {
    let mut items: Vec<UnitMenuItem> = group_menu_entries(group, local, unit)
        .into_iter()
        .map(|entry| UnitMenuItem {
            name: format!("UnitFrameContextMenu{}", entry.frame_key()),
            label: entry.label().into(),
            action: entry.action().into(),
        })
        .collect();
    if !unit.eq_ignore_ascii_case(local) {
        items.push(UnitMenuItem {
            name: "UnitFrameContextMenuTrade".into(),
            label: "Trade".into(),
            action: ACTION_UNIT_MENU_TRADE.into(),
        });
    }
    items
}

/// The unit a right-click on the unit frames opens the menu for, and whether it came from
/// FocusFrame (`FocusFrame_OpenMenu`, TargetFrame.lua:1131-1138). FocusFrame is shown only
/// while the focus unit is replicated, so `focus` is that replicated unit.
fn menu_unit(
    on_target_frame: bool,
    on_focus_frame: bool,
    target: Option<u64>,
    focus: Option<u64>,
) -> Option<(u64, bool)> {
    if on_focus_frame {
        return focus.map(|unit| (unit, true));
    }
    target.filter(|_| on_target_frame).map(|unit| (unit, false))
}

/// The open menu's entries: the focus entry, the player entries, the raid target icons.
fn menu_items(from_focus_frame: bool, player_items: Vec<UnitMenuItem>) -> Vec<UnitMenuItem> {
    std::iter::once(focus_menu_item(from_focus_frame))
        .chain(player_items)
        .chain(raid_target_items())
        .collect()
}

/// The focus after a focus entry's click on `unit`'s menu
/// (`UnitPopupSetFocusButtonMixin:OnClick` → `FocusUnit(unit)`,
/// `UnitPopupClearFocusButtonMixin:OnClick` → `ClearFocus()`).
fn focus_after(action: &str, unit: u64, focus: Option<u64>) -> Option<u64> {
    match action {
        ACTION_UNIT_MENU_SET_FOCUS => Some(unit),
        ACTION_UNIT_MENU_CLEAR_FOCUS => None,
        _ => focus,
    }
}

fn inside([x, y, w, h]: [f32; 4], point: Vector2) -> bool {
    point.x >= x && point.x <= x + w && point.y >= y && point.y <= y + h
}

impl GameClient {
    /// Right-click on a player's TargetFrame opens the menu and any other right-click
    /// closes it; a left press outside the open menu closes it. Clicks on its entries
    /// reach the authored buttons. Returns whether the event opened the menu.
    pub(super) fn unit_menu_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if !button.is_pressed() || self.game_menu_ui.is_some() {
            return false;
        }
        let point = button.get_position();
        match button.get_button_index() {
            MouseButton::RIGHT => self.open_unit_menu(point),
            MouseButton::LEFT if !self.pointer_over_unit_menu() => {
                self.unit_menu = UnitMenu::default();
                false
            }
            _ => false,
        }
    }

    fn open_unit_menu(&mut self, point: Vector2) -> bool {
        self.unit_menu = UnitMenu::default();
        let on_frame = |name: &str| {
            self.targeting
                .frame_ui()
                .and_then(|ui| ui.bind().frame_rect(name))
                .is_some_and(|(rect, _)| inside(rect, point))
        };
        let (on_target_frame, on_focus_frame) = (on_frame("TargetFrame"), on_frame("FocusFrame"));
        let focus = self.targeting.focus.filter(|&id| {
            self.replica
                .unit(id)
                .is_some_and(crate::replicated::is_unit)
        });
        let Some((unit, from_focus_frame)) = menu_unit(
            on_target_frame,
            on_focus_frame,
            self.targeting_target(),
            focus,
        ) else {
            return false;
        };
        let Some(local) = self.account.session.selected_character_name.clone() else {
            return false;
        };
        let Some(name) = self.replica.unit(unit).and_then(|unit| unit.name()) else {
            return false;
        };
        let title = name.to_owned();
        let player = self.target_player_name(unit);
        let player_items = match &player {
            Some(player) => player_items(&self.account.group, &local, player),
            None => Vec::new(),
        };
        let items = menu_items(from_focus_frame, player_items);
        self.unit_menu = UnitMenu {
            state: self.unit_menu_state(title, items, point),
            unit: Some(unit),
            player,
        };
        true
    }

    /// The menu closes once its unit is neither the target nor the focus.
    pub(super) fn close_unit_menu_without_unit(&mut self) {
        let Some(unit) = self.unit_menu.unit else {
            return;
        };
        if Some(unit) != self.targeting_target() && Some(unit) != self.targeting.focus {
            self.unit_menu = UnitMenu::default();
        }
    }

    fn target_player_name(&self, unit: u64) -> Option<String> {
        self.replica
            .unit(unit)
            .and_then(|unit| unit.get::<Player>())
            .map(|player| player.name.clone())
    }

    /// The menu at the pointer, kept on screen (Bevy `UnitFrameClick::menu_for`).
    fn unit_menu_state(
        &self,
        title: String,
        items: Vec<UnitMenuItem>,
        point: Vector2,
    ) -> UnitFrameMenuState {
        let scale = self.effective_ui_scale();
        let viewport = self
            .base()
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size / scale)
            .unwrap_or_default();
        let max_x = (viewport.x - UNIT_MENU_W).max(0.0);
        let max_y = (viewport.y - unit_menu_height(items.len())).max(0.0);
        UnitFrameMenuState {
            visible: true,
            title,
            x: (point.x / scale).clamp(0.0, max_x),
            y: (point.y / scale).clamp(0.0, max_y),
            player_items: items,
            difficulty_menu: None,
        }
    }

    /// Whether the pointer is over the open menu (its frame or any entry).
    fn pointer_over_unit_menu(&self) -> bool {
        if !self.unit_menu.state.visible {
            return false;
        }
        let mut node = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.gui_get_hovered_control())
            .map(|control| control.upcast::<Node>());
        while let Some(current) = node {
            if current
                .get_name()
                .to_string()
                .starts_with("UnitFrameContextMenu")
            {
                return true;
            }
            node = current.get_parent();
        }
        false
    }

    /// The menu's button clicks; the menu closes after any entry.
    pub(super) fn poll_unit_menu_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.targeting.frame_ui().cloned() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return Ok(());
            }
            self.run_unit_menu_action(&action)?;
        }
    }

    fn run_unit_menu_action(&mut self, action: &str) -> Result<(), SessionError> {
        let menu = std::mem::take(&mut self.unit_menu);
        if !menu.state.visible {
            return Ok(());
        }
        if let Some(index) = action.strip_prefix(ACTION_UNIT_MENU_RAID_TARGET_PREFIX) {
            let index = index
                .parse()
                .map_err(|error| SessionError(format!("Raid target entry {action}: {error}")))?;
            return match menu.unit {
                Some(unit) => self.set_raid_icon(unit, index),
                None => Ok(()),
            };
        }
        if let (ACTION_UNIT_MENU_SET_FOCUS | ACTION_UNIT_MENU_CLEAR_FOCUS, Some(unit)) =
            (action, menu.unit)
        {
            self.targeting.focus = focus_after(action, unit, self.targeting.focus);
            return Ok(());
        }
        let Some(unit) = menu.player else {
            return Ok(());
        };
        match action {
            ACTION_UNIT_MENU_TRADE => self.initiate_trade(unit),
            ACTION_UNIT_MENU_CLOSE => Ok(()),
            action => match GroupMenuEntry::from_action(action) {
                Some(entry) => self.account.send_group(entry.command(&unit)),
                None => Err(SessionError(format!(
                    "Unit menu action not converted: {action}"
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOGGER: u64 = 42;
    const WOLF: u64 = 77;

    fn labels(items: &[UnitMenuItem]) -> Vec<&str> {
        items.iter().map(|item| item.label.as_str()).collect()
    }

    /// Retail `TARGET` lists Set Focus; `FOCUS`, opened from FocusFrame, Clear Focus.
    #[test]
    fn clear_focus_is_offered_only_on_the_focus_units_menu() {
        let target_menu = menu_unit(true, false, Some(HOGGER), Some(WOLF));
        assert_eq!(target_menu, Some((HOGGER, false)));
        let focus_menu = menu_unit(false, true, Some(HOGGER), Some(WOLF));
        assert_eq!(focus_menu, Some((WOLF, true)));
        assert_eq!(menu_unit(false, true, Some(HOGGER), None), None, "no focus");
        assert_eq!(menu_unit(false, false, Some(HOGGER), Some(WOLF)), None);

        let target_items = menu_items(false, Vec::new());
        assert_eq!(labels(&target_items)[0], "Set Focus");
        assert!(!labels(&target_items).contains(&"Clear Focus"));
        let focus_items = menu_items(true, Vec::new());
        assert_eq!(labels(&focus_items)[0], "Clear Focus");
        assert!(!labels(&focus_items).contains(&"Set Focus"));
        assert!(labels(&focus_items).contains(&"Skull"), "raid targets stay");
    }

    #[test]
    fn set_focus_takes_the_menus_unit_and_clear_focus_drops_it() {
        let focused = focus_after(ACTION_UNIT_MENU_SET_FOCUS, HOGGER, None);
        assert_eq!(focused, Some(HOGGER));
        assert_eq!(
            focus_after(ACTION_UNIT_MENU_SET_FOCUS, WOLF, focused),
            Some(WOLF)
        );
        assert_eq!(
            focus_after(ACTION_UNIT_MENU_CLEAR_FOCUS, HOGGER, focused),
            None
        );
        assert_eq!(
            focus_after(ACTION_UNIT_MENU_CLOSE, HOGGER, focused),
            focused
        );
    }
}
