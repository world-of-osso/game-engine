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
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellbookTab {
    pub name: String,
    pub spells: Vec<SpellbookSpell>,
}

/// General and class tabs always; a spec tab for an active non-Initial spec.
/// Actives come before passives, each in learn order.
pub fn build_spellbook_tabs(
    known: &[u32],
    spec_id: Option<u32>,
    catalog: Option<&SpellCatalogData>,
) -> Vec<SpellbookTab> {
    let class_name = catalog
        .and_then(|data| data.tabs.class_name(spec_id, known))
        .unwrap_or("Class");
    let spec_name = catalog.and_then(|data| data.tabs.spec_name(spec_id));
    let mut tabs = vec![tab("General"), tab(class_name)];
    if let Some(name) = spec_name {
        tabs.push(tab(name));
    }
    for &id in known {
        let kind = catalog.map_or(SpellbookTabKind::General, |data| {
            data.tabs.classify(id, spec_id)
        });
        let index = match kind {
            SpellbookTabKind::General => 0,
            SpellbookTabKind::Class => 1,
            SpellbookTabKind::Spec => tabs.len() - 1,
        };
        tabs[index].spells.push(spellbook_spell(id, catalog));
    }
    for tab in &mut tabs {
        tab.spells.sort_by_key(|spell| spell.passive);
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
        },
        None => SpellbookSpell {
            id,
            name: format!("Spell {id}"),
            subtext: String::new(),
            passive: false,
            icon_file_data_id: 0,
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
                    initial: false,
                    spells: [76671].into(),
                },
            )]
            .into(),
            class_spells: [(35395, 2), (20271, 2)].into(),
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
        let tabs = build_spellbook_tabs(&[76671, 6603, 35395, 20271], Some(66), Some(&catalog));
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
        let tabs = build_spellbook_tabs(&[35395], None, None);
        assert_eq!(tabs.len(), 2);
        assert_eq!(names(&tabs[0]), ["Spell 35395"]);
    }
}
