use bevy::prelude::*;

pub mod textures {
    /// Health bar fill (shared with casting bar).
    pub const HEALTH_BAR_FILL: u32 = 4505182;

    // --- Role icons (LFG role atlas) ---
    /// LFG role icon sheet (tank/healer/DPS in one atlas).
    pub const LFG_ROLE_ICONS: u32 = 337499;
    /// Role icons sheet (modern).
    pub const ROLE_ICONS: u32 = 2134184;
    /// LFG role icon sheet (original).
    pub const LFG_ROLE: u32 = 252190;

    // --- Ready check icons ---
    /// Ready check: accepted (green checkmark).
    pub const READY_CHECK_OK: u32 = 136814;
    /// Ready check: not ready (red X).
    pub const READY_CHECK_FAIL: u32 = 136813;
    /// Ready check: pending (yellow ?).
    pub const READY_CHECK_WAIT: u32 = 136815;
    /// Ready check frame background.
    pub const READY_CHECK_FRAME: u32 = 136825;

    // --- Debuff ---
    /// Debuff border overlay (colored by type: magic/curse/poison/disease).
    pub const DEBUFF_BORDER: u32 = 130758;
    /// Debuff type color overlays.
    pub const DEBUFF_OVERLAYS: u32 = 130759;
}

// --- Role ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GroupRole {
    #[default]
    Dps,
    Tank,
    Healer,
}

impl GroupRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dps => "D",
            Self::Tank => "T",
            Self::Healer => "H",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Dps => "Damage",
            Self::Tank => "Tank",
            Self::Healer => "Healer",
        }
    }
}

// --- Ready check ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReadyCheck {
    #[default]
    None,
    Pending,
    Accepted,
    Declined,
}

impl ReadyCheck {
    pub fn is_active(self) -> bool {
        self != Self::None
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Pending => "?",
            Self::Accepted => "✓",
            Self::Declined => "✗",
        }
    }
}

// --- Power type ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PowerType {
    #[default]
    Mana,
    Rage,
    Energy,
    Focus,
    RunicPower,
}

impl PowerType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mana => "Mana",
            Self::Rage => "Rage",
            Self::Energy => "Energy",
            Self::Focus => "Focus",
            Self::RunicPower => "Runic Power",
        }
    }
}

// --- Debuff ---

#[derive(Clone, Debug, PartialEq)]
pub struct UnitDebuff {
    pub name: String,
    pub icon_fdid: u32,
    pub stacks: u32,
    pub remaining_secs: f32,
}

impl UnitDebuff {
    pub fn has_stacks(&self) -> bool {
        self.stacks > 1
    }

    pub fn time_text(&self) -> String {
        let secs = self.remaining_secs as u32;
        if secs >= 60 {
            format!("{}m", secs / 60)
        } else {
            format!("{secs}s")
        }
    }
}

// --- Unit state ---

#[derive(Clone, Debug, PartialEq)]
pub struct GroupUnitState {
    pub name: String,
    pub health_current: u32,
    pub health_max: u32,
    pub power_current: u32,
    pub power_max: u32,
    pub power_type: PowerType,
    pub role: GroupRole,
    pub debuffs: Vec<UnitDebuff>,
    pub in_range: bool,
    pub alive: bool,
    pub online: bool,
    pub ready_check: ReadyCheck,
    /// Incoming heals as raw HP amount.
    pub incoming_heals: u32,
}

impl GroupUnitState {
    pub fn health_fraction(&self) -> f32 {
        if self.health_max == 0 {
            return 0.0;
        }
        (self.health_current as f32 / self.health_max as f32).min(1.0)
    }

    pub fn power_fraction(&self) -> f32 {
        if self.power_max == 0 {
            return 0.0;
        }
        (self.power_current as f32 / self.power_max as f32).min(1.0)
    }

