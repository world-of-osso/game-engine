use bevy::prelude::*;
use shared::components::AuraView;

use crate::spell_catalog::SpellCatalog;

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

    /// Retail `SecondsToTimeAbbrev` text ("1 h", "5 m", "59 s"); empty when permanent.
    pub fn timer_text(&self) -> String {
        if self.is_permanent() {
            return String::new();
        }
        let secs = self.remaining.max(0.0);
        for (unit, label) in [(86_400.0, "d"), (3_600.0, "h"), (60.0, "m")] {
            if secs >= unit {
                return format!("{} {label}", (secs / unit).ceil() as u32);
            }
        }
        format!("{} s", secs as u32)
    }
}

/// Runtime aura state for the local player.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct AuraState {
    pub auras: Vec<AuraInstance>,
}

/// Aura state of any other world unit, for target frames and nameplates.
#[derive(Component, Clone, Debug, PartialEq, Default)]
pub struct UnitAuraState {
    pub auras: Vec<AuraInstance>,
}

impl AuraState {
    pub fn buffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| !a.is_debuff)
    }

    pub fn debuffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| a.is_debuff)
    }

    pub fn tick(&mut self, dt: f32) {
        tick_auras(&mut self.auras, dt);
    }
}

impl UnitAuraState {
    pub fn buffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| !a.is_debuff)
    }

    pub fn debuffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| a.is_debuff)
    }

    pub fn tick(&mut self, dt: f32) {
        tick_auras(&mut self.auras, dt);
    }
}

/// Counts timed auras down and drops the ones that ran out; the server removal follows.
fn tick_auras(auras: &mut Vec<AuraInstance>, dt: f32) {
    for aura in auras.iter_mut() {
        aura.remaining = (aura.remaining - dt).max(0.0);
    }
    auras.retain(|a| a.is_permanent() || a.remaining > 0.0);
}

/// Who cast an aura, relative to the viewing client.
pub struct AuraCasterLookup<'a> {
    /// Server entity bits of the local player.
    pub local_player: Option<u64>,
    pub name_of: &'a dyn Fn(u64) -> Option<String>,
}

/// Displayable auras in Retail order: buffs, then debuffs with the local player's first.
/// Hidden and passive auras are dropped.
pub fn aura_instances(
    views: &[AuraView],
    catalog: &SpellCatalog,
    casters: &AuraCasterLookup,
) -> Vec<AuraInstance> {
    let mut auras: Vec<AuraInstance> = views
        .iter()
        .filter(|view| view.flags & (AuraView::FLAG_HIDDEN | AuraView::FLAG_PASSIVE) == 0)
        .map(|view| aura_instance(view, catalog, casters))
        .collect();
    auras.sort_by_key(|aura| (aura.is_debuff, !aura.from_local_player || !aura.is_debuff));
    auras
}

