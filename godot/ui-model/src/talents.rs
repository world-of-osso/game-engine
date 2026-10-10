//! UI projection and staged allocations over the local Retail DB2 graph.
#[path = "talent_search.rs"]
pub mod search;
#[path = "talent_editor.rs"]
mod talent_editor;
use game_engine_core::spell_catalog::{SPELL_DB2_BUILD, SpellCatalogData};
use game_engine_core::talent_data::{TalentNode, TalentPage, load_talent_page};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
pub use talent_editor::TalentEditor;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentView {
    pub graph: TalentPage,
    pub editor: TalentEditor,
    pub level: u8,
    pub names: BTreeMap<u32, String>,
    pub descriptions: BTreeMap<u32, String>,
    pub replaced_names: BTreeMap<u32, String>,
    pub on_action_bar: BTreeSet<u32>,
    pub rules: Option<game_engine_core::talent_data::rule_data::TraitTree>,
    /// Entry ID -> actual definition override or spell icon FDID.
    pub icons: BTreeMap<u32, u32>,
    /// These retain graph/spell metadata, but never reach the placeholder-art loader.
    pub missing_icons: BTreeSet<u32>,
}
impl TalentView {
    pub fn node_rank(&self, node: &TalentNode) -> u32 {
        if self.editor.snapshot.is_some() {
            self.editor.node_rank(node.id)
        } else {
            node.granted_ranks
        }
    }
    pub fn entry_rank(&self, node: &TalentNode, entry: u32) -> u32 {
        if self.editor.snapshot.is_some() {
            u32::from(self.editor.rank(node.id, entry))
        } else {
            node.granted_ranks
        }
    }
    pub fn nodes(&self) -> impl Iterator<Item = &TalentNode> {
        self.graph
            .class
            .nodes
            .iter()
            .chain(&self.graph.spec.nodes)
            .chain(self.graph.heroes.iter().flat_map(|tree| &tree.nodes))
            .chain(self.graph.hero_selection.iter())
    }
}

pub fn load_talent_view(
    data: &Path,
    class: u32,
    spec: u32,
    catalog: &SpellCatalogData,
) -> Result<TalentView, String> {
    let graph = load_talent_page(&data.join("db2").join(SPELL_DB2_BUILD), class, spec)?;
    let rules = game_engine_core::talent_data::load_trait_rules(
        &data.join("db2").join(SPELL_DB2_BUILD),
        graph.tree_id,
    )?;
    let mut view = TalentView {
        graph,
        rules: Some(rules),
        ..Default::default()
    };
    view.icons = collect_talent_icons(&view, catalog)?;
    view.names = view
        .nodes()
        .flat_map(|node| &node.entries)
        .map(|entry| {
            let name = if entry.subtree_id != 0 {
                view.graph
                    .heroes
                    .iter()
                    .find(|tree| tree.id == entry.subtree_id)
                    .map(|tree| tree.name.clone())
                    .ok_or_else(|| {
                        format!(
                            "Talent entry {} has no visible hero tree {}",
                            entry.id, entry.subtree_id
                        )
                    })?
            } else if entry.spell_id == 0 {
                // Local Mainline contains undefined Monk entries. Keep their
                // graph identity/ranks, but do not invent a spell name or art.
                String::new()
            } else {
                catalog
                    .get(entry.spell_id)
                    .map(|spell| spell.name.to_string())
                    .ok_or_else(|| {
                        format!("Talent entry {} has no spell {}", entry.id, entry.spell_id)
                    })?
            };
            Ok((entry.id, name))
        })
        .collect::<Result<_, String>>()?;
    let context = game_engine_core::spell_catalog::SpellTextContext {
        spec_id: Some(spec),
        ..Default::default()
    };
    view.descriptions = view
        .nodes()
        .flat_map(|node| &node.entries)
        .filter_map(|entry| {
            let description = if entry.subtree_id != 0 {
                view.graph
                    .heroes
                    .iter()
                    .find(|tree| tree.id == entry.subtree_id)
                    .map(|tree| tree.description.clone())
            } else {
                catalog.render_description(entry.spell_id, &context)
            };
            description.map(|text| (entry.id, text))
        })
        .collect();
    view.replaced_names = view
        .rules
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .flat_map(|node| &node.entries)
        .filter_map(|entry| {
            catalog
                .get(entry.overrides_spell_id)
                .map(|spell| (entry.id, spell.name.to_string()))
        })
        .collect();
    view.missing_icons = read_missing_icons(data, &view.icons)?;
    if !view.missing_icons.is_empty() {
        eprintln!(
            "Talents missing local icon FDIDs (no substitute art): {:?}",
            view.missing_icons
        );
    }
    Ok(view)
}
fn collect_talent_icons(
    view: &TalentView,
    catalog: &SpellCatalogData,
) -> Result<BTreeMap<u32, u32>, String> {
    view.nodes()
        .flat_map(|node| &node.entries)
        .map(|entry| {
            let fdid = if entry.spell_id == 0 && entry.override_icon == 0 {
                0
            } else if entry.override_icon != 0 {
                entry.override_icon
            } else {
                catalog
                    .get(entry.spell_id)
                    .ok_or_else(|| {
                        format!(
                            "Talent entry {}: missing local spell {}",
                            entry.id, entry.spell_id
                        )
                    })?
                    .icon_fdid
            };
            Ok((entry.id, fdid))
        })
        .collect()
}
fn read_missing_icons(data: &Path, icons: &BTreeMap<u32, u32>) -> Result<BTreeSet<u32>, String> {
    let mut missing = BTreeSet::new();
    for &fdid in icons.values() {
        if fdid == 0 {
            continue;
        }
        let path = data.join(format!("textures/{fdid}.blp"));
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => (),
            Ok(_) => return Err(format!("Talent icon {} is not a file", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.insert(fdid);
            }
            Err(error) => return Err(format!("Read talent icon {}: {error}", path.display())),
        }
    }
    Ok(missing)
}

/// Shared entry identity for the live spell tooltip source.
pub fn talent_button_node(name: &str) -> Option<u32> {
    name.strip_prefix("TalentNode")
        .or_else(|| name.strip_prefix("TalentChoiceNode"))?
        .split_once("Entry")?
        .0
        .parse()
        .ok()
}

pub fn talent_button_spell(name: &str) -> Option<u32> {
    let node = name
        .strip_prefix("TalentNode")
        .or_else(|| name.strip_prefix("TalentChoiceNode"))?;
    let (_, spell) = node.rsplit_once("Spell")?;
    spell.strip_suffix("Button")?.parse().ok()
}
