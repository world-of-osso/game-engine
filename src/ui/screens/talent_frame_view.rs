//! Builds the talent window view model from a class tree, the edited config and
//! the spell catalog: node placement per section, node looks, edges, labels and
//! hover tooltips.

use std::collections::HashMap;

use bevy::prelude::Resource;

use super::talent_frame_component::{
    CHOICE_W, ChoiceIconView, EdgeView, FRAME_W, NODE_SIZE, NodeLook, NodeShape, NodeView,
    SectionLabel, SpecButtonView, TREE_BOTTOM, TREE_INSET, TREE_TOP, TalentTreeView,
    talent_choice_name, talent_node_name,
};
use crate::spell_catalog::SpellCatalog;
use crate::talent_tree::rules::{self, ConfigEntry};
use crate::talent_tree::session::{TalentSession, is_choice, total_ranks};
use crate::talent_tree::{
    CURRENCY_FLAG_CLASS, CURRENCY_FLAG_SPEC, NodeKind, SPEC_UNLOCK_LEVEL, TalentEntry, TalentNode,
    TalentTree, TalentTreeData,
};

/// Retail `INV_Misc_QuestionMark`, shown when an entry has no known icon.
const UNKNOWN_ICON_FDID: u32 = 134400;
const SECTION_GAP: f32 = 28.0;
/// Space between the hero selection node and the hero sub-tree below it.
const HERO_SELECT_GAP: f32 = 18.0;

#[derive(Clone, Debug, PartialEq)]
pub struct TalentTooltip {
    pub title: String,
    pub rank: String,
    pub description: String,
}

/// Tooltip content per talent frame name (`TalentNode_<id>`, choice halves).
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct TalentTooltips {
    pub by_frame: HashMap<String, TalentTooltip>,
}

pub struct TalentViewInput<'a> {
    pub data: &'a TalentTreeData,
    pub session: &'a TalentSession<'a>,
    pub config: &'a [ConfigEntry],
    pub catalog: &'a SpellCatalog,
    pub has_pending: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Class,
    Spec,
    HeroSelect,
    Hero,
}