fn aura_instance(
    view: &AuraView,
    catalog: &SpellCatalog,
    casters: &AuraCasterLookup,
) -> AuraInstance {
    let spell = catalog.get(view.spell_id);
    AuraInstance {
        instance_id: view.instance_id,
        spell_id: view.spell_id,
        name: spell
            .map(|spell| spell.name.to_string())
            .unwrap_or_default(),
        description: catalog
            .render_aura_description(view.spell_id)
            .unwrap_or_default(),
        icon_fdid: spell.map_or(0, |spell| spell.icon_fdid),
        source: view
            .caster
            .and_then(|caster| (casters.name_of)(caster))
            .unwrap_or_default(),
        from_local_player: view.caster.is_some() && view.caster == casters.local_player,
        duration: view.duration_ms as f32 / 1000.0,
        remaining: view.remaining_ms as f32 / 1000.0,
        stacks: u32::from(view.stacks),
        is_debuff: view.harmful,
        debuff_type: DebuffType::from_dispel_id(view.dispel_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_catalog::{CatalogSpell, SpellCatalogData, SpellCatalogState};

    const LOCAL: u64 = 42;
    const OTHER: u64 = 77;

    fn make_buff(name: &str, duration: f32, remaining: f32) -> AuraInstance {
        AuraInstance {
            instance_id: 1,
            spell_id: 1,
            name: name.into(),
            description: String::new(),
            icon_fdid: 12345,
            source: String::new(),
            from_local_player: false,
            duration,
            remaining,
            stacks: 1,
            is_debuff: false,
            debuff_type: DebuffType::None,
        }
    }

    fn view(instance_id: u32, spell_id: u32, caster: u64, harmful: bool, flags: u16) -> AuraView {
        AuraView {
            instance_id,
            spell_id,
            caster: Some(caster),
            stacks: 0,
            charges: 0,
            duration_ms: 18_000,
            remaining_ms: 12_500,
            harmful,
            dispel_type: 1,
            flags,
        }
    }

    fn catalog() -> SpellCatalog {
        let spell = |id: u32, name: &str, icon: u32, aura_description: &str| CatalogSpell {
            id,
            name: name.into(),
            icon_fdid: icon,
            aura_description: aura_description.into(),
            ..Default::default()
        };
        SpellCatalog {
            state: SpellCatalogState::Ready(SpellCatalogData::from_parts(
                vec![
                    spell(589, "Shadow Word: Pain", 136207, "Suffering Shadow damage."),
                    spell(21562, "Power Word: Fortitude", 135987, "Stamina increased."),
                    spell(172, "Corruption", 136118, ""),
                ],
                Default::default(),
            )),
        }
    }

    fn lookup(caster: u64) -> Option<String> {
        (caster == OTHER).then(|| "Kobold Geomancer".to_string())
    }

    fn instances(views: &[AuraView]) -> Vec<AuraInstance> {
        let casters = AuraCasterLookup {
            local_player: Some(LOCAL),
            name_of: &lookup,
        };
        aura_instances(views, &catalog(), &casters)
    }

    #[test]
    fn replicated_views_resolve_spell_data_and_drop_hidden_or_passive() {
        let auras = instances(&[
            view(1, 21562, LOCAL, false, AuraView::FLAG_FROM_PLAYER),
            view(2, 589, OTHER, true, 0),
            view(3, 172, LOCAL, false, AuraView::FLAG_PASSIVE),
            view(4, 172, LOCAL, false, AuraView::FLAG_HIDDEN),
        ]);
        assert_eq!(auras.len(), 2);
        let fort = &auras[0];
        assert_eq!(fort.name, "Power Word: Fortitude");
        assert_eq!(fort.icon_fdid, 135987);
        assert_eq!(fort.description, "Stamina increased.");
        assert!(fort.from_local_player && !fort.is_debuff);
        let pain = &auras[1];
        assert_eq!(
            (pain.instance_id, pain.is_debuff, pain.debuff_type),
            (2, true, DebuffType::Magic)
        );
        assert_eq!(pain.source, "Kobold Geomancer");
        assert!(!pain.from_local_player);
        assert_eq!((pain.duration, pain.remaining), (18.0, 12.5));
    }

    #[test]
    fn buffs_come_first_then_the_local_players_debuffs() {
        let auras = instances(&[
            view(1, 589, OTHER, true, 0),
            view(2, 172, LOCAL, true, 0),
            view(3, 21562, OTHER, false, 0),
            view(4, 589, LOCAL, true, 0),
        ]);
        let order: Vec<u32> = auras.iter().map(|aura| aura.instance_id).collect();
        assert_eq!(order, [3, 2, 4, 1]);
    }

    #[test]
    fn timer_text_uses_retail_abbreviations() {
        let text = |remaining: f32| make_buff("A", 100_000.0, remaining).timer_text();
        assert_eq!(make_buff("Perm", 0.0, 0.0).timer_text(), "");
        assert_eq!(text(59.9), "59 s");
        assert_eq!(text(0.4), "0 s");
        assert_eq!(text(60.0), "1 m");
        assert_eq!(text(125.0), "3 m");
        assert_eq!(text(3600.0), "1 h");
        assert_eq!(text(3700.0), "2 h");
        assert_eq!(text(90_000.0), "2 d");
    }

    #[test]
    fn tick_counts_down_and_drops_expired_timed_auras() {
        let mut state = AuraState {
            auras: vec![
                make_buff("Perm", 0.0, 0.0),
                make_buff("Short", 10.0, 1.0),
                make_buff("Long", 10.0, 5.0),
            ],
        };
        state.tick(2.0);
        let left: Vec<(&str, f32)> = state
            .auras
            .iter()
            .map(|aura| (aura.name.as_str(), aura.remaining))
            .collect();
        assert_eq!(left, [("Perm", 0.0), ("Long", 3.0)]);
    }

    #[test]
    fn dispel_ids_map_to_retail_border_colors() {
        let colors: Vec<&str> = (0..=4)
            .map(|id| DebuffType::from_dispel_id(id).border_color())
            .collect();
        assert_eq!(
            colors,
            [
                "0.8,0.0,0.0,1.0",
                "0.2,0.6,1.0,1.0",
                "0.6,0.0,1.0,1.0",
                "0.6,0.4,0.0,1.0",
                "0.0,0.6,0.0,1.0",
            ]
        );
    }

    #[test]
    fn colorblind_debuff_border_colors_are_distinct() {
        let colors: Vec<&str> = (0..=4)
            .map(|id| DebuffType::from_dispel_id(id).border_color_for_mode(true))
            .collect();
        for (i, a) in colors.iter().enumerate() {
            for b in &colors[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
