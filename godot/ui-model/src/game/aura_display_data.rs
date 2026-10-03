//! Displayable auras from replicated `UnitAuras` views, without the Bevy runtime:
//! the shared model of the BuffFrame, DebuffFrame, TargetFrame and nameplate auras.

use shared::components::AuraView;

use crate::spell_catalog::{SpellCatalogData, SpellTextContext};

/// Debuff dispel type, determines border color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DebuffType {
    #[default]
    None,
    Magic,
    Curse,
    Disease,
    Poison,
}

impl DebuffType {
    /// Retail `SpellDispelType` id (1 magic, 2 curse, 3 disease, 4 poison).
    pub fn from_dispel_id(id: u8) -> Self {
        match id {
            1 => Self::Magic,
            2 => Self::Curse,
            3 => Self::Disease,
            4 => Self::Poison,
            _ => Self::None,
        }
    }

    /// RGBA border color for this debuff type.
    pub fn border_color(self) -> &'static str {
        self.border_color_for_mode(false)
    }

    pub fn border_color_for_mode(self, colorblind_mode: bool) -> &'static str {
        if colorblind_mode {
            return match self {
                Self::None => "0.5,0.2,0.2,1.0",
                Self::Magic => "0.1,0.7,1.0,1.0",
                Self::Curse => "1.0,0.3,1.0,1.0",
                Self::Disease => "1.0,0.55,0.0,1.0",
                Self::Poison => "0.0,0.85,0.75,1.0",
            };
        }
        // Retail `DebuffTypeColor`.
        match self {
            Self::None => "0.8,0.0,0.0,1.0",
            Self::Magic => "0.2,0.6,1.0,1.0",
            Self::Curse => "0.6,0.0,1.0,1.0",
            Self::Disease => "0.6,0.4,0.0,1.0",
            Self::Poison => "0.0,0.6,0.0,1.0",
        }
    }
}

/// A single active buff or debuff.
#[derive(Clone, Debug, PartialEq)]
pub struct AuraInstance {
    pub instance_id: u32,
    pub spell_id: u32,
    pub name: String,
    /// Rendered `AuraDescription_lang`.
    pub description: String,
    pub icon_fdid: u32,
    /// Caster name, empty when unknown.
    pub source: String,
    /// Cast by the local player.
    pub from_local_player: bool,
    /// Cast by a player or a player's pet (`isFromPlayerOrPlayerPet`).
    pub from_player: bool,
    /// Total duration in seconds (0 = permanent).
    pub duration: f32,
    /// Remaining time in seconds.
    pub remaining: f32,
    pub stacks: u32,
    pub is_debuff: bool,
    pub debuff_type: DebuffType,
}

impl AuraInstance {
    pub fn is_permanent(&self) -> bool {
        self.duration <= 0.0
    }

    /// Retail `SecondsToTimeAbbrev` text ("2 h", "5 m", "89 s"): a unit is used from 1.5 of
    /// it and rounds up (TimeUtil.lua:463-483); empty when permanent.
    pub fn timer_text(&self) -> String {
        if self.is_permanent() {
            return String::new();
        }
        let secs = self.remaining.max(0.0);
        for (unit, label) in [(86_400.0, "d"), (3_600.0, "h"), (60.0, "m")] {
            if secs >= unit * 1.5 {
                return format!("{} {label}", (secs / unit).ceil() as u32);
            }
        }
        format!("{} s", secs as u32)
    }
}

/// Who cast an aura, relative to the viewing client.
pub struct AuraCasterLookup<'a> {
    /// Server entity bits of the local player.
    pub local_player: Option<u64>,
    pub name_of: &'a dyn Fn(u64) -> Option<String>,
}

/// Displayable auras in replicated (server slot) order, which BuffFrame and DebuffFrame
/// keep (`AuraUtil.ForEachAura`, BuffFrame.lua:638,761). Hidden and passive auras are
/// dropped. Without a loaded catalog names, descriptions and icons are empty.
pub fn aura_instances(
    views: &[AuraView],
    catalog: Option<&SpellCatalogData>,
    casters: &AuraCasterLookup,
    text_ctx: &SpellTextContext,
) -> Vec<AuraInstance> {
    views
        .iter()
        .filter(|view| view.flags & (AuraView::FLAG_HIDDEN | AuraView::FLAG_PASSIVE) == 0)
        .map(|view| aura_instance(view, catalog, casters, text_ctx))
        .collect()
}

fn aura_instance(
    view: &AuraView,
    catalog: Option<&SpellCatalogData>,
    casters: &AuraCasterLookup,
    text_ctx: &SpellTextContext,
) -> AuraInstance {
    let spell = catalog.and_then(|catalog| catalog.get(view.spell_id));
    AuraInstance {
        instance_id: view.instance_id,
        spell_id: view.spell_id,
        name: spell
            .map(|spell| spell.name.to_string())
            .unwrap_or_default(),
        description: catalog
            .and_then(|catalog| catalog.render_aura_description(view.spell_id, text_ctx))
            .unwrap_or_default(),
        icon_fdid: spell.map_or(0, |spell| spell.icon_fdid),
        source: view
            .caster
            .and_then(|caster| (casters.name_of)(caster))
            .unwrap_or_default(),
        from_local_player: view.caster.is_some() && view.caster == casters.local_player,
        from_player: view.flags & AuraView::FLAG_FROM_PLAYER != 0,
        duration: view.duration_ms as f32 / 1000.0,
        remaining: view.remaining_ms as f32 / 1000.0,
        stacks: u32::from(view.stacks),
        is_debuff: view.harmful,
        debuff_type: DebuffType::from_dispel_id(view.dispel_type),
    }
}
