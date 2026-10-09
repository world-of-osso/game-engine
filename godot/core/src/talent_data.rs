//! Read-only local DB2 talent projection; see docs/specs/talents.md.
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentEntry {
    pub id: u32,
    pub spell_id: u32,
    pub override_icon: u32,
    pub max_ranks: u32,
    pub entry_type: u32,
    pub subtree_id: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentNode {
    pub id: u32,
    pub position: [f32; 2],
    pub node_type: u32,
    pub flags: u32,
    pub granted_ranks: u32,
    pub entries: Vec<TalentEntry>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentEdge {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub visual_style: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentTree {
    pub id: u32,
    pub name: String,
    pub nodes: Vec<TalentNode>,
    pub edges: Vec<TalentEdge>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentPage {
    pub tree_id: u32,
    pub class: TalentTree,
    pub spec: TalentTree,
    pub heroes: Vec<TalentTree>,
    pub hero_selection: Option<TalentNode>,
    pub starter_loadout_id: Option<u32>,
}
pub fn load_talent_page(_dir: &Path, _class: u32, _spec: u32) -> Result<TalentPage, String> {
    Ok(TalentPage::default())
}
