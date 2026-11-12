//! Retail `SecureUnitButton` clicks: left-click targets that frame's unit; right-click
//! opens the matching unit menu. TargetFrame, FocusFrame and PetFrame menu actions live here.
//! The original Bevy `rendering/ui/unit_frames.rs` `UnitFrameClick` handled this path.
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
use crate::replicated::{UnitFields, is_unit};
use crate::ui::RegistryUi;

/// `UnitPopupRaidTarget<n>ButtonMixin:OnClick` → `SetRaidTargetIcon(unit, n)`.
const ACTION_UNIT_MENU_RAID_TARGET_PREFIX: &str = "unit_menu_raid_target:";
/// Retail UnitPopupAchievementButtonMixin opens SetAchievementComparison(unit).
const ACTION_COMPARE_ACHIEVEMENTS: &str = "unit_menu_compare_achievements";
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
    if unit.eq_ignore_ascii_case(local) {
        for (key, label, action) in [
            (
                "DungeonDifficulty",
                "Dungeon Difficulty",
                "unit_menu_dungeon_open",
            ),
            ("RaidDifficulty", "Raid Difficulty", "unit_menu_raid_open"),
        ] {
            items.push(UnitMenuItem {
                name: format!("UnitFrameContextMenu{key}"),
                label: label.into(),
                action: action.into(),
            });
        }
    }
    if !unit.eq_ignore_ascii_case(local) {
        items.push(UnitMenuItem {
            name: "UnitFrameContextMenuCompareAchievements".into(),
            label: "Compare Achievements".into(),
            action: ACTION_COMPARE_ACHIEVEMENTS.into(),
        });
        items.push(UnitMenuItem {
            name: "UnitFrameContextMenuTrade".into(),
            label: "Trade".into(),
            action: ACTION_UNIT_MENU_TRADE.into(),
        });
    }
    items
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

#[derive(Clone, Copy)]
struct UnitFrameClickTarget {
    rect: [f32; 4],
    unit: u64,
    from_focus_frame: bool,
}

fn clicked_unit_frame(point: Vector2, targets: &[UnitFrameClickTarget]) -> Option<(u64, bool)> {
    targets
        .iter()
        .find(|target| inside(target.rect, point))
        .map(|target| (target.unit, target.from_focus_frame))
}

fn apply_unit_frame_click(clicked_unit: Option<(u64, bool)>, selected: &mut Option<u64>) -> bool {
    let Some((unit, _)) = clicked_unit else {
        return false;
    };
    *selected = Some(unit);
    true
}

fn visible_frame_rect(ui: &RegistryUi, name: &str) -> Option<[f32; 4]> {
    let frame = ui.frame_control(name)?;
    if !frame.is_visible_in_tree() {
        return None;
    }
    ui.frame_rect(name).map(|(rect, _)| rect)
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

fn group_member_target_at<'a>(
    frames: &'a game_engine_ui_model::group_frames_component::GroupFramesState,
    compact: bool,
    point: Vector2,
    rect: impl Fn(&str) -> Option<[f32; 4]>,
    unit_for_name: impl FnOnce(&str) -> Option<u64>,
) -> Option<u64> {
    let member = group_member_at(frames, compact, point, rect)?;
    unit_for_name(member)
}

