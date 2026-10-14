//! Derive legal level-90 AV defensive/healing purchases from the local source tree.
//! Output is a normal CommitTraitConfig request, never a spell/stat DB grant.
use game_engine_core::talent_data::{
    load_talent_page, load_trait_rules,
    rule_data::{TraitNode, TraitNodeEntry, TraitNodeType, TraitTree},
    rules::{ConfigEntry, Purchase, TraitContext, build_config},
};
use shared::protocol::{CommitTraitConfig, TraitEntrySelection};
use std::path::{Path, PathBuf};

// Last Stand12975 exists only in legacy tree880, not active Protection tree850.
const PROTECTION_SPELLS: &[u32] = &[871, 1160, 190456];
const HOLY_SPELLS: &[u32] = &[2050, 47788, 64843];

fn read_source_tree(data: &Path, class: u32, spec: u32) -> Result<TraitTree, String> {
    let page = load_talent_page(data, class, spec)?;
    load_trait_rules(data, page.tree_id)
}

fn plan_trait_commit(
    tree: &TraitTree,
    context: TraitContext,
    wanted: &[u32],
) -> Result<CommitTraitConfig, String> {
    reject_missing_source_spells(tree, wanted)?;
    let candidates = order_main_tree_candidates(tree, wanted);
    let mut purchases = Vec::new();
    loop {
        let config = build_config(tree, context, &purchases)?;
        let missing = missing_spells(tree, &config, wanted);
        if missing.is_empty() {
            return encode_trait_commit(context.spec_id, &config);
        }
        purchases = buy_available_rank(tree, context, &purchases, &candidates)
            .ok_or_else(|| format!("Source tree {} cannot allocate requested spells {missing:?} within level {} budgets", tree.tree_id, context.level))?;
    }
}

fn order_main_tree_candidates<'a>(
    tree: &'a TraitTree,
    wanted: &[u32],
) -> Vec<(&'a TraitNode, &'a TraitNodeEntry)> {
    let mut candidates: Vec<_> = tree
        .nodes
        .iter()
        .filter(|node| node.sub_tree_id == 0 && node.kind != TraitNodeType::SubTreeSelection)
        .flat_map(|node| {
            node.entries
                .iter()
                .enumerate()
                .map(move |(index, entry)| (node, entry, index))
        })
        .collect();
    candidates.sort_by_key(|(node, entry, index)| {
        (
            !wanted.contains(&entry.spell_id),
            node.pos_y,
            node.pos_x,
            node.id,
            *index,
        )
    });
    candidates
        .into_iter()
        .map(|(node, entry, _)| (node, entry))
        .collect()
}

fn reject_missing_source_spells(tree: &TraitTree, wanted: &[u32]) -> Result<(), String> {
    let missing: Vec<_> = wanted
        .iter()
        .copied()
        .filter(|spell| {
            !tree
                .nodes
                .iter()
                .flat_map(|node| &node.entries)
                .any(|entry| entry.spell_id == *spell)
        })
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Requested spells {missing:?} are absent from source trait tree {}",
            tree.tree_id
        ))
    }
}

fn missing_spells(tree: &TraitTree, config: &[ConfigEntry], wanted: &[u32]) -> Vec<u32> {
    wanted
        .iter()
        .copied()
        .filter(|spell| {
            !config.iter().any(|selected| {
                selected.total() > 0
                    && tree
                        .node(selected.node_id)
                        .and_then(|node| node.entry(selected.entry_id))
                        .is_some_and(|entry| entry.spell_id == *spell)
            })
        })
        .collect()
}

fn buy_available_rank(
    tree: &TraitTree,
    context: TraitContext,
    purchases: &[Purchase],
    candidates: &[(&TraitNode, &TraitNodeEntry)],
) -> Option<Vec<Purchase>> {
    for (node, entry) in candidates {
        let ranks = purchases
            .iter()
            .find(|purchase| purchase.node_id == node.id && purchase.entry_id == entry.id)
            .map_or(0, |purchase| purchase.ranks);
        if ranks >= entry.max_ranks {
            continue;
        }
        let candidate = add_purchased_rank(purchases, node.id, entry.id);
        // Refusal is normal availability: the same validator checks prerequisites,
        // choice conflicts, gates and currency budgets that the server enforces.
        if build_config(tree, context, &candidate).is_ok() {
            return Some(candidate);
        }
    }
    None
}

