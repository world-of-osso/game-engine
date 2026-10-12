//! Mainline ProfessionsSpecFrameMixin radial layout; TraitNode.PosX/Y are zero for these trees.
use shared::protocol::ProfessionSpecTreeSnapshot;
use std::collections::{BTreeMap, BTreeSet};

type Centers = BTreeMap<u32, [f32; 2]>;
type Children = BTreeMap<u32, Vec<u32>>;

pub(super) fn path_centers(tree: &ProfessionSpecTreeSnapshot) -> Result<Centers, String> {
    let path_ids: BTreeSet<_> = tree
        .nodes
        .iter()
        .filter(|node| node.kind == 1 || node.node_id == tree.root_node_id)
        .map(|node| node.node_id)
        .collect();
    let children = collect_children(tree, &path_ids);
    let primary = children
        .get(&tree.root_node_id)
        .cloned()
        .unwrap_or_default();
    let max_primary = primary.len().max(1);
    let max_secondary = primary
        .iter()
        .map(|id| children.get(id).map_or(0, Vec::len))
        .max()
        .unwrap_or(0)
        .max(1);
    let (primary_rule, secondary_rule) = layout_rules(max_primary, max_secondary)?;
    // Retail root(3470,2910) /10, relative to TreeView(2,85).
    let root = [349.0, 376.0];
    let mut centers = Centers::from([(tree.root_node_id, root)]);
    for (id, center, rotation) in place_children(root, 0.0, &primary, primary_rule) {
        centers.insert(id, center);
        let secondary = children.get(&id).map_or(&[][..], Vec::as_slice);
        for (leaf, center, _) in place_children(center, rotation, secondary, secondary_rule) {
            if children
                .get(&leaf)
                .is_some_and(|children| !children.is_empty())
            {
                return Err(format!(
                    "Retail profession tree {} exceeds two path layers at {leaf}",
                    tree.tree_id
                ));
            }
            centers.insert(leaf, center);
        }
    }
    let missing: Vec<_> = path_ids
        .into_iter()
        .filter(|id| !centers.contains_key(id))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Profession tree {} lacks source path edges for {missing:?}",
            tree.tree_id
        ));
    }
    Ok(centers)
}
fn collect_children(tree: &ProfessionSpecTreeSnapshot, paths: &BTreeSet<u32>) -> Children {
    let mut children = Children::new();
    let mut seen = BTreeSet::new();
    for edge in &tree.edges {
        let pair = (edge.left_node_id, edge.right_node_id);
        if edge.visual_style != 0
            && paths.contains(&pair.0)
            && paths.contains(&pair.1)
            && seen.insert(pair)
        {
            children.entry(pair.0).or_default().push(pair.1);
        }
    }
    children
}
fn place_children(
    parent: [f32; 2],
    parent_rotation: f32,
    children: &[u32],
    rule: (f32, f32),
) -> Vec<(u32, [f32; 2], f32)> {
    let pivot = children.len() as f32 * 0.5 - 0.5;
    children
        .iter()
        .enumerate()
        .map(|(index, &id)| {
            let rotation = parent_rotation + (index as f32 - pivot) * rule.0;
            let angle = rotation.to_radians();
            (
                id,
                [
                    parent[0] + angle.sin() * rule.1,
                    parent[1] + angle.cos() * rule.1,
                ],
                rotation,
            )
        })
        .collect()
}
fn layout_rules(primary: usize, secondary: usize) -> Result<((f32, f32), (f32, f32)), String> {
    // Specializations.lua PathLayoutInfo; distances scaled by TalentButtonUtil /10.
    let root = match (primary, secondary) {
        (1, 1..=5) => (0.0, 120.0),
        (2, 1) | (2, 4) => (90.0, 120.0),
        (2, 2..=3) => (80.0, 120.0),
        (2, 5) => (100.0, 130.0),
        (3, 1..=4) => (70.0, 120.0),
        (3, 5) => (60.0, 130.0),
        (4, 1) => (50.0, 120.0),
        (4, 2) => (60.0, 120.0),
        (4, 3) => (55.0, 120.0),
        (5, 1..=2) => (40.0, 120.0),
        _ => {
            return Err(format!(
                "No Retail profession path layout for {primary} primary/{secondary} secondary children"
            ));
        }
    };
    let leaf = match (primary, secondary) {
        (1, 1) | (2, 1) | (3, 1) | (4, 1) | (5, 1) => 0.0,
        (1, 2) | (3, 2) | (4, 2) | (5, 2) => 80.0,
        (1, 3) => 60.0,
        (1, 4) => 50.0,
        (1, 5) | (2, 2..=4) | (3, 3..=4) => 40.0,
        (2, 5) => 35.0,
        (3, 5) | (4, 3) => 30.0,
        _ => {
            return Err(format!(
                "No Retail secondary path layout for {primary}/{secondary}"
            ));
        }
    };
    Ok((root, (leaf, 120.0)))
}