fn section_of(tree: &TalentTree, node: &TalentNode) -> Section {
    if node.kind == NodeKind::SubTreeSelection {
        return Section::HeroSelect;
    }
    if node.sub_tree_id != 0 {
        return Section::Hero;
    }
    let flags = node
        .entries
        .first()
        .map(|entry| {
            rules::entry_costs(tree, node, entry.id).fold(0, |flags, cost| {
                flags | tree.currency_flags(cost.currency_id)
            })
        })
        .unwrap_or(0);
    if flags & CURRENCY_FLAG_CLASS != 0 {
        Section::Class
    } else {
        Section::Spec
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl Bounds {
    fn of<'n>(nodes: impl Iterator<Item = &'n TalentNode>) -> Option<Self> {
        nodes.fold(None, |bounds: Option<Bounds>, node| {
            let (x, y) = (node.pos_x, node.pos_y);
            Some(match bounds {
                None => Bounds {
                    min_x: x,
                    max_x: x,
                    min_y: y,
                    max_y: y,
                },
                Some(b) => Bounds {
                    min_x: b.min_x.min(x),
                    max_x: b.max_x.max(x),
                    min_y: b.min_y.min(y),
                    max_y: b.max_y.max(y),
                },
            })
        })
    }

    fn width(&self) -> f32 {
        (self.max_x - self.min_x) as f32
    }

    fn height(&self) -> f32 {
        (self.max_y - self.min_y) as f32
    }
}

/// Maps tree coordinates of one section to frame coordinates.
#[derive(Clone, Copy)]
struct Placement {
    origin_x: f32,
    origin_y: f32,
    min_x: i32,
    min_y: i32,
    scale: f32,
}

impl Placement {
    fn center(&self, node: &TalentNode) -> (f32, f32) {
        (
            self.origin_x + NODE_SIZE / 2.0 + (node.pos_x - self.min_x) as f32 * self.scale,
            self.origin_y + NODE_SIZE / 2.0 + (node.pos_y - self.min_y) as f32 * self.scale,
        )
    }
}

struct Layout {
    placed: HashMap<u32, (f32, f32)>,
    class_label: Option<(f32, f32)>,
    hero_label: Option<(f32, f32)>,
    spec_label: Option<(f32, f32)>,
}

/// Class tree left, hero tree middle, spec tree right, all at one scale that
/// fits the tree area. Class and spec rows share their top so rows line up.
fn layout(tree: &TalentTree, visible: &[&TalentNode], hero_sub_tree: Option<u32>) -> Layout {
    let in_section = |section| {
        visible
            .iter()
            .copied()
            .filter(move |node| section_of(tree, node) == section)
    };
    let hero_nodes =
        || in_section(Section::Hero).filter(|node| Some(node.sub_tree_id) == hero_sub_tree);
    let class = Bounds::of(in_section(Section::Class));
    let spec = Bounds::of(in_section(Section::Spec));
    let hero = Bounds::of(hero_nodes());
    let has_hero_select = in_section(Section::HeroSelect).next().is_some();
    let units_w = class.map_or(0.0, |b| b.width())
        + spec.map_or(0.0, |b| b.width())
        + hero.map_or(0.0, |b| b.width());
    let units_h = class
        .map_or(0.0, |b| b.height())
        .max(spec.map_or(0.0, |b| b.height()));
    let columns = [class.is_some(), has_hero_select, spec.is_some()]
        .iter()
        .filter(|present| **present)
        .count() as f32;
    let avail_w = FRAME_W - 2.0 * TREE_INSET - columns * CHOICE_W - (columns - 1.0) * SECTION_GAP;
    let avail_h = TREE_BOTTOM - TREE_TOP - NODE_SIZE;
    let scale = (avail_w / units_w.max(1.0)).min(avail_h / units_h.max(1.0));
    let column_w = |bounds: Option<Bounds>| bounds.map_or(0.0, |b| b.width() * scale) + CHOICE_W;
    let class_w = class.map(|b| b.width() * scale + NODE_SIZE);
    let hero_w = has_hero_select.then(|| column_w(hero));
    let spec_w = spec.map(|b| b.width() * scale + NODE_SIZE);
    let total_w = [class_w, hero_w, spec_w].iter().flatten().sum::<f32>()
        + (columns - 1.0).max(0.0) * SECTION_GAP;
    let top_y = [class, spec]
        .iter()
        .flatten()
        .map(|b| b.min_y)
        .min()
        .unwrap_or(0);

    let mut placed = HashMap::new();
    let mut x = (FRAME_W - total_w) / 2.0;
    let place_section = |bounds: Bounds,
                         origin: (f32, f32),
                         min_y: i32,
                         nodes: &mut dyn Iterator<Item = &TalentNode>| {
        let placement = Placement {
            origin_x: origin.0,
            origin_y: origin.1,
            min_x: bounds.min_x,
            min_y,
            scale,
        };
        nodes
            .map(|node| (node.id, placement.center(node)))
            .collect::<Vec<_>>()
    };
    let mut class_label = None;
    if let (Some(bounds), Some(width)) = (class, class_w) {
        placed.extend(place_section(
            bounds,
            (x, TREE_TOP),
            top_y,
            &mut in_section(Section::Class),
        ));
        class_label = Some((x, width));
        x += width + SECTION_GAP;
    }
    let mut hero_label = None;
    if let Some(width) = hero_w {
        let center_x = x + width / 2.0;
        for node in in_section(Section::HeroSelect) {
            placed.insert(node.id, (center_x, TREE_TOP + NODE_SIZE / 2.0));
        }
        if let Some(bounds) = hero {
            let origin_x = center_x - (bounds.width() * scale + NODE_SIZE) / 2.0;
            let origin_y = TREE_TOP + NODE_SIZE + HERO_SELECT_GAP;
            placed.extend(place_section(
                bounds,
                (origin_x, origin_y),
                bounds.min_y,
                &mut hero_nodes(),
            ));
        }
        hero_label = Some((x, width));
        x += width + SECTION_GAP;
    }
    let mut spec_label = None;
    if let (Some(bounds), Some(width)) = (spec, spec_w) {
        placed.extend(place_section(
            bounds,
            (x, TREE_TOP),
            top_y,
            &mut in_section(Section::Spec),
        ));
        spec_label = Some((x, width));
    }
    Layout {
        placed,
        class_label,
        hero_label,
        spec_label,
    }
}

fn entry_name(entry: &TalentEntry, catalog: &SpellCatalog) -> String {
    if !entry.override_name.is_empty() {
        return entry.override_name.clone();
    }
    catalog
        .get(entry.spell_id)
        .map(|spell| spell.name.to_string())
        .unwrap_or_else(|| format!("Spell {}", entry.spell_id))
}

fn entry_icon(entry: &TalentEntry, catalog: &SpellCatalog) -> u32 {
    if entry.override_icon != 0 {
        return entry.override_icon;
    }
    catalog
        .get(entry.spell_id)
        .map(|spell| spell.icon_fdid)
        .filter(|&fdid| fdid != 0)
        .unwrap_or(UNKNOWN_ICON_FDID)
}

fn node_shape(node: &TalentNode) -> NodeShape {
    if is_choice(node) {
        NodeShape::Choice
    } else if node.entries.first().is_some_and(|entry| entry.passive) {
        NodeShape::Circle
    } else {
        NodeShape::Square
    }
}

/// (ranks held, max ranks) of the node; a choice node counts its chosen entry.
fn node_ranks(node: &TalentNode, config: &[ConfigEntry]) -> (u32, u32) {
    if is_choice(node) {
        let chosen = node
            .entries
            .iter()
            .find(|entry| total_ranks(config, node.id, entry.id) > 0);
        let entry = chosen.or(node.entries.first());
        return entry.map_or((0, 0), |entry| {
            (total_ranks(config, node.id, entry.id), entry.max_ranks)
        });
    }
    node.entries.iter().fold((0, 0), |(held, max), entry| {
        (
            held + total_ranks(config, node.id, entry.id),
            max + entry.max_ranks,
        )
    })
}

fn node_look(session: &TalentSession, config: &[ConfigEntry], node: &TalentNode) -> NodeLook {
    let (held, max) = node_ranks(node, config);
    if max > 0 && held >= max {
        return NodeLook::Maxed;
    }
    let can_add = if is_choice(node) {
        node.entries
            .iter()
            .any(|entry| session.try_add(config, node.id, entry.id).is_some())
    } else {
        session
            .add_target(config, node)
            .is_some_and(|entry_id| session.try_add(config, node.id, entry_id).is_some())
    };
    if can_add || held > 0 {
        NodeLook::Open
    } else {
        NodeLook::Locked
    }
}

fn border_art_name(shape: NodeShape, look: NodeLook) -> String {
    let shape = match shape {
        NodeShape::Square => "square",
        NodeShape::Circle => "circle",
        NodeShape::Choice => "choice",
    };
    let color = match look {
        NodeLook::Maxed => "yellow",
        NodeLook::Open => "green",
        NodeLook::Locked => "gray",
    };
    format!("talents-node-{shape}-{color}")
}

fn tooltip(entry: &TalentEntry, held: u32, catalog: &SpellCatalog) -> TalentTooltip {
    TalentTooltip {
        title: entry_name(entry, catalog),
        rank: format!("Rank {held}/{}", entry.max_ranks),
        description: catalog
            .render_description(entry.spell_id)
            .unwrap_or_default(),
    }
}

fn node_view(
    input: &TalentViewInput,
    node: &TalentNode,
    (x, y): (f32, f32),
    tooltips: &mut TalentTooltips,
) -> NodeView {
    let config = input.config;
    let shape = node_shape(node);
    let look = node_look(input.session, config, node);
    let (held, max) = node_ranks(node, config);
    let chosen = |entry: &TalentEntry| total_ranks(config, node.id, entry.id) > 0;
    let icon_entries: Vec<&TalentEntry> = if is_choice(node) {
        node.entries.iter().take(2).collect()
    } else {
        node.entries.first().into_iter().collect()
    };
    let icons = icon_entries
        .iter()
        .map(|entry| ChoiceIconView {
            entry_id: entry.id,
            icon_fdid: entry_icon(entry, input.catalog),
            chosen: chosen(entry),
        })
        .collect();
    let shown = node
        .entries
        .iter()
        .find(|entry| chosen(entry))
        .or(node.entries.first());
    if let Some(entry) = shown {
        let entry_held = total_ranks(config, node.id, entry.id);
        tooltips.by_frame.insert(
            talent_node_name(node.id),
            tooltip(entry, entry_held, input.catalog),
        );
    }
    if is_choice(node) {
        for entry in &icon_entries {
            let entry_held = total_ranks(config, node.id, entry.id);
            tooltips.by_frame.insert(
                talent_choice_name(node.id, entry.id),
                tooltip(entry, entry_held, input.catalog),
            );
        }
    }
    NodeView {
        node_id: node.id,
        x,
        y,
        shape,
        look,
        rank_text: format!("{held}/{max}"),
        border: input.data.art(&border_art_name(shape, look)),
        icons,
    }
}

fn edge_views(
    input: &TalentViewInput,
    visible: &[&TalentNode],
    placed: &HashMap<u32, (f32, f32)>,
) -> Vec<EdgeView> {
    let tree = input.session.tree;
    let mut edges = Vec::new();
    for node in visible {
        let Some(&(to_x, to_y)) = placed.get(&node.id) else {
            continue;
        };
        for &(parent_id, _) in &node.parents {
            let (Some(&(from_x, from_y)), Some(parent)) =
                (placed.get(&parent_id), tree.node(parent_id))
            else {
                continue;
            };
            let (dx, dy) = (to_x - from_x, to_y - from_y);
            let distance = (dx * dx + dy * dy).sqrt();
            let length = (distance - NODE_SIZE).max(0.0);
            if length <= 0.0 {
                continue;
            }
            let (held, max) = node_ranks(parent, input.config);
            let color = if max > 0 && held >= max {
                "yellow"
            } else {
                "gray"
            };
            edges.push(EdgeView {
                from: parent_id,
                to: node.id,
                x: (from_x + to_x) / 2.0,
                y: (from_y + to_y) / 2.0,
                length,
                angle: dy.atan2(dx),
                art: input.data.art(&format!("talents-arrow-line-{color}")),
            });
        }
    }
    edges
}

/// Unspent points of the currency the node's first entry costs.
fn node_currency(tree: &TalentTree, node: &TalentNode) -> Option<u32> {
    let entry = node.entries.first()?;
    rules::entry_costs(tree, node, entry.id)
        .next()
        .map(|cost| cost.currency_id)
}

fn section_label(text: String, placed: Option<(f32, f32)>) -> Option<SectionLabel> {
    placed.map(|(x, width)| SectionLabel { text, x, width })
}

pub fn build_tree_view(input: &TalentViewInput) -> (TalentTreeView, TalentTooltips) {
    let session = input.session;
    let tree = session.tree;
    let ctx = session.ctx;
    let hero_sub_tree = rules::selected_sub_tree(tree, input.config);
    let visible: Vec<&TalentNode> = tree
        .nodes
        .iter()
        .filter(|node| !node.entries.is_empty() && rules::node_visible(tree, ctx, node))
        .filter(|node| node.sub_tree_id == 0 || Some(node.sub_tree_id) == hero_sub_tree)
        .collect();
    let layout = layout(tree, &visible, hero_sub_tree);
    let mut tooltips = TalentTooltips::default();
    let nodes = visible
        .iter()
        .filter_map(|node| {
            let center = *layout.placed.get(&node.id)?;
            Some(node_view(input, node, center, &mut tooltips))
        })
        .collect();
    let edges = edge_views(input, &visible, &layout.placed);

    let unspent = session.unspent(input.config);
    let points = |flag: u32| {
        unspent
            .iter()
            .find(|(currency, _)| tree.currency_flags(*currency) & flag != 0)
            .map_or(0, |(_, amount)| *amount)
    };
    let class_name = input.data.class_name(tree.class_id).unwrap_or("Class");
    let spec_name = tree
        .specs
        .iter()
        .find(|spec| spec.id == ctx.spec_id)
        .map_or("Specialization", |spec| spec.name.as_str());
    let hero_label = hero_label(input, &visible, hero_sub_tree, &unspent);
    let view = TalentTreeView {
        background: input.data.spec_background(tree, ctx.spec_id),
        class_label: section_label(
            format!("{class_name}  {}", points(CURRENCY_FLAG_CLASS)),
            layout.class_label,
        ),
        spec_label: section_label(
            format!("{spec_name}  {}", points(CURRENCY_FLAG_SPEC)),
            layout.spec_label,
        ),
        hero_label: section_label(hero_label, layout.hero_label),
        specs: tree
            .specs
            .iter()
            .filter(|spec| spec.selectable())
            .map(|spec| SpecButtonView {
                spec_id: spec.id,
                name: spec.name.clone(),
                active: spec.id == ctx.spec_id,
                enabled: ctx.level >= SPEC_UNLOCK_LEVEL,
            })
            .collect(),
        nodes,
        edges,
        has_pending: input.has_pending,
    };
    (view, tooltips)
}

/// Selected hero tree name and its points; before a pick, the first visible option's.
fn hero_label(
    input: &TalentViewInput,
    visible: &[&TalentNode],
    selected: Option<u32>,
    unspent: &[(u32, i32)],
) -> String {
    let tree = input.session.tree;
    let sub_tree = selected.or_else(|| {
        visible
            .iter()
            .find(|node| node.kind == NodeKind::SubTreeSelection)
            .and_then(|node| node.entries.first())
            .map(|entry| entry.sub_tree_id)
    });
    let Some(sub_tree) = sub_tree else {
        return "Hero Talents".to_string();
    };
    let name = if selected.is_some() {
        tree.sub_tree_name(sub_tree).unwrap_or("Hero Talents")
    } else {
        "Hero Talents"
    };
    let currency = tree
        .nodes
        .iter()
        .filter(|node| node.sub_tree_id == sub_tree)
        .find_map(|node| node_currency(tree, node));
    match currency.and_then(|id| unspent.iter().find(|(c, _)| *c == id)) {
        Some((_, amount)) => format!("{name}  {amount}"),
        None => name.to_string(),
    }
}
