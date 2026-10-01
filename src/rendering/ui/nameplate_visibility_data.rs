//! Engine-free retail nameplate visibility and alpha rules (used by the Godot client
//! through `game-engine-core`). Retail decides plate visibility in the engine from CVars; the
//! default UI only exposes them (Blizzard_SettingsDefinitions_Frame/Nameplates.lua).
//! Defaults are the engine CVar table (wow-ui-sim `src/cvars.yaml:977-1008`, from wowless).

/// The nameplate CVars that decide which units get a plate and how opaque it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NameplateCvars {
    /// `nameplateShowAll` ("Always Show Nameplates"). Off: "Nameplates by default are
    /// only shown in combat" (`OPTION_TOOLTIP_UNIT_NAMEPLATES_AUTOMODE`).
    pub show_all: bool,
    /// `nameplateShowEnemies`: plates for units the player can attack.
    pub show_enemies: bool,
    /// `nameplateShowFriendlyPlayers`.
    pub show_friendly_players: bool,
    /// `nameplateShowFriendlyNpcs`.
    pub show_friendly_npcs: bool,
    /// `nameplateMaxDistance`, yards.
    pub max_distance: f32,
    /// `nameplateOccludedAlphaMult`: alpha factor of a plate whose unit is behind
    /// world geometry.
    pub occluded_alpha_mult: f32,
    /// `nameplateSelectedAlpha`: alpha of the current target's plate, in place of the
    /// distance fade.
    pub selected_alpha: f32,
}

impl Default for NameplateCvars {
    fn default() -> Self {
        Self {
            show_all: false,
            show_enemies: true,
            show_friendly_players: false,
            show_friendly_npcs: false,
            max_distance: 60.0,
            occluded_alpha_mult: 0.4,
            selected_alpha: 1.0,
        }
    }
}

/// What the plate rules need to know about one unit, from the local player's view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateUnit {
    pub is_local_player: bool,
    /// Clear of `UNIT_FLAG_NOT_SELECTABLE`.
    pub selectable: bool,
    pub alive: bool,
    pub is_player: bool,
    /// The local player can attack it (hostile or neutral); otherwise it is friendly.
    pub enemy: bool,
    /// The local player's current target.
    pub targeted: bool,
    pub in_combat_with_player: bool,
    /// Yards from the local player.
    pub distance: f32,
}

/// Whether `unit` has a plate: its kind's switch is on, it is in range, and either
/// `nameplateShowAll` is on or it is the target or fighting the player.
pub fn plate_shown(cvars: &NameplateCvars, unit: &PlateUnit) -> bool {
    if unit.is_local_player || !unit.selectable || !unit.alive {
        return false;
    }
    if unit.distance > cvars.max_distance {
        return false;
    }
    let kind_enabled = if unit.enemy {
        cvars.show_enemies
    } else if unit.is_player {
        cvars.show_friendly_players
    } else {
        cvars.show_friendly_npcs
    };
    kind_enabled && (cvars.show_all || unit.targeted || unit.in_combat_with_player)
}

/// A unit fights the player while the player is on its threat list
/// (`CompactUnitFrame_IsOnThreatListWithPlayer`: `UnitDetailedThreatSituation` is non-nil,
/// CompactUnitFrame.lua:563-566), or, for a unit without one (another player), while it
/// is in combat with the player as its target.
pub fn in_combat_with_player<T: PartialEq>(
    in_combat: bool,
    target: Option<T>,
    threat_list: &[T],
    player: T,
) -> bool {
    threat_list.contains(&player) || (in_combat && target == Some(player))
}

/// Whether a plate's health bar takes the hostile colour (1, 0, 0) instead of the unit's
/// selection colour: `considerSelectionInCombatAsHostile` (on for Retail nameplates,
/// Blizzard_NamePlateFrameOptions.lua:30, :53) with the player on the unit's threat list
/// and the unit not friendly (CompactUnitFrame.lua:674-675).
pub fn selection_in_combat_is_hostile<T: PartialEq>(
    threat_list: &[T],
    player: T,
    friendly: bool,
) -> bool {
    !friendly && threat_list.contains(&player)
}

/// Legacy camera-to-health-body fade, independent of the viewer-to-unit CVar limit.
/// Full opacity up to half the configured distance; zero at the distance itself.
pub fn nameplate_alpha(distance: f32, fade_far: f32) -> f32 {
    let fade_far = fade_far.max(1.0);
    let fade_near = (fade_far * 0.5).max(1.0);
    if distance <= fade_near {
        1.0
    } else if distance >= fade_far {
        0.0
    } else {
        1.0 - (distance - fade_near) / (fade_far - fade_near)
    }
}

/// Plate alpha: the target's plate takes `nameplateSelectedAlpha` (default 1.0,
/// cvars.yaml:990) instead of `distance_fade`, so it never fades with camera distance;
/// behind world geometry either is scaled by `nameplateOccludedAlphaMult`.
pub fn plate_alpha(
    cvars: &NameplateCvars,
    selected: bool,
    distance_fade: f32,
    occluded: bool,
) -> f32 {
    let alpha = if selected {
        cvars.selected_alpha
    } else {
        distance_fade
    };
    if occluded {
        alpha * cvars.occluded_alpha_mult
    } else {
        alpha
    }
}

#[cfg(test)]
mod threat_tests {
    use super::*;

    const LOCAL: u64 = 7;
    const PARTY: u64 = 9;

    #[test]
    fn a_creature_fights_the_player_while_the_player_is_on_its_threat_list() {
        // A polymorphed creature has no target but keeps its threat list.
        assert!(in_combat_with_player(true, None, &[PARTY, LOCAL], LOCAL));
        assert!(!in_combat_with_player(true, Some(PARTY), &[PARTY], LOCAL));
        // Another player has no threat list: combat and target decide.
        assert!(in_combat_with_player(true, Some(LOCAL), &[], LOCAL));
        assert!(!in_combat_with_player(false, Some(LOCAL), &[], LOCAL));
    }

    #[test]
    fn a_neutral_creature_that_has_the_player_on_its_threat_list_shows_hostile() {
        assert!(selection_in_combat_is_hostile(&[LOCAL], LOCAL, false));
        assert!(!selection_in_combat_is_hostile(&[PARTY], LOCAL, false));
        assert!(!selection_in_combat_is_hostile::<u64>(&[], LOCAL, false));
        // UnitIsFriend: a friendly unit keeps its selection colour.
        assert!(!selection_in_combat_is_hostile(&[LOCAL], LOCAL, true));
    }
}
