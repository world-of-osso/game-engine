use game_engine_core::{
    blp,
    spell_catalog::{SpellCatalogPaths, load_spell_catalog},
};
use game_engine_ui_model::talents::load_talent_view;
use std::{collections::BTreeMap, path::PathBuf};

// Hero selectors are navigation entries, not spell icons. These local Mainline
// Monk entries have no authored spell/icon: 124870 -> definition129708,
// 124883 -> definition129721, 125051 -> definition0. Preserve graph/ranks;
// blank art is an explicit source-data exception, never a replacement spell.
const UNDEFINED_MONK_ENTRIES: &[u32] = &[124870, 124883, 125051];
#[test]
fn every_specialization_talent_entry_has_authentic_shipped_icon() {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    let mut decoded = BTreeMap::<u32, Result<(), String>>::new();
    let mut failures = Vec::new();
    let mut specs = 0;
    for (&spec, metadata) in catalog
        .tabs
        .specs
        .iter()
        .filter(|(_, s)| !s.initial && s.class_id != 0)
    {
        specs += 1;
        let view = match load_talent_view(&data, metadata.class_id, spec, &catalog) {
            Ok(view) => view,
            Err(error) => {
                failures.push(format!("spec {spec}: {error}"));
                continue;
            }
        };
        if spec == 70 {
            let node = view
                .graph
                .class
                .nodes
                .iter()
                .find(|node| node.id == 81598)
                .unwrap();
            let entry = &node.entries[0];
            assert_eq!(
                (entry.id, entry.spell_id, entry.override_icon),
                (102584, 115750, 0)
            );
            // TraitDefinition107589 -> SpellMisc88700 -> shipped571553.blp.
            assert_eq!(view.icons[&entry.id], 571553);
        }
        let mut total = 0;
        let mut resolved = 0;
        for node in view.nodes() {
            for entry in &node.entries {
                if entry.subtree_id != 0 {
                    continue;
                }
                total += 1;
                let fdid = view.icons[&entry.id];
                if UNDEFINED_MONK_ENTRIES.contains(&entry.id) {
                    assert_eq!((entry.spell_id, entry.override_icon, fdid), (0, 0, 0));
                    assert_eq!(view.names[&entry.id], "");
                    println!("TALENT_ICON_EXCEPTION\t{spec}\t{}", entry.id);
                    continue;
                }
                let result = decoded.entry(fdid).or_insert_with(|| {
                    let bytes = std::fs::read(data.join(format!("textures/{fdid}.blp")))
                        .map_err(|e| e.to_string())?;
                    let image = blp::decode_rgba(&bytes)?;
                    if !image
                        .pixels
                        .chunks_exact(4)
                        .any(|p| p[3] != 0 && p[..3].iter().any(|&c| c != 0))
                    {
                        return Err("empty decoded icon".into());
                    }
                    Ok(())
                });
                match result {
                    Ok(()) => resolved += 1,
                    Err(error) => failures.push(format!(
                        "spec {spec} node {} entry {} spell {} icon {fdid}: {error}",
                        node.id, entry.id, entry.spell_id
                    )),
                }
            }
        }
        println!(
            "TALENT_ICONS\t{}\t{spec}\t{resolved}\t{total}\t{}\t{}\t{}",
            metadata.class_id,
            view.graph.class.nodes.len(),
            view.graph.spec.nodes.len(),
            view.graph
                .heroes
                .iter()
                .map(|t| t.nodes.len())
                .sum::<usize>()
        );
    }
    assert_eq!(specs, 40);
    assert!(
        failures.is_empty(),
        "{} unresolved talent entries: {failures:#?}",
        failures.len()
    );
}