    pub fn incoming_heals_fraction(&self) -> f32 {
        if self.health_max == 0 {
            return 0.0;
        }
        let remaining = self.health_max.saturating_sub(self.health_current);
        let capped = self.incoming_heals.min(remaining);
        capped as f32 / self.health_max as f32
    }

    pub fn health_text(&self) -> String {
        format!("{}/{}", self.health_current, self.health_max)
    }

    pub fn is_dead(&self) -> bool {
        self.health_current == 0 && self.health_max > 0
    }
}

// --- Loot distribution ---

/// How loot is distributed among group members.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LootMethod {
    /// Anyone can loot any corpse.
    FreeForAll,
    /// Members take turns looting.
    RoundRobin,
    /// Leader assigns loot manually.
    MasterLooter,
    /// Roll on items above threshold (need/greed/pass).
    #[default]
    GroupLoot,
    /// Need before greed rolls on items above threshold.
    NeedBeforeGreed,
    /// Client-side personal loot presentation.
    PersonalLoot,
}

impl LootMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::FreeForAll => "Free For All",
            Self::RoundRobin => "Round Robin",
            Self::MasterLooter => "Master Looter",
            Self::GroupLoot => "Group Loot",
            Self::NeedBeforeGreed => "Need Before Greed",
            Self::PersonalLoot => "Personal Loot",
        }
    }
}

/// Minimum item quality that triggers the loot roll popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LootThreshold {
    /// Gray (poor) — rolls on everything.
    Poor,
    /// White (common).
    Common,
    /// Green (uncommon) — WoW default.
    #[default]
    Uncommon,
    /// Blue (rare).
    Rare,
    /// Purple (epic).
    Epic,
    /// Orange (legendary).
    Legendary,
}

impl LootThreshold {
    pub fn label(self) -> &'static str {
        match self {
            Self::Poor => "Poor",
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }

    /// WoW quality ID (0–5).
    pub fn quality_id(self) -> u8 {
        match self {
            Self::Poor => 0,
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 3,
            Self::Epic => 4,
            Self::Legendary => 5,
        }
    }
}

/// Active loot distribution settings for the group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LootSettings {
    pub method: LootMethod,
    pub threshold: LootThreshold,
}

// --- Group ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RaidGroupData {
    pub members: Vec<GroupUnitState>,
}

// --- Runtime resources ---

/// Party state: up to 4 members (excluding self).
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct PartyState {
    pub members: Vec<GroupUnitState>,
    pub ready_check_active: bool,
    pub loot: LootSettings,
}

impl PartyState {
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn all_ready(&self) -> bool {
        self.members
            .iter()
            .all(|m| m.ready_check == ReadyCheck::Accepted)
    }

    /// Begin a new ready check, setting all members to Pending.
    pub fn start_ready_check(&mut self) {
        self.ready_check_active = true;
        for m in &mut self.members {
            m.ready_check = ReadyCheck::Pending;
        }
    }

    /// Record a member's ready check response by name.
    pub fn respond_ready_check(&mut self, name: &str, response: ReadyCheck) {
        if let Some(m) = self.members.iter_mut().find(|m| m.name == name) {
            m.ready_check = response;
        }
    }

    /// End the ready check, resetting all member states.
    pub fn finish_ready_check(&mut self) {
        self.ready_check_active = false;
        for m in &mut self.members {
            m.ready_check = ReadyCheck::None;
        }
    }

    /// Assign a role to a member by name. Returns true if found.
    pub fn assign_role(&mut self, name: &str, role: GroupRole) -> bool {
        if let Some(m) = self.members.iter_mut().find(|m| m.name == name) {
            m.role = role;
            true
        } else {
            false
        }
    }

    /// Count members with a specific role.
    pub fn role_count(&self, role: GroupRole) -> usize {
        self.members.iter().filter(|m| m.role == role).count()
    }

