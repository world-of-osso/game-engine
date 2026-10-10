//! TargetFrame, FocusFrame and PetFrame right-click menu (Retail `UnitPopup`; Bevy
//! `rendering/ui/unit_frames.rs` `UnitFrameClick`): right-clicking any frame opens the
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
const ACTION_DISMISS_PET: &str = "unit_menu_dismiss_pet";
const ACTION_RIDE_VEHICLE: &str = "unit_menu_ride_vehicle";
const ACTION_DISMISS_HUNTER_PET: &str = "unit_menu_dismiss_hunter_pet";
const DISMISS_PET_SPELL: u32 = 2641;
const HUNTER_CLASS: u8 = 3;
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
fn ride_item() -> UnitMenuItem {
    UnitMenuItem {
        name: "UnitFrameContextMenuRide".into(),
        label: "Ride".into(),
        action: ACTION_RIDE_VEHICLE.into(),
    }
}

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

/// Retail UnitPopupSharedButtonMixins.lua:1037-1056: PET_DISMISS calls
/// PetDismiss for summoned pets, but casts learned 2641 for hunter pets.
fn pet_menu_items(hunter: bool, knows_dismiss: bool) -> Vec<UnitMenuItem> {
    if hunter && !knows_dismiss {
        return Vec::new();
    }
    let action = if hunter {
        ACTION_DISMISS_HUNTER_PET
    } else {
        ACTION_DISMISS_PET
    };
    vec![UnitMenuItem {
        name: "UnitFrameContextMenuDismissPet".into(),
        label: "Dismiss Pet".into(),
        action: action.into(),
    }]
}

fn dismiss_pet_action(
    action: &str,
    unit: u64,
    pet: Option<u64>,
) -> Option<shared::protocol::PetAction> {
    if action != ACTION_DISMISS_PET || Some(unit) != pet {
        return None;
    }
    Some(shared::protocol::PetAction {
        pet: unit,
        action: shared::protocol::pet_action_button(
            shared::protocol::COMMAND_ABANDON,
            shared::protocol::ACT_COMMAND,
        ),
        target: None,
        position: None,
    })
}

fn inside([x, y, w, h]: [f32; 4], point: Vector2) -> bool {
    point.x >= x && point.x <= x + w && point.y >= y && point.y <= y + h
}

fn group_member_at<'a>(
    frames: &'a game_engine_ui_model::group_frames_component::GroupFramesState,
    compact: bool,
    point: Vector2,
    rect: impl Fn(&str) -> Option<[f32; 4]>,
) -> Option<&'a str> {
    let hit = |frame: String, name: &'a str| {
        rect(&frame)
            .filter(|bounds| inside(*bounds, point))
            .map(|_| name)
    };
    if frames.raid.iter().any(|group| !group.is_empty()) {
        return frames.raid.iter().enumerate().find_map(|(group, members)| {
            members.iter().enumerate().find_map(|(index, member)| {
                hit(
                    format!("CompactRaidGroup{}Member{}", group + 1, index + 1),
                    &member.name,
                )
            })
        });
    }
    if compact {
        return frames.party.iter().enumerate().find_map(|(index, member)| {
            hit(
                format!("CompactPartyFrameMember{}", index + 1),
                &member.name,
            )
        });
    }
    frames
        .portrait_party
        .members
        .iter()
        .enumerate()
        .find_map(|(index, member)| hit(format!("PartyMemberFrame{}", index + 1), &member.name))
}

impl GameClient {
    /// Right-click on group, TargetFrame, FocusFrame or the local PetFrame opens the menu; other right-clicks
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
        if self.open_group_member_menu(point) {
            return true;
        }
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
        let selected = if on_frame("PetFrame") {
            self.local_pet_id().map(|pet| (pet, false))
        } else {
            menu_unit(
                on_target_frame,
                on_focus_frame,
                self.targeting_target(),
                focus,
            )
        };
        let Some((unit, from_focus_frame)) = selected else {
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
        let mut player_items = match &player {
            Some(player) => player_items(&self.account.group, &local, player),
            None => Vec::new(),
        };
        if player
            .as_ref()
            .is_some_and(|name| self.ride_mount_id(name).is_some())
        {
            player_items.push(ride_item());
        }
        let pet_items = if Some(unit) == self.local_pet_id() {
            let hunter = self
                .world
                .local_player_id()
                .and_then(|id| self.replica.unit(id))
                .and_then(|unit| unit.get::<Player>())
                .is_some_and(|player| player.class == HUNTER_CLASS);
            let knows_dismiss = self.account.spells.known().contains(&DISMISS_PET_SPELL);
            pet_menu_items(hunter, knows_dismiss)
        } else {
            Vec::new()
        };
        let items = menu_items(
            from_focus_frame,
            player_items.into_iter().chain(pet_items).collect(),
        );
        self.unit_menu = UnitMenu {
            state: self.unit_menu_state(title, items, point),
            unit: Some(unit),
            player,
        };
        true
    }

