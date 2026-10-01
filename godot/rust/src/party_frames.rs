//! Raid-style party and raid frames and the `PARTY_INVITE` popup (docs/specs/group-frames.md).
//!
//! The server owns the group: `GroupRosterSnapshot`, `GroupMemberStates` and the invite
//! messages fill the account's `GroupState` (`account.rs`), and these frames show it as the
//! root client's `scenes/group_frames` does. Accept on the popup sends
//! `RespondGroupInvite { accept: true }`; Decline and the 60 s timeout decline.

use std::time::Duration;

use game_engine_session::SessionScreen;
use game_engine_ui_model::aura_display_data::DebuffType;
use game_engine_ui_model::compact_unit_frame_component::{
    CompactDebuffView, CompactUnitView, UnitStatus,
};
use game_engine_ui_model::damage_meter_data::class_color;
use game_engine_ui_model::group_frames_component::{GroupFramesState, RAID_GROUPS};
use game_engine_ui_model::group_state::{GroupCommand, GroupState};
use game_engine_ui_model::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine_ui_model::static_popup_component::{StaticPopupState, parse_popup_action};
use godot::prelude::*;
use shared::components::{Player, PowerType};
use shared::death::DeathState;
use shared::protocol::{GROUP_INVITE_TIMEOUT_SECS, GroupMemberSnapshot};

use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};

/// `CompactUnitFrame_UpdateInRange`: members beyond 40 yd fade.
const GROUP_FRAME_RANGE: f32 = 40.0;
const PARTY_INVITE_POPUP: &str = "PARTY_INVITE";

#[derive(Default)]
pub(crate) struct GroupFramesHud {
    frames: Option<Gd<RegistryUi>>,
    popups_ui: Option<Gd<RegistryUi>>,
    pub(crate) popups: PopupStack,
}

impl GroupFramesHud {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.frames, &mut self.popups_ui]
            .into_iter()
            .flatten()
        {
            visit(ui)?;
        }
        Ok(())
    }

    fn close(&mut self) {
        for ui in [self.frames.take(), self.popups_ui.take()]
            .into_iter()
            .flatten()
        {
            ui.free();
        }
        self.popups.clear();
    }
}

/// Who the frames are seen by: the player's name and the current target's.
pub(crate) struct GroupViewer<'a> {
    pub local_name: Option<&'a str>,
    pub target_name: Option<&'a str>,
}

/// Party mode lists the player first, then the others in roster order (`CRFSort_Group`);
/// raid mode fills each subgroup column in roster order.
pub(crate) fn group_frames_state(
    group: &GroupState,
    viewer: &GroupViewer,
    icon_fdid: &dyn Fn(u32) -> u32,
) -> GroupFramesState {
    let mut state = GroupFramesState {
        raid: vec![Vec::new(); RAID_GROUPS],
        ..Default::default()
    };
    if group.is_raid {
        for member in &group.members {
            let index = usize::from(member.subgroup.clamp(1, RAID_GROUPS as u8) - 1);
            state.raid[index].push(member_view(group, viewer, member, icon_fdid));
        }
        return state;
    }
    let mut ordered: Vec<&GroupMemberSnapshot> = group.members.iter().collect();
    ordered.sort_by_key(|member| Some(member.name.as_str()) != viewer.local_name);
    state.party = ordered
        .into_iter()
        .map(|member| member_view(group, viewer, member, icon_fdid))
        .collect();
    state
}