    /// True when ready check is active and all have responded (no Pending).
    pub fn all_responded(&self) -> bool {
        self.ready_check_active
            && self
                .members
                .iter()
                .all(|m| m.ready_check != ReadyCheck::Pending)
    }
}

/// Raid state: up to 8 groups of 5.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct RaidState {
    pub groups: Vec<RaidGroupData>,
    pub ready_check_active: bool,
}

impl RaidState {
    pub fn total_members(&self) -> usize {
        self.groups.iter().map(|g| g.members.len()).sum()
    }

    pub fn alive_count(&self) -> usize {
        self.groups
            .iter()
            .flat_map(|g| &g.members)
            .filter(|m| m.alive)
            .count()
    }

    pub fn all_ready(&self) -> bool {
        self.groups
            .iter()
            .flat_map(|g| &g.members)
            .all(|m| m.ready_check == ReadyCheck::Accepted)
    }
}

// --- Ready check popup state ---

/// Ready check lifecycle state shown as a popup to the local player.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct ReadyCheckState {
    /// Whether a ready check is currently in progress.
    pub active: bool,
    /// Name of the player who initiated the check.
    pub initiator: String,
    /// Seconds remaining before the check times out.
    pub remaining_secs: f32,
    /// The local player's response.
    pub local_response: ReadyCheck,
}

/// Ready check timeout in seconds (WoW default).
pub const READY_CHECK_TIMEOUT_SECS: f32 = 30.0;

impl ReadyCheckState {
    /// Begin a new ready check from the given initiator.
    pub fn start(&mut self, initiator: String) {
        self.active = true;
        self.initiator = initiator;
        self.remaining_secs = READY_CHECK_TIMEOUT_SECS;
        self.local_response = ReadyCheck::Pending;
    }

    /// Record the local player's response.
    pub fn respond(&mut self, response: ReadyCheck) {
        self.local_response = response;
    }

    /// Whether the local player still needs to respond.
    pub fn awaiting_response(&self) -> bool {
        self.active && self.local_response == ReadyCheck::Pending
    }

    /// End the ready check (timeout or all responded).
    pub fn finish(&mut self) {
        self.active = false;
        self.remaining_secs = 0.0;
    }
}

// --- Client → server intents ---

/// A pending group/party action to send to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupIntent {
    /// Leader initiates a ready check.
    InitiateReadyCheck,
    /// Local player accepts the ready check.
    AcceptReadyCheck,
    /// Local player declines the ready check.
    DeclineReadyCheck,
    /// Assign a role to a party/raid member.
    SetRole {
        player_name: String,
        role: GroupRole,
    },
    /// Set the local player's own role.
    SetOwnRole { role: GroupRole },
    /// Change the group loot distribution method (leader only).
    SetLootMethod { method: LootMethod },
    /// Change the loot quality threshold (leader only).
    SetLootThreshold { threshold: LootThreshold },
}

/// Queue of group intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct GroupIntentQueue {
    pub pending: Vec<GroupIntent>,
}

impl GroupIntentQueue {
    pub fn initiate_ready_check(&mut self) {
        self.pending.push(GroupIntent::InitiateReadyCheck);
    }

    pub fn accept_ready_check(&mut self) {
        self.pending.push(GroupIntent::AcceptReadyCheck);
    }

    pub fn decline_ready_check(&mut self) {
        self.pending.push(GroupIntent::DeclineReadyCheck);
    }

    pub fn set_role(&mut self, player_name: String, role: GroupRole) {
        self.pending
            .push(GroupIntent::SetRole { player_name, role });
    }

    pub fn set_own_role(&mut self, role: GroupRole) {
        self.pending.push(GroupIntent::SetOwnRole { role });
    }

    pub fn set_loot_method(&mut self, method: LootMethod) {
        self.pending.push(GroupIntent::SetLootMethod { method });
    }

    pub fn set_loot_threshold(&mut self, threshold: LootThreshold) {
        self.pending
            .push(GroupIntent::SetLootThreshold { threshold });
    }