    fn open_group_member_menu(&mut self, point: Vector2) -> bool {
        let Some(ui) = self.group_frames.frame_ui() else {
            return false;
        };
        let frames = self.group_frames_view();
        let compact = game_engine_ui_model::hud_layout::active_layout_settings()
            .use_raid_style_party_frames
            .unwrap_or(true);
        let member = group_member_at(&frames, compact, point, |name| {
            ui.bind().frame_rect(name).map(|(rect, _)| rect)
        });
        let Some(member) = member else {
            return false;
        };
        let Some(local) = self.account.session.selected_character_name.as_deref() else {
            return false;
        };
        let mut items = player_items(&self.account.group, local, member);
        if self.ride_mount_id(member).is_some() {
            items.push(ride_item());
        }
        self.unit_menu = UnitMenu {
            state: self.unit_menu_state(member.to_owned(), items, point),
            unit: None,
            player: Some(member.to_owned()),
        };
        true
    }

    /// Group menus depend on roster membership, not replicated unit visibility.
    /// Other menus close once their unit is neither target, focus nor the local pet.
    pub(super) fn close_unit_menu_without_unit(&mut self) {
        let Some(unit) = self.unit_menu.unit else {
            if let Some(name) = &self.unit_menu.player {
                let in_group = self
                    .account
                    .group
                    .members
                    .iter()
                    .any(|member| &member.name == name);
                if !in_group {
                    self.unit_menu = UnitMenu::default();
                }
            }
            return;
        };
        let still_available = [
            self.targeting_target(),
            self.targeting.focus,
            self.local_pet_id(),
        ]
        .contains(&Some(unit));
        if !still_available {
            self.unit_menu = UnitMenu::default();
        }
    }

    fn ride_mount_id(&self, name: &str) -> Option<u64> {
        let local_id = self.world.local_player_id()?;
        let local = self.replica.unit(local_id)?;
        if local.has::<shared::components::VehiclePassenger>()
            || local.has::<shared::components::Mounted>()
        {
            return None;
        }
        if !self
            .account
            .group
            .members
            .iter()
            .any(|member| member.name.eq_ignore_ascii_case(name))
        {
            return None;
        }
        self.replica
            .units()
            .find(|unit| {
                unit.server_id != local_id
                    && unit
                        .get::<Player>()
                        .is_some_and(|player| player.name.eq_ignore_ascii_case(name))
                    && unit
                        .get::<shared::components::Mounted>()
                        .is_some_and(|mount| mount.vehicle_id != 0)
            })
            .map(|unit| unit.server_id)
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
        if let Some(unit) = menu.unit {
            let pet = self.local_pet_id();
            if let Some(message) = dismiss_pet_action(action, unit, pet) {
                return self.send_pet_action(message);
            }
            if action == ACTION_DISMISS_HUNTER_PET && Some(unit) == pet {
                return self.cast_spell(DISMISS_PET_SPELL);
            }
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
            ACTION_RIDE_VEHICLE => match self.ride_mount_id(&unit) {
                Some(driver) => self.account.send_board_vehicle(driver),
                None => Err(SessionError(
                    "Passenger mount is no longer available".into(),
                )),
            },
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

#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn ride_player_mount(&self, name: GString) -> GString {
        match self.ride_mount_id(&name.to_string()) {
            Some(driver) => self
                .account
                .send_board_vehicle(driver)
                .err()
                .map_or_else(GString::new, |error| GString::from(error.to_string())),
            None => GString::from("Party member has no available passenger mount"),
        }
    }

    #[func]
    fn leave_vehicle(&self) -> GString {
        self.account
            .send_exit_vehicle()
            .err()
            .map_or_else(GString::new, |error| GString::from(error.to_string()))
    }

    #[func]
    fn vehicle_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let Some(unit) = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
        else {
            return state;
        };
        state.set("entity", unit.server_id as i64);
        if let Some(mount) = unit.get::<shared::components::Mounted>() {
            state.set("vehicle_id", mount.vehicle_id);
            state.set("display_id", mount.mount_display_id);
            state.set("passengers", mount.seats.iter().flatten().count() as i64);
        }
        if let Some(passenger) = unit.get::<shared::components::VehiclePassenger>() {
            state.set("driver", passenger.driver as i64);
            state.set("seat_index", passenger.seat_index as i64);
            state.set("seat_id", passenger.seat_id);
        }
        state
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

    #[test]
    fn pet_dismiss_menu_entry_sends_command_three_only_for_own_pet() {
        let items = menu_items(false, pet_menu_items(false, false));
        let entry = items
            .iter()
            .find(|item| item.label == "Dismiss Pet")
            .expect("PET_DISMISS on the pet menu");
        let message = dismiss_pet_action(&entry.action, WOLF, Some(WOLF)).unwrap();
        assert_eq!(message.pet, WOLF);
        assert_eq!(
            shared::protocol::pet_action_button_action(message.action),
            3
        );
        assert_eq!(
            shared::protocol::pet_action_button_type(message.action),
            shared::protocol::ACT_COMMAND
        );
        assert_eq!(message.target, None);
        assert_eq!(message.position, None);
        assert!(dismiss_pet_action(&entry.action, HOGGER, Some(WOLF)).is_none());
        assert!(dismiss_pet_action(&entry.action, WOLF, None).is_none());
        assert!(dismiss_pet_action(ACTION_UNIT_MENU_CLOSE, WOLF, Some(WOLF)).is_none());
    }

    #[test]
    fn hunter_pet_dismiss_requires_the_learned_spell_not_abandon_command() {
        assert!(pet_menu_items(true, false).is_empty());
        let items = pet_menu_items(true, true);
        assert_eq!(labels(&items), ["Dismiss Pet"]);
        assert!(dismiss_pet_action(&items[0].action, WOLF, Some(WOLF)).is_none());
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
