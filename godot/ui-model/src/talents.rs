//! Read-only UI projection over the core DB2 graph; no allocations or network state.
use game_engine_core::spell_catalog::{SPELL_DB2_BUILD, SpellCatalogData};
use game_engine_core::talent_data::{TalentNode, TalentPage, load_talent_page};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentView {
    pub graph: TalentPage,
    /// Entry ID -> actual definition override or spell icon FDID.
    pub icons: BTreeMap<u32, u32>,
    /// These retain graph/spell metadata, but never reach the placeholder-art loader.
    pub missing_icons: BTreeSet<u32>,
}
impl TalentView {
    pub fn nodes(&self) -> impl Iterator<Item = &TalentNode> {
        self.graph
            .class
            .nodes
            .iter()
            .chain(&self.graph.spec.nodes)
            .chain(self.graph.heroes.iter().flat_map(|tree| &tree.nodes))
    }
}

pub fn load_talent_view(
    data: &Path,
    class: u32,
    spec: u32,
    catalog: &SpellCatalogData,
) -> Result<TalentView, String> {
    let graph = load_talent_page(&data.join("db2").join(SPELL_DB2_BUILD), class, spec)?;
    let mut view = TalentView {
        graph,
        ..Default::default()
    };
    view.icons = collect_talent_icons(&view, catalog)?;
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
            let fdid = if entry.override_icon != 0 {
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

/// Entry buttons carry no onclick. This shared identity is also the live tooltip source.
pub fn talent_button_spell(name: &str) -> Option<u32> {
    let node = name.strip_prefix("TalentNode")?;
    let (_, spell) = node.rsplit_once("Spell")?;
    spell.strip_suffix("Button")?.parse().ok()
}

/// Staged configuration for the owning player.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentEditor {
    pub snapshot: Option<shared::protocol::TraitConfigSnapshot>,
    pub error_text: Option<String>,
    pub choice_node: Option<u32>,
}
impl TalentEditor {
    pub fn receive_snapshot(&mut self, _snapshot: shared::protocol::TraitConfigSnapshot) {}
    pub fn receive_result(&mut self, _result: shared::protocol::TraitCommitResult) {}
    pub fn rank(&self, _node: u32, _entry: u32) -> u8 {
        0
    }
    pub fn unspent(&self, _view: &TalentView) -> Vec<(u32, i32)> {
        Vec::new()
    }
    pub fn dirty(&self) -> bool {
        false
    }
    pub fn purchase(&mut self, _view: &TalentView, _level: u8, _node: u32, _entry: u32) -> bool {
        false
    }
    pub fn refund(&mut self, _view: &TalentView, _level: u8, _node: u32) -> bool {
        false
    }
    pub fn undo(&mut self) {}
    pub fn apply(&self) -> Option<shared::protocol::CommitTraitConfig> {
        None
    }
}