    pub fn drain(&mut self) -> Vec<GroupIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(hp: u32, max: u32) -> GroupUnitState {
        GroupUnitState {
            name: "Test".into(),
            health_current: hp,
            health_max: max,
            power_current: 0,
            power_max: 0,
            power_type: PowerType::Mana,
            role: GroupRole::Dps,
            debuffs: vec![],
            in_range: true,
            alive: hp > 0,
            online: true,
            ready_check: ReadyCheck::None,
            incoming_heals: 0,
        }
    }

    // --- GroupRole ---

    #[test]
    fn role_labels() {
        assert_eq!(GroupRole::Tank.label(), "T");
        assert_eq!(GroupRole::Healer.label(), "H");
        assert_eq!(GroupRole::Dps.label(), "D");
    }

    #[test]
    fn role_display_names() {
        assert_eq!(GroupRole::Tank.display_name(), "Tank");
        assert_eq!(GroupRole::Healer.display_name(), "Healer");
        assert_eq!(GroupRole::Dps.display_name(), "Damage");
    }

    // --- ReadyCheck ---

    #[test]
    fn ready_check_active() {
        assert!(!ReadyCheck::None.is_active());
        assert!(ReadyCheck::Pending.is_active());
        assert!(ReadyCheck::Accepted.is_active());
        assert!(ReadyCheck::Declined.is_active());
    }

    #[test]
    fn ready_check_symbols() {
        assert_eq!(ReadyCheck::Accepted.symbol(), "✓");
        assert_eq!(ReadyCheck::Pending.symbol(), "?");
        assert_eq!(ReadyCheck::Declined.symbol(), "✗");
        assert_eq!(ReadyCheck::None.symbol(), "");
    }

    // --- PowerType ---

    #[test]
    fn power_type_labels() {
        assert_eq!(PowerType::Mana.label(), "Mana");
        assert_eq!(PowerType::Rage.label(), "Rage");
        assert_eq!(PowerType::Energy.label(), "Energy");
        assert_eq!(PowerType::RunicPower.label(), "Runic Power");
    }

    // --- UnitDebuff ---

    #[test]
    fn debuff_stacks() {
        let d = UnitDebuff {
            name: "Bleed".into(),
            icon_fdid: 1,
            stacks: 3,
            remaining_secs: 10.0,
        };
        assert!(d.has_stacks());

        let single = UnitDebuff {
            stacks: 1,
            ..d.clone()
        };
        assert!(!single.has_stacks());
    }

    #[test]
    fn debuff_time_text() {
        let short = UnitDebuff {
            name: "X".into(),
            icon_fdid: 1,
            stacks: 1,
            remaining_secs: 45.0,
        };
        assert_eq!(short.time_text(), "45s");

        let long = UnitDebuff {
            remaining_secs: 125.0,
            ..short
        };
        assert_eq!(long.time_text(), "2m");
    }

    // --- GroupUnitState ---

    #[test]
    fn health_fraction() {
        assert!((unit(90, 100).health_fraction() - 0.9).abs() < 0.01);
        assert_eq!(unit(0, 0).health_fraction(), 0.0);
        assert!((unit(100, 100).health_fraction() - 1.0).abs() < 0.01);
    }

    #[test]
    fn power_fraction() {
        let u = GroupUnitState {
            power_current: 50,
            power_max: 200,
            ..unit(100, 100)
        };
        assert!((u.power_fraction() - 0.25).abs() < 0.01);

        assert_eq!(unit(100, 100).power_fraction(), 0.0);
    }

    #[test]
    fn incoming_heals_fraction() {
        let u = GroupUnitState {
            health_current: 70,
            health_max: 100,
            incoming_heals: 20,
            ..unit(70, 100)
        };
        assert!((u.incoming_heals_fraction() - 0.2).abs() < 0.01);

        // Capped at remaining health
        let over = GroupUnitState {
            incoming_heals: 50,
            ..u
        };
        assert!((over.incoming_heals_fraction() - 0.3).abs() < 0.01);

        // Full health — no incoming shown
        let full = GroupUnitState {
            health_current: 100,
            incoming_heals: 10,
            ..unit(100, 100)
        };
        assert_eq!(full.incoming_heals_fraction(), 0.0);
    }