fn add_purchased_rank(purchases: &[Purchase], node_id: u32, entry_id: u32) -> Vec<Purchase> {
    let mut next = purchases.to_vec();
    if let Some(selected) = next
        .iter_mut()
        .find(|purchase| purchase.node_id == node_id && purchase.entry_id == entry_id)
    {
        selected.ranks += 1;
    } else {
        next.push(Purchase {
            node_id,
            entry_id,
            ranks: 1,
        });
    }
    next
}

fn encode_trait_commit(spec_id: u32, config: &[ConfigEntry]) -> Result<CommitTraitConfig, String> {
    let entries = config
        .iter()
        .map(|selected| {
            let rank = u8::try_from(selected.total())
                .map_err(|error| format!("Trait entry {} rank: {error}", selected.entry_id))?;
            Ok(TraitEntrySelection {
                node_id: selected.node_id,
                entry_id: selected.entry_id,
                rank,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CommitTraitConfig { spec_id, entries })
}

fn write_role_fixture(
    data: &Path,
    output: &Path,
    class: u32,
    spec: u32,
    wanted: &[u32],
) -> Result<(), String> {
    let tree = read_source_tree(data, class, spec)?;
    let request = plan_trait_commit(
        &tree,
        TraitContext {
            spec_id: spec,
            level: 90,
        },
        wanted,
    )?;
    let file = output.join(format!("trait-{spec}.json"));
    let json = serde_json::to_string_pretty(&request).map_err(|error| error.to_string())?;
    std::fs::write(&file, json).map_err(|error| format!("Write {}: {error}", file.display()))
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let data = PathBuf::from(args.next().ok_or("Expected source DB2 directory")?);
    let output = PathBuf::from(args.next().ok_or("Expected fixture output directory")?);
    if args.next().is_some() {
        return Err("Expected exactly source and output directories".into());
    }
    std::fs::create_dir_all(&output)
        .map_err(|error| format!("Create {}: {error}", output.display()))?;
    write_role_fixture(&data, &output, 1, 73, PROTECTION_SPELLS)?;
    write_role_fixture(&data, &output, 5, 257, HOLY_SPELLS)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_data() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933")
    }

    fn assert_requested_spells(class: u32, spec: u32, wanted: &[u32]) {
        let tree = read_source_tree(&source_data(), class, spec).unwrap();
        let request = plan_trait_commit(
            &tree,
            TraitContext {
                spec_id: spec,
                level: 90,
            },
            wanted,
        )
        .unwrap();
        for spell in wanted {
            assert!(
                request.entries.iter().any(|selection| selection.rank > 0
                    && tree
                        .node(selection.node_id)
                        .unwrap()
                        .entry(selection.entry_id)
                        .unwrap()
                        .spell_id
                        == *spell),
                "source build must purchase spell {spell}"
            );
        }
    }

    #[test]
    fn source_protection_build_purchases_wall_shout_and_ignore_pain() {
        assert_requested_spells(1, 73, PROTECTION_SPELLS);
    }

    #[test]
    fn active_protection_tree_refuses_legacy_last_stand_without_substitution() {
        let tree = read_source_tree(&source_data(), 1, 73).unwrap();
        let result = plan_trait_commit(
            &tree,
            TraitContext {
                spec_id: 73,
                level: 90,
            },
            &[12975],
        );
        assert_eq!(
            result,
            Err("Requested spells [12975] are absent from source trait tree 850".into())
        );
    }

    #[test]
    fn source_holy_build_purchases_serenity_guardian_and_hymn() {
        assert_requested_spells(5, 257, HOLY_SPELLS);
    }
}
