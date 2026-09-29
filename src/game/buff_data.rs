use bevy::prelude::*;
pub use crate::aura_display_data::{AuraCasterLookup, AuraInstance, DebuffType, aura_instances};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_catalog::{CatalogSpell, SpellCatalogData, SpellTextContext};
    use shared::components::AuraView;

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
            from_player: false,
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

    fn catalog() -> SpellCatalogData {
        let spell = |id: u32, name: &str, icon: u32, aura_description: &str| CatalogSpell {
            id,
            name: name.into(),
            icon_fdid: icon,
            aura_description: aura_description.into(),
            ..Default::default()
        };
        SpellCatalogData::from_parts(
            vec![
                spell(589, "Shadow Word: Pain", 136207, "Suffering Shadow damage."),
                spell(21562, "Power Word: Fortitude", 135987, "Stamina increased."),
                spell(172, "Corruption", 136118, ""),
            ],
            Default::default(),
        )
    }

    fn lookup(caster: u64) -> Option<String> {
        (caster == OTHER).then(|| "Kobold Geomancer".to_string())
    }

    fn instances(views: &[AuraView]) -> Vec<AuraInstance> {
        let casters = AuraCasterLookup {
            local_player: Some(LOCAL),
            name_of: &lookup,
        };
        aura_instances(views, Some(&catalog()), &casters, &SpellTextContext::default())
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
        assert!(fort.from_local_player && fort.from_player && !fort.is_debuff);
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
    fn auras_keep_the_replicated_slot_order() {
        let auras = instances(&[
            view(1, 589, OTHER, true, 0),
            view(2, 172, LOCAL, true, 0),
            view(3, 21562, OTHER, false, 0),
            view(4, 589, LOCAL, true, 0),
        ]);
        let order: Vec<u32> = auras.iter().map(|aura| aura.instance_id).collect();
        assert_eq!(order, [1, 2, 3, 4]);
    }

    #[test]
    fn timer_text_uses_retail_abbreviations() {
        let text = |remaining: f32| make_buff("A", 100_000.0, remaining).timer_text();
        assert_eq!(make_buff("Perm", 0.0, 0.0).timer_text(), "");
        assert_eq!(text(59.9), "59 s");
        assert_eq!(text(0.4), "0 s");
        assert_eq!(text(60.0), "60 s");
        assert_eq!(text(89.9), "89 s");
        assert_eq!(text(90.0), "2 m");
        assert_eq!(text(125.0), "3 m");
        assert_eq!(text(3600.0), "60 m");
        assert_eq!(text(5400.0), "2 h");
        assert_eq!(text(90_000.0), "25 h");
        assert_eq!(text(129_600.0), "2 d");
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
