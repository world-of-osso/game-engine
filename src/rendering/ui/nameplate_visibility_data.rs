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

/// A unit fights the player while it is in combat with the player as its target.
/// The client sees only the replicated combat flag and target, not a threat list.
pub fn in_combat_with_player<T: PartialEq>(in_combat: bool, target: Option<T>, player: T) -> bool {
    in_combat && target == Some(player)
}

/// Plate alpha: `nameplateOccludedAlphaMult` behind world geometry, else opaque.
pub fn plate_alpha(cvars: &NameplateCvars, occluded: bool) -> f32 {
    if occluded {
        cvars.occluded_alpha_mult
    } else {
        1.0
    }
}
