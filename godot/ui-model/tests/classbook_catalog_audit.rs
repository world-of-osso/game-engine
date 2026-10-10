//! Diagnostic output only: local catalog membership is not gameplay correctness.
use game_engine_core::spell_catalog::{SpellCatalogPaths, load_spell_catalog};
use game_engine_core::talent_data::load_talent_page;
use game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab;
use game_engine_ui_model::spellbook_preview::load_class_preview_state;

#[test]
#[ignore = "data-only coverage report, run explicitly with --nocapture"]
fn emit_classbook_catalog_coverage() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    let mut classes: Vec<_> = catalog.tabs.class_names.iter().collect();
    classes.sort_by_key(|(id, _)| **id);
    for (&class, name) in classes {
        let mut specs: Vec<_> = catalog
            .tabs
            .specs
            .iter()
            .filter(|(_, spec)| spec.class_id == class && !spec.initial)
            .collect();
        specs.sort_by_key(|(id, spec)| (spec.order_index, **id));
        if specs.is_empty() {
            continue;
        }
        let class_count = catalog
            .tabs
            .class_spells
            .iter()
            .filter(|(id, owner)| {
                **owner == class
                    && catalog
                        .get(**id)
                        .is_some_and(|spell| spell.spellbook.lists(true))
            })
            .count();
        println!("CLASSBOOK_CLASS\t{class}\t{name}\t{class_count}");
        let mut missing_class: Vec<_> = catalog
            .tabs
            .class_spells
            .iter()
            .filter_map(|(&id, &owner)| (owner == class && catalog.get(id).is_none()).then_some(id))
            .collect();
        missing_class.sort_unstable();
        println!("CLASSBOOK_MISSING_CLASS\t{class}\t{missing_class:?}");
        for (&spec, info) in specs {
            let mut missing_spec: Vec<_> = info
                .spells
                .iter()
                .copied()
                .filter(|&id| catalog.get(id).is_none())
                .collect();
            missing_spec.sort_unstable();
            println!("CLASSBOOK_MISSING_SPEC\t{spec}\t{missing_spec:?}");
            let spec_count = info
                .spells
                .iter()
                .filter(|id| {
                    catalog
                        .get(**id)
                        .is_some_and(|spell| spell.spellbook.lists(true))
                })
                .count();
            let book_error =
                load_class_preview_state(&data, PlayerSpellsTab::Spellbook, class, spec)
                    .err()
                    .unwrap_or_default();
            let talent_error =
                load_class_preview_state(&data, PlayerSpellsTab::Talents, class, spec)
                    .err()
                    .unwrap_or_default();
            let graph = load_talent_page(
                &data
                    .join("db2")
                    .join(game_engine_core::spell_catalog::SPELL_DB2_BUILD),
                class,
                spec,
            );
            match graph {
                Ok(graph) => {
                    let nodes = graph
                        .class
                        .nodes
                        .iter()
                        .chain(&graph.spec.nodes)
                        .chain(graph.heroes.iter().flat_map(|tree| &tree.nodes))
                        .chain(graph.hero_selection.iter());
                    for node in nodes.clone() {
                        let icons: Vec<_> = node
                            .entries
                            .iter()
                            .filter(|entry| entry.subtree_id == 0)
                            .map(|entry| {
                                if entry.override_icon != 0 {
                                    entry.override_icon
                                } else {
                                    catalog
                                        .get(entry.spell_id)
                                        .map_or(0, |spell| spell.icon_fdid)
                                }
                            })
                            .collect();
                        println!("CLASSBOOK_NODE_ICONS\t{spec}\t{}\t{icons:?}", node.id);
                    }
                    let missing = nodes
                        .filter(|node| {
                            node.entries.iter().any(|entry| {
                                if entry.subtree_id != 0 {
                                    return false;
                                }
                                let icon = if entry.override_icon != 0 {
                                    entry.override_icon
                                } else {
                                    catalog
                                        .get(entry.spell_id)
                                        .map_or(0, |spell| spell.icon_fdid)
                                };
                                icon == 0 || !data.join(format!("textures/{icon}.blp")).is_file()
                            })
                        })
                        .count();
                    let hero_nodes: usize = graph.heroes.iter().map(|tree| tree.nodes.len()).sum();
                    println!(
                        "CLASSBOOK_SPEC\t{class}\t{spec}\t{}\t{spec_count}\t{}\t{}\t{}\t{hero_nodes}\t{missing}\t{book_error}\t{talent_error}",
                        info.name,
                        graph.class.nodes.len(),
                        graph.spec.nodes.len(),
                        graph.heroes.len()
                    );
                }
                Err(error) => println!(
                    "CLASSBOOK_SPEC\t{class}\t{spec}\t{}\t{spec_count}\t0\t0\t0\t0\t0\t{book_error}\t{error}",
                    info.name
                ),
            }
        }
    }
}
