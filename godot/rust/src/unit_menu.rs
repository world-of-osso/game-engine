//! TargetFrame right-click menu (Retail `UnitPopup`; Bevy `rendering/ui/unit_frames.rs`
//! `UnitFrameClick`): right-clicking the target frame opens the authored
//! `UnitFrameContextMenu`. For a player it has the group entries and, for another player,
//! Trade (`UnitPopupTradeButtonMixin:OnClick` → `InitiateTrade(unit)`); every unit gets the
//! raid target icons (`UnitPopupRaidTargetButtonMixin`), listed inline rather than in a
//! submenu. An entry's click runs it and closes the menu; a click outside closes it.
//! Set/Clear Focus are not converted.
use game_engine_ui_model::group_state::{GroupMenuEntry, group_menu_entries};
use game_engine_ui_model::inworld_unit_frames_component::{
    ACTION_UNIT_MENU_CLEAR_FOCUS, ACTION_UNIT_MENU_CLOSE, ACTION_UNIT_MENU_SET_FOCUS,
    ACTION_UNIT_MENU_TRADE, UNIT_MENU_W, UnitFrameMenuState, UnitMenuItem, unit_menu_height,
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
        let on_target_frame = self
            .targeting
            .frame_ui()
            .and_then(|ui| ui.bind().frame_rect("TargetFrame"))
            .is_some_and(|(rect, _)| inside(rect, point));
        let (Some(unit), Some(local)) = (
            self.targeting_target(),
            self.account.session.selected_character_name.clone(),
        ) else {
            return false;
        };
        let Some(name) = self.replica.unit(unit).and_then(|unit| unit.name()) else {
            return false;
        };
        if !on_target_frame {
            return false;
        }
        let title = name.to_owned();
        let player = self.target_player_name(unit);
        let mut items = match &player {
            Some(player) => player_items(&self.account.group, &local, player),
            None => Vec::new(),
        };
        items.extend(raid_target_items());
        self.unit_menu = UnitMenu {
            state: self.unit_menu_state(title, items, point),
            unit: Some(unit),
            player,
        };
        true
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
        let Some(unit) = menu.player else {
            return Ok(());
        };
        match action {
            ACTION_UNIT_MENU_TRADE => self.initiate_trade(unit),
            ACTION_UNIT_MENU_SET_FOCUS | ACTION_UNIT_MENU_CLEAR_FOCUS => self
                .add_world_error("Focus is not converted to the native client.")
                .map_err(SessionError),
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