/// One member's frame from the roster and its live state; before the first live state
/// the bars are empty, as nothing is invented.
fn member_view(
    group: &GroupState,
    viewer: &GroupViewer,
    member: &GroupMemberSnapshot,
    icon_fdid: &dyn Fn(u32) -> u32,
) -> CompactUnitView {
    let live = group.live.get(&member.name).filter(|_| member.online);
    let status = match (member.online, live.map(|l| l.death)) {
        (false, _) => UnitStatus::Offline,
        (true, Some(DeathState::Dead | DeathState::Ghost)) => UnitStatus::Dead,
        _ => UnitStatus::Online,
    };
    CompactUnitView {
        name: member.name.clone(),
        class_rgb: class_color(member.class),
        health_fraction: live.map(|l| fraction(i64::from(l.health), i64::from(l.max_health))),
        power: live.and_then(|l| l.power.as_ref()).map(|power| {
            (
                fraction(i64::from(power.current), i64::from(power.max)),
                power_bar_rgb(power.power),
            )
        }),
        role: member.role,
        status,
        in_range: in_range(group, viewer, member),
        selected: viewer.target_name == Some(member.name.as_str()),
        ready: group.ready_mark(&member.name),
        debuffs: live
            .map(|l| {
                l.debuffs
                    .iter()
                    .map(|aura| CompactDebuffView {
                        icon_fdid: icon_fdid(aura.spell_id),
                        dispel: DebuffType::from_dispel_id(aura.dispel_type),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Offline members and members without a known position are out of range; the player is
/// always in range.
fn in_range(group: &GroupState, viewer: &GroupViewer, member: &GroupMemberSnapshot) -> bool {
    if viewer.local_name == Some(member.name.as_str()) {
        return true;
    }
    let (Some(local), Some(other)) = (
        viewer.local_name.and_then(|name| group.live.get(name)),
        group.live.get(&member.name).filter(|_| member.online),
    ) else {
        return false;
    };
    let (a, b) = (local.position, other.position);
    let d2 = (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2);
    d2 <= GROUP_FRAME_RANGE.powi(2)
}

fn fraction(current: i64, max: i64) -> f32 {
    if max <= 0 {
        return 0.0;
    }
    (current as f32 / max as f32).clamp(0.0, 1.0)
}

/// `PowerBarColor` (PowerBarColorUtil.lua:18-33); others take mana's colour as Retail
/// does for tokens without an entry (CompactUnitFrame.lua:780-786).
fn power_bar_rgb(power: PowerType) -> [f32; 3] {
    match power {
        PowerType::Rage => [1.0, 0.0, 0.0],
        PowerType::Focus => [1.0, 0.5, 0.25],
        PowerType::Energy => [1.0, 1.0, 0.0],
        PowerType::RunicPower => [0.0, 0.82, 1.0],
        PowerType::LunarPower => [0.30, 0.52, 0.90],
        PowerType::Maelstrom => [0.0, 0.5, 1.0],
        PowerType::Insanity => [0.40, 0.0, 0.80],
        PowerType::Fury => [0.788, 0.259, 0.992],
        PowerType::Pain => [1.0, 156.0 / 255.0, 0.0],
        _ => [0.0, 0.0, 1.0],
    }
}

/// `PARTY_INVITE`: "%s invites you to a group." (`INVITATION`), Accept / Decline, 60 s.
fn invite_popup_spec(inviter: &str) -> PopupSpec {
    PopupSpec {
        key: PARTY_INVITE_POPUP.into(),
        text: format!("{inviter} invites you to a group."),
        accept_label: "Accept".into(),
        cancel_label: Some("Decline".into()),
        timeout: Some(Duration::from_secs_f32(GROUP_INVITE_TIMEOUT_SECS)),
        confirm_text: None,
    }
}

/// Show the pending invite's popup, or hide it once the server cancels the invite.
fn sync_invite_popup(group: &GroupState, popups: &mut PopupStack) {
    match &group.pending_invite {
        Some(inviter) if !popups.contains(PARTY_INVITE_POPUP) => {
            popups.push(invite_popup_spec(inviter));
        }
        None if popups.contains(PARTY_INVITE_POPUP) => {
            popups.hide(PARTY_INVITE_POPUP);
        }
        _ => {}
    }
}

/// Answers of closed invite popups: Accept joins; Decline and the timeout decline, as
/// Retail's `PARTY_INVITE` `OnHide` does.
fn invite_answers(results: &[PopupResult]) -> Vec<bool> {
    results
        .iter()
        .filter(|result| result.key == PARTY_INVITE_POPUP)
        .map(|result| result.outcome == PopupOutcome::Accepted)
        .collect()
}

/// Original DELETE_GOOD_ITEM editbox limit, measured in Unicode characters.
const POPUP_EDITBOX_LETTERS: usize = 32;

fn type_popup_key(
    entry: &mut game_engine_ui_model::popup::PopupEntry,
    key: &godot::classes::InputEventKey,
) {
    if key.get_keycode() == godot::global::Key::BACKSPACE {
        entry.typed.pop();
    } else if let Some(character) = char::from_u32(key.get_unicode()) {
        let below_limit = entry.typed.chars().count() < POPUP_EDITBOX_LETTERS;
        if !character.is_control() && below_limit {
            entry.typed.push(character);
        }
    }
}

impl GameClient {
    /// Popup keyboard ownership precedes chat and gameplay bindings.
    pub(super) fn static_popup_key(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        let Ok(key) = event.clone().try_cast::<godot::classes::InputEventKey>() else {
            return false;
        };
        if !key.is_pressed() || !self.group_frames.popups.is_open() {
            return false;
        }
        let stack = &mut self.group_frames.popups;
        match key.get_keycode() {
            godot::global::Key::ESCAPE => stack.cancel_top(),
            godot::global::Key::ENTER | godot::global::Key::KP_ENTER => stack.accept_top(),
            _ => {
                let Some(entry) = stack.typing_target() else {
                    return false;
                };
                type_popup_key(entry, &key);
            }
        }
        true
    }

    /// Per frame in the world: the invite popup's clicks and timeout, then the frames.
    pub(super) fn update_group_frames(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.group_frames.close();
            return Ok(());
        }
        self.poll_invite_popup_actions()?;
        let hud = &mut self.group_frames;
        hud.popups.tick(Duration::from_secs_f32(delta.max(0.0)));
        let results = hud.popups.drain_results();
        self.dispatch_bag_destroy_results(&results)?;
        self.dispatch_quest_abandon_results(&results)?;
        self.hide_stale_bag_destroy_popups();
        for accept in invite_answers(&results) {
            self.account.group.pending_invite = None;
            self.account
                .send_group(GroupCommand::RespondInvite(accept))?;
        }
        sync_invite_popup(&self.account.group, &mut self.group_frames.popups);
        let popups = StaticPopupState {
            popups: self.group_frames.popups.visible(),
        };
        let frames = self.group_frames_view();
        self.show_group_ui(frames, popups)?;
        Ok(())
    }

    fn group_frames_view(&self) -> GroupFramesState {
        let target_name = self
            .targeting_target()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<Player>())
            .map(|player| player.name.clone());
        let local_name = self.account.session.selected_character_name.clone();
        let viewer = GroupViewer {
            local_name: local_name.as_deref(),
            target_name: target_name.as_deref(),
        };
        let catalog = self.spells.catalog();
        let icon = |spell_id: u32| {
            catalog
                .and_then(|data| data.get(spell_id))
                .map_or(0, |spell| spell.icon_fdid)
        };
        group_frames_state(&self.account.group, &viewer, &icon)
    }

    fn show_group_ui(
        &mut self,
        frames: GroupFramesState,
        popups: StaticPopupState,
    ) -> Result<(), String> {
        if let Some(ui) = self.group_frames.frames.as_mut() {
            ui.bind_mut().set_state(frames)?;
        } else {
            let ui = self.attach_group_ui("GroupFramesUI", |ui| ui.show_group_frames(frames))?;
            self.group_frames.frames = Some(ui);
        }
        if let Some(ui) = self.group_frames.popups_ui.as_mut() {
            ui.bind_mut().set_state(popups)?;
        } else {
            let ui = self.attach_group_ui("StaticPopupUI", |ui| ui.show_static_popups(popups))?;
            self.group_frames.popups_ui = Some(ui);
        }
        Ok(())
    }

    fn attach_group_ui(
        &mut self,
        name: &str,
        show: impl FnOnce(&mut RegistryUi) -> Result<(), String>,
    ) -> Result<Gd<RegistryUi>, String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(name);
        self.base_mut().add_child(&ui);
        let shown = show(&mut ui.bind_mut());
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        Ok(ui)
    }

    /// Accept / Decline clicks on the popups.
    fn poll_invite_popup_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.group_frames.popups_ui.as_mut() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return Ok(());
            }
            let (id, outcome) = parse_popup_action(&action)
                .ok_or_else(|| format!("Unknown popup action: {action}"))?;
            self.group_frames.popups.resolve(id, outcome);
        }
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Group state for automation: roster, the pending invite and the party frames' fills.
    #[func]
    fn group_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let group = &self.account.group;
        state.set("in_group", group.in_group());
        state.set("is_raid", group.is_raid);
        state.set(
            "pending_invite",
            group.pending_invite.clone().unwrap_or_default().as_str(),
        );
        let frames = self.group_frames_view();
        let party: Array<VarDictionary> = frames
            .party
            .iter()
            .map(|view| {
                let mut member = VarDictionary::new();
                member.set("name", view.name.as_str());
                member.set("health", view.health_fraction.unwrap_or(-1.0));
                member.set("power", view.power.map_or(-1.0, |(fill, _)| fill));
                member.set("status", format!("{:?}", view.status).as_str());
                member.set("selected", view.selected);
                member.set("in_range", view.in_range);
                member
            })
            .collect();
        state.set("party", &party);
        state.set("popups", self.group_frames.popups.visible().len() as i64);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::{AuraView, Position, PowerEntry};
    use shared::protocol::{GroupMemberState, GroupRoleSnapshot, GroupRosterSnapshot};

    fn member(name: &str, class: u8, online: bool) -> GroupMemberSnapshot {
        GroupMemberSnapshot {
            name: name.into(),
            role: GroupRoleSnapshot::None,
            is_leader: name == "Ann",
            online,
            subgroup: 1,
            class,
            level: 10,
            entity: None,
        }
    }

    fn live(name: &str, health: u32, death: DeathState, x: f32) -> GroupMemberState {
        GroupMemberState {
            name: name.into(),
            health,
            max_health: 400,
            power: Some(PowerEntry {
                power: PowerType::Mana,
                current: 250,
                max: 1000,
                partial: 0,
                regen_per_sec: 0.0,
            }),
            death,
            position: Position { x, y: 0.0, z: 0.0 },
            debuffs: Vec::<AuraView>::new(),
        }
    }

    /// Ann leads Bob (the player), Cid (a ghost 90 yd away) and Dee (offline): Bob's own
    /// frame comes first, each shows its live health and mana, Cid shows Dead and faded,
    /// Dee Offline with empty bars, and the target is highlighted.
    #[test]
    fn party_frames_show_the_player_first_with_live_bars_status_and_range() {
        let mut group = GroupState::default();
        group.apply_roster(GroupRosterSnapshot {
            is_raid: false,
            ready_count: 0,
            total_count: 4,
            members: vec![
                member("Ann", 1, true),
                member("Bob", 8, true),
                member("Cid", 5, true),
                member("Dee", 4, false),
            ],
            loot_method: shared::loot::LootMode::PersonalLoot,
        });
        group.apply_member_states(vec![
            live("Ann", 300, DeathState::Alive, 10.0),
            live("Bob", 400, DeathState::Alive, 0.0),
            live("Cid", 0, DeathState::Ghost, 90.0),
        ]);
        let viewer = GroupViewer {
            local_name: Some("Bob"),
            target_name: Some("Ann"),
        };
        let state = group_frames_state(&group, &viewer, &|_| 0);
        let names: Vec<_> = state.party.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["Bob", "Ann", "Cid", "Dee"]);
        let [bob, ann, cid, dee] = &state.party[..] else {
            panic!("four frames");
        };
        assert_eq!(bob.health_fraction, Some(1.0));
        assert_eq!(ann.health_fraction, Some(0.75));
        assert_eq!(ann.power, Some((0.25, [0.0, 0.0, 1.0])));
        assert_eq!(bob.class_rgb, [0.25, 0.78, 0.92], "mage RAID_CLASS_COLORS");
        assert!(ann.selected && !bob.selected);
        assert!(ann.in_range);
        assert_eq!(cid.status, UnitStatus::Dead);
        assert!(!cid.in_range, "90 yd away");
        assert_eq!(dee.status, UnitStatus::Offline);
        assert_eq!((dee.health_fraction, dee.power), (None, None));
        assert!(state.raid.iter().all(Vec::is_empty));

        // Leaving empties the roster and removes the frames.
        group.apply_roster(GroupRosterSnapshot {
            is_raid: false,
            ready_count: 0,
            total_count: 0,
            members: Vec::new(),
            loot_method: shared::loot::LootMode::PersonalLoot,
        });
        assert!(group_frames_state(&group, &viewer, &|_| 0).party.is_empty());
    }

    /// A pending invite opens `PARTY_INVITE`; Accept answers true once, and a server
    /// cancel closes a popup without an answer.
    #[test]
    fn invite_popup_answers_accept_and_closes_on_cancel() {
        let mut group = GroupState {
            pending_invite: Some("Ann".into()),
            ..Default::default()
        };
        let mut popups = PopupStack::default();
        sync_invite_popup(&group, &mut popups);
        let shown = popups.visible();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].spec.text, "Ann invites you to a group.");
        popups.resolve(shown[0].id, PopupOutcome::Accepted);
        assert_eq!(invite_answers(&popups.drain_results()), [true]);
        assert!(invite_answers(&popups.drain_results()).is_empty());

        sync_invite_popup(&group, &mut popups);
        group.pending_invite = None;
        sync_invite_popup(&group, &mut popups);
        assert!(popups.visible().is_empty());
        assert!(invite_answers(&popups.drain_results()).is_empty());

        group.pending_invite = Some("Ann".into());
        sync_invite_popup(&group, &mut popups);
        popups.tick(Duration::from_secs_f32(GROUP_INVITE_TIMEOUT_SECS));
        assert_eq!(
            invite_answers(&popups.drain_results()),
            [false],
            "the timeout declines"
        );
    }
}