impl GameClient {
    /// Unit-frame left clicks select their unit. Right clicks open the unit menu; clicks
    /// outside an open menu close it and continue through normal input dispatch.
    pub(super) fn unit_menu_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if !button.is_pressed() || self.game_menu_ui.is_some() {
            return false;
        }
        let point = button.get_position();
        match button.get_button_index() {
            MouseButton::RIGHT => {
                let opened = self.open_unit_menu(point);
                if opened {
                    self.mark_unit_frame_click_handled();
                }
                opened
            }
            MouseButton::LEFT if self.pointer_over_unit_menu() => false,
            MouseButton::LEFT => {
                self.unit_menu = UnitMenu::default();
                let mut selected = self.targeting_target();
                let clicked_unit = self.unit_frame_click_target(point);
                if !apply_unit_frame_click(clicked_unit, &mut selected) {
                    return false;
                }
                self.set_target(selected);
                self.mark_unit_frame_click_handled();
                true
            }
            _ => false,
        }
    }

    fn open_unit_menu(&mut self, point: Vector2) -> bool {
        self.unit_menu = UnitMenu::default();
        if self.open_group_member_menu(point) {
            return true;
        }
        let Some((unit, from_focus_frame)) = self.unit_frame_click_target(point) else {
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
        let Some(ui) = self.group_frames.frame_ui().cloned() else {
            return false;
        };
        let frames = self.group_frames_view();
        let compact = game_engine_ui_model::hud_layout::active_layout_settings()
            .use_raid_style_party_frames
            .unwrap_or(true);
        let host = ui.bind();
        let member = group_member_at(&frames, compact, point, |name| {
            visible_frame_rect(&host, name)
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
    fn unit_frame_click_target(&self, point: Vector2) -> Option<(u64, bool)> {
        if let Some(unit) = self.group_frame_unit_at(point) {
            return Some((unit, false));
        }
        let ui = self.targeting.frame_ui()?.clone();
        let targets = self.unit_frame_click_targets(&ui.bind());
        clicked_unit_frame(point, &targets)
    }

    fn group_frame_unit_at(&self, point: Vector2) -> Option<u64> {
        let ui = self.group_frames.frame_ui()?.clone();
        let frames = self.group_frames_view();
        let compact = game_engine_ui_model::hud_layout::active_layout_settings()
            .use_raid_style_party_frames
            .unwrap_or(true);
        let host = ui.bind();
        group_member_target_at(
            &frames,
            compact,
            point,
            |name| visible_frame_rect(&host, name),
            |name| self.group_member_unit(name),
        )
    }

    fn unit_frame_click_targets(&self, ui: &RegistryUi) -> Vec<UnitFrameClickTarget> {
        let target = self
            .targeting_target()
            .filter(|id| self.replica.unit(*id).is_some_and(is_unit));
        let target_of_target = target
            .and_then(|id| self.replica.unit(id)?.unit_target())
            .filter(|id| self.replica.unit(*id).is_some_and(is_unit));
        let focus = self
            .targeting
            .focus
            .filter(|id| self.replica.unit(*id).is_some_and(is_unit));
        let player = self
            .world
            .local_player_id()
            .filter(|id| self.replica.unit(*id).is_some_and(is_unit));
        let pet = self.local_pet_id();
        [
            ("PetFrame", pet, false),
            ("TargetOfTargetFrame", target_of_target, false),
            ("FocusFrame", focus, true),
            ("TargetFrame", target, false),
            ("PlayerFrame", player, false),
        ]
        .into_iter()
        .filter_map(|(name, unit, from_focus_frame)| {
            let unit = unit?;
            let rect = visible_frame_rect(ui, name)?;
            Some(UnitFrameClickTarget {
                rect,
                unit,
                from_focus_frame,
            })
        })
        .collect()
    }

    fn mark_unit_frame_click_handled(&mut self) {
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }

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
            self.world.local_player_id(),
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
    fn open_difficulty_submenu(&mut self, raid: bool) {
        use game_engine_ui_model::inworld_unit_frames_component::{
            DifficultyMenuEntry, DifficultyMenuState,
        };
        if !self.unit_menu.state.visible {
            return;
        }
        let local = self
            .account
            .session
            .selected_character_name
            .as_deref()
            .unwrap_or("");
        let leader = !self.account.group.in_group() || self.account.group.is_leader(local);
        let outside = self.world_map_id.is_some_and(|id| {
            self.raid_info
                .catalog
                .loaded()
                .and_then(|c| c.as_ref().ok())
                .is_some_and(|c| !c.is_instance(id))
        });
        let enabled = leader && outside;
        let selected = self.account.raid_difficulty;
        let normalized = selected.map(|s| match s.legacy_difficulty_id {
            5 => 3,
            6 => 4,
            id => id,
        });
        let ids: &[(u32, &str)] = if raid {
            &[
                (14, "Normal"),
                (15, "Heroic"),
                (16, "Mythic"),
                (3, "10 Player"),
                (4, "25 Player"),
            ]
        } else {
            &[(1, "Normal"), (2, "Heroic"), (23, "Mythic")]
        };
        let entries = ids
            .iter()
            .map(|&(id, label)| DifficultyMenuEntry {
                difficulty_id: id,
                label: label.into(),
                checked: if raid {
                    if id <= 4 {
                        normalized == Some(id)
                    } else {
                        selected.is_some_and(|s| s.difficulty_id == id)
                    }
                } else {
                    self.account.dungeon_difficulty == Some(id)
                },
                enabled: enabled
                    && if raid {
                        selected.is_some()
                            && (id > 4 || selected.is_some_and(|s| s.difficulty_id != 16))
                    } else {
                        self.account.dungeon_difficulty.is_some()
                    },
            })
            .collect();
        let menu = DifficultyMenuState {
            x: self.unit_menu.state.x + UNIT_MENU_W,
            y: self.unit_menu.state.y,
            entries,
        };
        self.unit_menu.state.difficulty_menu = None;
        self.unit_menu.state.raid_difficulty_menu = None;
        if raid {
            self.unit_menu.state.raid_difficulty_menu = Some(menu)
        } else {
            self.unit_menu.state.difficulty_menu = Some(menu)
        }
    }

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
            raid_difficulty_menu: None,
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
                || current
                    .get_name()
                    .to_string()
                    .starts_with("UnitFrameRaidDifficultyMenu")
                || current
                    .get_name()
                    .to_string()
                    .starts_with("UnitFrameDifficultyMenu")
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
        if action == "unit_menu_raid_open" || action == "unit_menu_dungeon_open" {
            self.open_difficulty_submenu(action == "unit_menu_raid_open");
            return Ok(());
        }
        if let Some(id) = action.strip_prefix("unit_menu_raid_difficulty:") {
            let id: u32 = id
                .parse()
                .map_err(|e| SessionError(format!("Raid difficulty: {e}")))?;
            if self
                .unit_menu
                .state
                .raid_difficulty_menu
                .as_ref()
                .is_some_and(|m| m.entries.iter().any(|e| e.difficulty_id == id && e.enabled))
            {
                let selection = self
                    .account
                    .raid_difficulty
                    .ok_or_else(|| SessionError("Raid selection unavailable".into()))?;
                for request in game_engine_ui_model::raid_info::raid_selection_requests(
                    selection,
                    id,
                    id == 3 || id == 4,
                ) {
                    self.account
                        .send_raid_difficulty(request.difficulty_id, request.legacy)?;
                }
                self.unit_menu = UnitMenu::default();
            }
            return Ok(());
        }
        if let Some(id) = action.strip_prefix("unit_menu_dungeon_difficulty:") {
            let id: u32 = id
                .parse()
                .map_err(|e| SessionError(format!("Dungeon difficulty: {e}")))?;
            if self
                .unit_menu
                .state
                .difficulty_menu
                .as_ref()
                .is_some_and(|m| m.entries.iter().any(|e| e.difficulty_id == id && e.enabled))
            {
                self.account.send_set_dungeon_difficulty(id)?;
                self.unit_menu = UnitMenu::default();
            }
            return Ok(());
        }
        let menu = std::mem::take(&mut self.unit_menu);
        if !menu.state.visible {
            return Ok(());
        }
        if action == ACTION_COMPARE_ACHIEVEMENTS {
            let target = menu
                .unit
                .ok_or_else(|| SessionError("No comparison target selected".into()))?;
            let requests = self
                .account
                .achievements
                .action(&format!("achievement:compare:{target}"));
            for request in requests {
                self.account.send_achievement_request(request)?;
            }
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
                .map_or_else(GString::new, |error| {
                    GString::from(error.to_string().as_str())
                }),
            None => GString::from("Party member has no available passenger mount"),
        }
    }

    #[func]
    fn leave_vehicle(&self) -> GString {
        self.account
            .send_exit_vehicle()
            .err()
            .map_or_else(GString::new, |error| {
                GString::from(error.to_string().as_str())
            })
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
        let targets = [
            UnitFrameClickTarget {
                rect: [0.0, 0.0, 20.0, 20.0],
                unit: HOGGER,
                from_focus_frame: false,
            },
            UnitFrameClickTarget {
                rect: [30.0, 0.0, 20.0, 20.0],
                unit: WOLF,
                from_focus_frame: true,
            },
        ];
        let target_menu_unit = clicked_unit_frame(Vector2::new(10.0, 10.0), &targets);
        assert_eq!(target_menu_unit, Some((HOGGER, false)));
        let focus_menu_unit = clicked_unit_frame(Vector2::new(40.0, 10.0), &targets);
        assert_eq!(focus_menu_unit, Some((WOLF, true)));

        let target_items = menu_items(target_menu_unit.unwrap().1, Vec::new());
        assert_eq!(labels(&target_items)[0], "Set Focus");
        assert!(!labels(&target_items).contains(&"Clear Focus"));
        let focus_items = menu_items(focus_menu_unit.unwrap().1, Vec::new());
        assert_eq!(labels(&focus_items)[0], "Clear Focus");
        assert!(!labels(&focus_items).contains(&"Set Focus"));
        assert!(labels(&focus_items).contains(&"Skull"), "raid targets stay");
    }

    #[test]
    fn player_and_party_frame_clicks_target_the_displayed_unit() {
        const PLAYER: u64 = 101;
        const PARTY_MEMBER: u64 = 202;
        let player_frame = [UnitFrameClickTarget {
            rect: [0.0, 0.0, 100.0, 50.0],
            unit: PLAYER,
            from_focus_frame: false,
        }];
        let mut selected = Some(HOGGER);
        assert!(apply_unit_frame_click(
            clicked_unit_frame(Vector2::new(50.0, 25.0), &player_frame),
            &mut selected,
        ));
        assert_eq!(selected, Some(PLAYER));

        let mut frames = game_engine_ui_model::group_frames_component::GroupFramesState::default();
        frames.portrait_party.members.push(
            game_engine_ui_model::portrait_party_frame_component::PortraitPartyMemberView::named(
                "Ann",
            ),
        );
        let party_target = group_member_target_at(
            &frames,
            false,
            Vector2::new(160.0, 20.0),
            |name| (name == "PartyMemberFrame1").then_some([120.0, 0.0, 80.0, 40.0]),
            |name| (name == "Ann").then_some(PARTY_MEMBER),
        );
        selected = Some(PLAYER);
        assert!(apply_unit_frame_click(
            party_target.map(|unit| (unit, false)),
            &mut selected,
        ));
        assert_eq!(selected, Some(PARTY_MEMBER));
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
