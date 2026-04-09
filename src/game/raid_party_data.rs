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
#[path = "raid_party_data_tests/mod.rs"]
mod tests;