    #[test]
    fn health_text_format() {
        assert_eq!(unit(450, 1000).health_text(), "450/1000");
    }

    #[test]
    fn is_dead() {
        assert!(unit(0, 100).is_dead());
        assert!(!unit(1, 100).is_dead());
        assert!(!unit(0, 0).is_dead()); // zero max = not dead, just no HP pool
    }

    // --- PartyState ---

    #[test]
    fn party_member_count() {
        let state = PartyState {
            members: vec![unit(100, 100), unit(50, 100)],
            ..Default::default()
        };
        assert_eq!(state.member_count(), 2);
    }

    #[test]
    fn party_all_ready() {
        let ready = PartyState {
            members: vec![
                GroupUnitState {
                    ready_check: ReadyCheck::Accepted,
                    ..unit(100, 100)
                },
                GroupUnitState {
                    ready_check: ReadyCheck::Accepted,
                    ..unit(100, 100)
                },
            ],
            ready_check_active: true,
            ..Default::default()
        };
        assert!(ready.all_ready());

        let not_ready = PartyState {
            members: vec![
                GroupUnitState {
                    ready_check: ReadyCheck::Accepted,
                    ..unit(100, 100)
                },
                GroupUnitState {
                    ready_check: ReadyCheck::Pending,
                    ..unit(100, 100)
                },
            ],
            ready_check_active: true,
            ..Default::default()
        };
        assert!(!not_ready.all_ready());
    }

    // --- RaidState ---

    #[test]
    fn raid_total_members() {
        let state = RaidState {
            groups: vec![
                RaidGroupData {
                    members: vec![unit(100, 100), unit(100, 100)],
                },
                RaidGroupData {
                    members: vec![unit(100, 100)],
                },
            ],
            ..Default::default()
        };
        assert_eq!(state.total_members(), 3);
    }

    #[test]
    fn raid_alive_count() {
        let state = RaidState {
            groups: vec![RaidGroupData {
                members: vec![unit(100, 100), unit(0, 100), unit(50, 100)],
            }],
            ..Default::default()
        };
        assert_eq!(state.alive_count(), 2);
    }

    #[test]
    fn raid_all_ready() {
        let state = RaidState {
            groups: vec![RaidGroupData {
                members: vec![GroupUnitState {
                    ready_check: ReadyCheck::Accepted,
                    ..unit(100, 100)
                }],
            }],
            ready_check_active: true,
        };
        assert!(state.all_ready());
    }

    // --- PartyState ready check lifecycle ---

    fn named_unit(name: &str) -> GroupUnitState {
        GroupUnitState {
            name: name.into(),
            ..unit(100, 100)
        }
    }

    #[test]
    fn party_start_ready_check() {
        let mut state = PartyState {
            members: vec![named_unit("Alice"), named_unit("Bob")],
            ..Default::default()
        };
        state.start_ready_check();
        assert!(state.ready_check_active);
        assert_eq!(state.members[0].ready_check, ReadyCheck::Pending);
        assert_eq!(state.members[1].ready_check, ReadyCheck::Pending);
    }

    #[test]
    fn party_respond_ready_check() {
        let mut state = PartyState {
            members: vec![named_unit("Alice"), named_unit("Bob")],
            ..Default::default()
        };
        state.start_ready_check();
        state.respond_ready_check("Alice", ReadyCheck::Accepted);
        assert_eq!(state.members[0].ready_check, ReadyCheck::Accepted);
        assert_eq!(state.members[1].ready_check, ReadyCheck::Pending);
        assert!(!state.all_responded());

        state.respond_ready_check("Bob", ReadyCheck::Declined);
        assert!(state.all_responded());
    }

