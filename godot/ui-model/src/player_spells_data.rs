//! Shared Player Spells tab and item data, used by the micro menu and spell overrides.

/// Blizzard_PlayerSpellsFrame.lua:14-16: independent pages, in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerSpellsTab {
    Specialization,
    Talents,
    #[default]
    Spellbook,
}

impl PlayerSpellsTab {
    pub const ALL: [Self; 3] = [Self::Specialization, Self::Talents, Self::Spellbook];

    pub fn title(self) -> &'static str {
        match self {
            Self::Specialization => "Specialization",
            Self::Talents => "Talents",
            Self::Spellbook => "Spellbook",
        }
    }
}

/// One spell entry of a category.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellbookItemView {
    pub spell_id: u32,
    pub name: String,
    pub subtext: String,
    pub icon_fdid: u32,
    pub passive: bool,
    /// Learned later: greyed with "Level N" (`FutureSpell`).
    pub available_at: Option<u32>,
    /// Remaining fraction of the effective spell's cooldown/GCD.
    pub cooldown_fraction: f32,
}
