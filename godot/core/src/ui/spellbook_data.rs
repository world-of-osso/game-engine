//! Spellbook tabs built from the known-spell list and the spell catalog.
//! Grouping rule: [`crate::spell_catalog::SpellbookTabIndex`].

use crate::spell_catalog::{SpellCatalogData, SpellbookTabKind};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellbookSpell {
    pub id: u32,
    pub name: String,
    pub subtext: String,
    pub passive: bool,
    pub icon_file_data_id: u32,
    /// Not known yet: Retail lists it greyed "Available at level N" (`SPELLBOOK_AVAILABLE_AT`).
    pub available_at: Option<u32>,
}

/// The viewing player, for the spells they learn at later levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellbookPlayer {
    pub class_id: u32,
    pub race_id: u32,
    pub level: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellbookTab {
    pub name: String,
    pub spells: Vec<SpellbookSpell>,
}

/// General, class and (for an active non-Initial spec) spec tabs, without the
/// empty ones: Retail lists no empty category. A spell is listed only when its
/// [`crate::spell_catalog::SpellbookListing`] allows it, known or not. Actives come before passives,
/// each in learn order, then (with `player`) the spells learned at later levels by level.
pub fn build_spellbook_tabs(
    known: &[u32],
    spec_id: Option<u32>,
    catalog: Option<&SpellCatalogData>,
    player: Option<SpellbookPlayer>,
) -> Vec<SpellbookTab> {
    let class_name = catalog
        .and_then(|data| data.tabs.class_name(spec_id, known))
        .unwrap_or("Class");
    let spec_name = catalog.and_then(|data| data.tabs.spec_name(spec_id));
    let mut tabs = vec![tab("General"), tab(class_name)];
    if let Some(name) = spec_name {
        tabs.push(tab(name));
    }
    let listed = |id, known| {
        catalog
            .and_then(|data| data.get(id))
            .is_none_or(|spell| spell.spellbook.lists(known))
    };
    let future = match (catalog, player) {
        (Some(data), Some(player)) => {
            data.tabs
                .future_spells(player.class_id, player.race_id, spec_id, player.level)
        }
        _ => Vec::new(),
    };
    let known_entries = known.iter().map(|&id| (id, None));
    let future_entries = future
        .iter()
        .filter(|spell| !known.contains(&spell.spell_id))
        .map(|spell| (spell.spell_id, Some(spell.level)));
    for (id, available_at) in known_entries
        .chain(future_entries)
        .filter(|&(id, available_at)| listed(id, available_at.is_none()))
    {
        let kind = catalog.map_or(SpellbookTabKind::General, |data| {
            data.tabs.classify(id, spec_id)
        });
        let index = match kind {
            SpellbookTabKind::General => 0,
            SpellbookTabKind::Class => 1,
            SpellbookTabKind::Spec => tabs.len() - 1,
        };
        let mut spell = spellbook_spell(id, catalog);
        spell.available_at = available_at;
        tabs[index].spells.push(spell);
    }
    tabs.retain(|tab| !tab.spells.is_empty());
    for tab in &mut tabs {
        // Stable: known actives, known passives, then future spells by level.
        tab.spells.sort_by_key(|spell| {
            (
                spell.available_at.is_some(),
                spell.passive && spell.available_at.is_none(),
            )
        });
    }
    tabs
}

fn tab(name: &str) -> SpellbookTab {
    SpellbookTab {
        name: name.to_string(),
        spells: Vec::new(),
    }
}

fn spellbook_spell(id: u32, catalog: Option<&SpellCatalogData>) -> SpellbookSpell {
    match catalog.and_then(|data| data.get(id)) {
        Some(spell) => SpellbookSpell {
            id,
            name: spell.name.to_string(),
            subtext: spell.subtext.to_string(),
            passive: spell.passive,
            icon_file_data_id: spell.icon_fdid,
            available_at: None,
        },
        None => SpellbookSpell {
            id,
            name: format!("Spell {id}"),
            subtext: String::new(),
            passive: false,
            icon_file_data_id: 0,
            available_at: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_catalog::{CatalogSpell, SpecTabInfo, SpellbookTabIndex};

    fn spell(id: u32, name: &str, passive: bool) -> CatalogSpell {
        CatalogSpell {
            id,
            name: name.into(),
            passive,
            icon_fdid: id + 1,
            ..Default::default()
        }
    }

    fn catalog() -> SpellCatalogData {
        let tabs = SpellbookTabIndex {
            class_names: [(2, "Paladin".to_string())].into(),
            specs: [(
                66,
                SpecTabInfo {
                    name: "Protection".into(),
                    class_id: 2,
                    order_index: 1,
                    initial: false,
                    spells: [76671].into(),
                    primary_stat_priority: 5,
                    icon_fdid: 236264,
                    role: 0,
                    description: String::new(),
                },
            )]
            .into(),
            class_spells: [(35395, 2), (20271, 2)].into(),
            ..Default::default()
        };
        SpellCatalogData::from_parts(
            vec![
                spell(6603, "Auto Attack", false),
                spell(35395, "Crusader Strike", false),
                spell(20271, "Judgment", false),
                spell(76671, "Mastery: Divine Bulwark", true),
            ],
            tabs,
        )
    }

    fn names(tab: &SpellbookTab) -> Vec<&str> {
        tab.spells.iter().map(|spell| spell.name.as_str()).collect()
    }

    #[test]
    fn known_spells_group_into_general_class_and_spec_tabs() {
        let catalog = catalog();
        let tabs =
            build_spellbook_tabs(&[76671, 6603, 35395, 20271], Some(66), Some(&catalog), None);
        let tab_names: Vec<_> = tabs.iter().map(|tab| tab.name.as_str()).collect();
        assert_eq!(tab_names, ["General", "Paladin", "Protection"]);
        assert_eq!(names(&tabs[0]), ["Auto Attack"]);
        assert_eq!(names(&tabs[1]), ["Crusader Strike", "Judgment"]);
        assert_eq!(names(&tabs[2]), ["Mastery: Divine Bulwark"]);
        assert!(tabs[2].spells[0].passive);
        assert_eq!(tabs[1].spells[0].icon_file_data_id, 35396);
    }

    #[test]
    fn missing_catalog_lists_spell_ids_on_general() {
        let tabs = build_spellbook_tabs(&[35395], None, None, None);
        assert_eq!(tabs.len(), 1);
        assert_eq!(names(&tabs[0]), ["Spell 35395"]);
    }
}