    #[test]
    fn party_finish_ready_check() {
        let mut state = PartyState {
            members: vec![named_unit("Alice")],
            ..Default::default()
        };
        state.start_ready_check();
        state.respond_ready_check("Alice", ReadyCheck::Accepted);
        state.finish_ready_check();
        assert!(!state.ready_check_active);
        assert_eq!(state.members[0].ready_check, ReadyCheck::None);
    }

    #[test]
    fn party_all_responded_not_active() {
        let state = PartyState::default();
        assert!(!state.all_responded());
    }

    // --- ReadyCheckState ---

    #[test]
    fn ready_check_state_start() {
        let mut state = ReadyCheckState::default();
        state.start("Leader".into());
        assert!(state.active);
        assert_eq!(state.initiator, "Leader");
        assert!(state.awaiting_response());
        assert!((state.remaining_secs - READY_CHECK_TIMEOUT_SECS).abs() < 0.01);
    }

    #[test]
    fn ready_check_state_respond() {
        let mut state = ReadyCheckState::default();
        state.start("Leader".into());
        state.respond(ReadyCheck::Accepted);
        assert!(!state.awaiting_response());
        assert_eq!(state.local_response, ReadyCheck::Accepted);
    }

    #[test]
    fn ready_check_state_finish() {
        let mut state = ReadyCheckState::default();
        state.start("Leader".into());
        state.respond(ReadyCheck::Accepted);
        state.finish();
        assert!(!state.active);
        assert_eq!(state.remaining_secs, 0.0);
    }

    #[test]
    fn ready_check_state_awaiting_only_when_pending() {
        let mut state = ReadyCheckState::default();
        assert!(!state.awaiting_response()); // not active
        state.start("X".into());
        assert!(state.awaiting_response()); // active + pending
        state.respond(ReadyCheck::Declined);
        assert!(!state.awaiting_response()); // responded
    }

    // --- GroupIntentQueue ---

    #[test]
    fn group_intent_initiate() {
        let mut queue = GroupIntentQueue::default();
        queue.initiate_ready_check();
        let drained = queue.drain();
        assert_eq!(drained[0], GroupIntent::InitiateReadyCheck);
    }

