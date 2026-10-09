//! Offline catalog previews and the unchanged legacy Mage snapshot, without a session/network.
use crate::spellbook_frame_component::{
    PlayerSpellsTab, SpellbookCategory, SpellbookFrameState, SpellbookGroup, SpellbookItemView,
    specialization_choices,
};
use game_engine_core::spell_catalog::{SpellCatalogPaths, load_spell_catalog};
use game_engine_core::spellbook_data::{SpellbookTab, build_spellbook_tabs};

const MAGE_CLASS: u32 = 8;
const FROST_SPEC: u32 = 64;
const ARCANE_SPEC: u32 = 62;
// Frostbolt, Frost Nova, Slow Fall, Remove Curse, Arcane Explosion/Intellect,
// Blink, Counterspell, Ice Lance, Icy Veins and Attack (a bounded known-spell snapshot).
const KNOWN_SPELLS: [u32; 11] = [
    116, 122, 130, 475, 1449, 1459, 1953, 2139, 30455, 12472, 6603,
];

pub fn load_preview_state(
    data: &std::path::Path,
    tab: PlayerSpellsTab,
) -> Result<SpellbookFrameState, String> {
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(data))?;
    for id in KNOWN_SPELLS {
        if catalog.get(id).is_none() {
            return Err(format!("Spellbook preview requires local spell {id}"));
        }
    }
    let spec = if tab == PlayerSpellsTab::Talents {
        ARCANE_SPEC
    } else {
        FROST_SPEC
    };
    let talents = if tab == PlayerSpellsTab::Talents {
        Some(load_preview_talents(data, MAGE_CLASS, spec, &catalog)?)
    } else {
        None
    };
    let tabs = build_spellbook_tabs(&KNOWN_SPELLS, Some(spec), Some(&catalog), None);
    let portrait_fdid = catalog
        .tabs
        .specs
        .get(&spec)
        .ok_or_else(|| format!("Spellbook preview requires local specialization {spec}"))?
        .icon_fdid;
    Ok(SpellbookFrameState {
        viewport: [1920.0, 1080.0],
        categories: preview_categories(tabs, "Mage"),
        tab,
        specializations: specialization_choices(&catalog.tabs, MAGE_CLASS, Some(spec)),
        talents,
        portrait_fdid,
        can_activate_spec: true,
        ..Default::default()
    })
}

/// Explicit class/spec catalog preview, independent of the legacy Mage snapshot.
pub fn load_class_preview_state(
    data: &std::path::Path,
    tab: PlayerSpellsTab,
    class: u32,
    spec: u32,
) -> Result<SpellbookFrameState, String> {
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(data))?;
    let class_name = catalog
        .tabs
        .class_names
        .get(&class)
        .ok_or_else(|| format!("Unknown preview class {class}"))?;
    let info = catalog
        .tabs
        .specs
        .get(&spec)
        .ok_or_else(|| format!("Unknown preview specialization {spec}"))?;
    if info.class_id != class || info.initial {
        return Err(format!(
            "Preview spec {spec} is not a playable specialization of class {class}"
        ));
    }
    let known = catalog_preview_spells(&catalog, class, spec);
    let tabs = build_spellbook_tabs(&known, Some(spec), Some(&catalog), None);
    if tabs.is_empty() {
        return Err(format!(
            "Preview class {class} spec {spec} has no listed catalog spells"
        ));
    }
    let categories = preview_categories(tabs, class_name);
    let talents = if tab == PlayerSpellsTab::Talents {
        Some(load_preview_talents(data, class, spec, &catalog)?)
    } else {
        None
    };
    Ok(SpellbookFrameState {
        viewport: [1920.0, 1080.0],
        categories,
        tab,
        specializations: specialization_choices(&catalog.tabs, class, Some(spec)),
        talents,
        portrait_fdid: info.icon_fdid,
        can_activate_spec: true,
        ..Default::default()
    })
}

fn catalog_preview_spells(
    catalog: &game_engine_core::spell_catalog::SpellCatalogData,
    class: u32,
    spec: u32,
) -> Vec<u32> {
    let mut known: Vec<_> = catalog
        .tabs
        .class_spells
        .iter()
        .filter_map(|(&id, &owner)| (owner == class).then_some(id))
        .chain(catalog.tabs.specs[&spec].spells.iter().copied())
        .filter(|&id| catalog.get(id).is_some())
        .collect();
    known.sort_unstable();
    known.dedup();
    known
}

fn load_preview_talents(
    data: &std::path::Path,
    class: u32,
    spec: u32,
    catalog: &game_engine_core::spell_catalog::SpellCatalogData,
) -> Result<crate::talents::TalentView, String> {
    let mut view = crate::talents::load_talent_view(data, class, spec, catalog)?;
    if view.graph.class.nodes.is_empty() || view.graph.spec.nodes.is_empty() {
        return Err(format!(
            "Preview class {class} spec {spec} requires nonempty class and spec talent trees"
        ));
    }
    let tree = view.rules.as_ref().ok_or("Preview talent rules missing")?;
    let context = game_engine_core::talent_data::rules::TraitContext {
        spec_id: spec,
        level: 80,
    };
    let grants = game_engine_core::talent_data::rules::granted_entries(tree, context);
    let entries = grants
        .iter()
        .map(|entry| shared::protocol::TraitEntrySelection {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            rank: entry.total() as u8,
        })
        .collect();
    let unspent = game_engine_core::talent_data::rules::unspent(tree, context, &grants);
    view.editor
        .receive_snapshot(shared::protocol::TraitConfigSnapshot {
            spec_id: spec,
            tree_id: view.graph.tree_id,
            entries,
            unspent,
        });
    view.level = context.level;
    Ok(view)
}

/// Capture-only pending edits through the same model operations as live clicks.
pub fn stage_talent_preview(state: &mut SpellbookFrameState) -> Result<(), String> {
    let view = state
        .talents
        .as_mut()
        .ok_or("Pending preview requires Talents")?;
    let mut editor = view.editor.clone();
    for tree in [&view.graph.class, &view.graph.spec] {
        let (node, entry) = tree
            .nodes
            .iter()
            .flat_map(|node| node.entries.iter().map(move |entry| (node.id, entry.id)))
            .find(|&(node, entry)| editor.can_purchase(view, view.level, node, entry))
            .ok_or_else(|| format!("Preview has no purchasable {} node", tree.name))?;
        editor.purchase(view, view.level, node, entry);
    }
    view.editor = editor;
    Ok(())
}

fn preview_categories(tabs: Vec<SpellbookTab>, class_name: &str) -> Vec<SpellbookCategory> {
    let (general, class): (Vec<_>, Vec<_>) = tabs
        .into_iter()
        .map(preview_group)
        .partition(|group| group.name == "General");
    vec![
        SpellbookCategory {
            name: class_name.into(),
            groups: class,
        },
        SpellbookCategory {
            name: "General".into(),
            groups: general,
        },
    ]
    .into_iter()
    .filter(|category| !category.groups.is_empty())
    .collect()
}

fn preview_group(tab: SpellbookTab) -> SpellbookGroup {
    SpellbookGroup {
        name: tab.name,
        items: tab
            .spells
            .into_iter()
            .map(|spell| SpellbookItemView {
                spell_id: spell.id,
                name: spell.name,
                subtext: if spell.passive {
                    "Passive".into()
                } else {
                    spell.subtext
                },
                icon_fdid: spell.icon_file_data_id,
                passive: spell.passive,
                available_at: spell.available_at,
            })
            .collect(),
    }
}