    #[test]
    fn group_intent_accept_decline() {
        let mut queue = GroupIntentQueue::default();
        queue.accept_ready_check();
        queue.decline_ready_check();
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0], GroupIntent::AcceptReadyCheck);
        assert_eq!(drained[1], GroupIntent::DeclineReadyCheck);
    }

    #[test]
    fn group_intent_set_role() {
        let mut queue = GroupIntentQueue::default();
        queue.set_role("Alice".into(), GroupRole::Tank);
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GroupIntent::SetRole {
                player_name: "Alice".into(),
                role: GroupRole::Tank,
            }
        );
    }

    #[test]
    fn group_intent_set_own_role() {
        let mut queue = GroupIntentQueue::default();
        queue.set_own_role(GroupRole::Healer);
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GroupIntent::SetOwnRole {
                role: GroupRole::Healer
            }
        );
    }

    #[test]
    fn group_intent_drain_clears() {
        let mut queue = GroupIntentQueue::default();
        queue.initiate_ready_check();
        assert_eq!(queue.drain().len(), 1);
        assert!(queue.pending.is_empty());
    }

    // --- Role assignment on PartyState ---

    #[test]
    fn party_assign_role() {
        let mut state = PartyState {
            members: vec![named_unit("Alice"), named_unit("Bob")],
            ..Default::default()
        };
        assert!(state.assign_role("Alice", GroupRole::Tank));
        assert_eq!(state.members[0].role, GroupRole::Tank);
        assert_eq!(state.members[1].role, GroupRole::Dps); // unchanged
    }

    #[test]
    fn party_assign_role_not_found() {
        let mut state = PartyState {
            members: vec![named_unit("Alice")],
            ..Default::default()
        };
        assert!(!state.assign_role("Unknown", GroupRole::Healer));
    }

    #[test]
    fn party_role_count() {
        let mut state = PartyState {
            members: vec![named_unit("A"), named_unit("B"), named_unit("C")],
            ..Default::default()
        };
        state.assign_role("A", GroupRole::Tank);
        state.assign_role("B", GroupRole::Healer);
        // C remains Dps (default)
        assert_eq!(state.role_count(GroupRole::Tank), 1);
        assert_eq!(state.role_count(GroupRole::Healer), 1);
        assert_eq!(state.role_count(GroupRole::Dps), 1);
    }

    // --- Loot distribution ---

    #[test]
    fn loot_method_labels() {
        assert_eq!(LootMethod::FreeForAll.label(), "Free For All");
        assert_eq!(LootMethod::GroupLoot.label(), "Group Loot");
        assert_eq!(LootMethod::NeedBeforeGreed.label(), "Need Before Greed");
        assert_eq!(LootMethod::MasterLooter.label(), "Master Looter");
        assert_eq!(LootMethod::RoundRobin.label(), "Round Robin");
        assert_eq!(LootMethod::PersonalLoot.label(), "Personal Loot");
    }

    #[test]
    fn loot_method_default_is_group_loot() {
        assert_eq!(LootMethod::default(), LootMethod::GroupLoot);
    }

    #[test]
    fn loot_threshold_labels() {
        assert_eq!(LootThreshold::Uncommon.label(), "Uncommon");
        assert_eq!(LootThreshold::Rare.label(), "Rare");
        assert_eq!(LootThreshold::Epic.label(), "Epic");
    }

    #[test]
    fn loot_threshold_default_is_uncommon() {
        assert_eq!(LootThreshold::default(), LootThreshold::Uncommon);
    }

    #[test]
    fn loot_threshold_quality_ids() {
        assert_eq!(LootThreshold::Poor.quality_id(), 0);
        assert_eq!(LootThreshold::Common.quality_id(), 1);
        assert_eq!(LootThreshold::Uncommon.quality_id(), 2);
        assert_eq!(LootThreshold::Rare.quality_id(), 3);
        assert_eq!(LootThreshold::Epic.quality_id(), 4);
        assert_eq!(LootThreshold::Legendary.quality_id(), 5);
    }

    #[test]
    fn party_loot_defaults() {
        let state = PartyState::default();
        assert_eq!(state.loot.method, LootMethod::GroupLoot);
        assert_eq!(state.loot.threshold, LootThreshold::Uncommon);
    }

    #[test]
    fn group_intent_set_loot_method() {
        let mut queue = GroupIntentQueue::default();
        queue.set_loot_method(LootMethod::MasterLooter);
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GroupIntent::SetLootMethod {
                method: LootMethod::MasterLooter
            }
        );
    }

    #[test]
    fn group_intent_set_loot_threshold() {
        let mut queue = GroupIntentQueue::default();
        queue.set_loot_threshold(LootThreshold::Epic);
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GroupIntent::SetLootThreshold {
                threshold: LootThreshold::Epic
            }
        );
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::HEALTH_BAR_FILL, 0);
        assert_ne!(textures::LFG_ROLE_ICONS, 0);
        assert_ne!(textures::ROLE_ICONS, 0);
        assert_ne!(textures::LFG_ROLE, 0);
        assert_ne!(textures::READY_CHECK_OK, 0);
        assert_ne!(textures::READY_CHECK_FAIL, 0);
        assert_ne!(textures::READY_CHECK_WAIT, 0);
        assert_ne!(textures::READY_CHECK_FRAME, 0);
        assert_ne!(textures::DEBUFF_BORDER, 0);
        assert_ne!(textures::DEBUFF_OVERLAYS, 0);
    }
}
