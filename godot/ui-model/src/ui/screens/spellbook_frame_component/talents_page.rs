//! Blizzard SharedTalentUtil.lua:494-509 and ClassTalentsFrame.xml:37-42.
//! Read-only hero previews are stacked, not activated: see docs/wiki/systems/talents.md.
use super::*;
use crate::talents::TalentView;
use game_engine_core::talent_data::{TalentNode, TalentTree};
use std::collections::BTreeMap;
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::widgets::texture::TextureSource;

const PAN: [f32; 2] = [49.0, 24.0];
const HERO_SCALE: f32 = 0.85;
// HeroSpecButton TOP102, height108; container TOP relative BOTTOM +34 (y-up),
// NodesContainer TOP -90 => 102+108-34+90 =266. OffsetY8 is applied to nodes.
const HERO_TOP: f32 = 266.0;
const HERO_PREVIEW_SPACING: f32 = 304.0;
const EDGE_COLOR: [f32; 4] = [0.35, 0.35, 0.35, 1.0];

pub(super) fn talents(state: &SpellbookFrameState, scale: f32) -> Element {
    let children = match &state.talents {
        Some(view) => page_contents(view, scale),
        None => heading(
            "TalentsContext",
            "Talents require a class and specialization",
            [BOOK_W / 2.0, BOOK_H / 2.0],
            scale,
        ),
    };
    rsx! { r#frame {
        name: "ClassTalentsFrame", width: {BOOK_W * scale}, height: {BOOK_H * scale},
        background_color: "0.0,0.0,0.0,1.0",
        pos_type: "absolute", pos_x: 0.0, pos_y: {BOOK_Y * scale},
        {children}
    } }
}
fn page_contents(view: &TalentView, scale: f32) -> Element {
    let graph = &view.graph;
    let mut children = heading("TalentClassName", &graph.class.name, [372.0, 45.0], scale);
    children.extend(heading(
        "TalentSpecName",
        &graph.spec.name,
        [BOOK_W - 401.0, 45.0],
        scale,
    ));
    children.extend(render_tree(&graph.class, view, None, scale));
    children.extend(render_tree(&graph.spec, view, None, scale));
    for (index, hero) in graph.heroes.iter().enumerate() {
        let y = HERO_TOP + index as f32 * HERO_PREVIEW_SPACING;
        children.extend(heading(
            &format!("TalentHero{}Name", hero.id),
            &hero.name,
            [BOOK_W / 2.0, y - 42.0],
            scale,
        ));
        children.extend(render_tree(hero, view, Some(index), scale));
    }
    children.extend(heading(
        "TalentsReadOnly",
        "Read-only — default grants only; no points spent",
        [BOOK_W / 2.0, BOOK_H - 38.0],
        scale,
    ));
    children
}
fn heading(name: &str, text: &str, center: [f32; 2], scale: f32) -> Element {
    label(
        Label {
            name: name.into(),
            text,
            rect: [center[0] - 240.0, center[1] - 12.0, 480.0, 24.0],
            size: 18.0,
            color: TAB_TEXT_SELECTED,
            justify: "CENTER",
        },
        scale,
    )
}
struct TreeLayout {
    centers: BTreeMap<u32, [f32; 2]>,
    node_scale: f32,
}
fn tree_layout(tree: &TalentTree, hero: Option<usize>) -> TreeLayout {
    let min_y = tree
        .nodes
        .iter()
        .map(|node| node.position[1])
        .fold(f32::INFINITY, f32::min);
    let center_x =
        tree.nodes.iter().map(|node| node.position[0]).sum::<f32>() / tree.nodes.len() as f32;
    let node_scale = if hero.is_some() { HERO_SCALE } else { 1.0 };
    let centers = tree
        .nodes
        .iter()
        .map(|node| {
            let [x, y] = node.position;
            let center = match hero {
                None => [x / 10.0 - PAN[0], y / 10.0 - PAN[1]],
                Some(index) => [
                    BOOK_W / 2.0 + (x - center_x) / 10.0 * HERO_SCALE,
                    HERO_TOP
                        + index as f32 * HERO_PREVIEW_SPACING
                        + ((y - min_y) / 10.0 - 8.0) * HERO_SCALE
                        + BUTTON_SIZE * HERO_SCALE / 2.0,
                ],
            };
            (node.id, center)
        })
        .collect();
    TreeLayout {
        centers,
        node_scale,
    }
}
fn render_tree(tree: &TalentTree, view: &TalentView, hero: Option<usize>, scale: f32) -> Element {
    let layout = tree_layout(tree, hero);
    let mut children: Element = tree
        .edges
        .iter()
        .filter(|edge| edge.visual_style != 0)
        .flat_map(|edge| {
            render_edge(
                edge.id,
                layout.centers[&edge.from],
                layout.centers[&edge.to],
                layout.node_scale,
                scale,
            )
        })
        .collect();
    for node in &tree.nodes {
        children.extend(render_node(
            node,
            view,
            layout.centers[&node.id],
            layout.node_scale,
            scale,
        ));
    }
    children
}
fn render_edge(id: u32, from: [f32; 2], to: [f32; 2], node_scale: f32, scale: f32) -> Element {
    let delta = [to[0] - from[0], to[1] - from[1]];
    let length = (delta[0].hypot(delta[1]) - BUTTON_SIZE * node_scale).max(0.0);
    let rotation = delta[1].atan2(delta[0]);
    let center = [(from[0] + to[0]) / 2.0, (from[1] + to[1]) / 2.0];
    rsx! { texture {
        name: {DynName(format!("TalentEdge{id}"))}, width: {length*scale}, height: {2.0*scale},
        rotation, pos_type: "absolute", pos_x: {(center[0]-length/2.0)*scale}, pos_y: {(center[1]-1.0)*scale},
    } }
}
fn render_node(
    node: &TalentNode,
    view: &TalentView,
    center: [f32; 2],
    node_scale: f32,
    scale: f32,
) -> Element {
    let size = BUTTON_SIZE * node_scale;
    let node_scale = node_scale * scale;
    let mut children = node_entries(node, view, node_scale);
    children.extend(node_border(node, node_scale));
    let max = node.entries[0].max_ranks;
    let ranks = format!("{}/{max}", node.granted_ranks);
    children.extend(label(
        Label {
            name: format!("TalentNode{}Ranks", node.id),
            text: &ranks,
            rect: [15.0, 32.0, 30.0, 14.0],
            size: 12.0,
            color: if node.granted_ranks > 0 {
                TAB_TEXT
            } else {
                TAB_TEXT_SELECTED
            },
            justify: "CENTER",
        },
        node_scale,
    ));
    rsx! { r#frame {
        name: {DynName(format!("TalentNode{}",node.id))}, width: {size*scale}, height: {size*scale},
        pos_type: "absolute", pos_x: {(center[0]-size/2.0)*scale}, pos_y: {(center[1]-size/2.0)*scale},
        {children}
    } }
}
fn node_entries(node: &TalentNode, view: &TalentView, scale: f32) -> Element {
    let choice = node.node_type == 2 && node.flags & 1 != 0;
    let count = if choice { node.entries.len() } else { 1 };
    let mut children = Vec::new();
    for (index, entry) in node.entries.iter().take(count).enumerate() {
        let layout = EntryLayout {
            index,
            count,
            scale,
        };
        children.extend(entry_icon(node, entry, view, layout));
        children.extend(entry_button(node, entry, layout));
    }
    children
}
#[derive(Clone, Copy)]
struct EntryLayout {
    index: usize,
    count: usize,
    scale: f32,
}
fn entry_icon(
    node: &TalentNode,
    entry: &game_engine_core::talent_data::TalentEntry,
    view: &TalentView,
    layout: EntryLayout,
) -> Element {
    use ui_toolkit::widget_def::{Attr, WidgetChild};
    let EntryLayout {
        index,
        count,
        scale,
    } = layout;
    let width = ICON_SIZE / count as f32;
    let left = index as f32 / count as f32;
    let coords = format!("{},{},0.0,1.0", left, left + 1.0 / count as f32);
    let tint = if node.granted_ranks > 0 {
        "1.0,1.0,1.0,1.0"
    } else {
        "0.55,0.55,0.55,1.0"
    };
    let mut icon = rsx! { texture {
        name: {DynName(icon_name(node,entry.id))}, tex_coords: {coords.as_str()},
        vertex_color: tint, width: {width*scale}, height: {ICON_SIZE*scale},
        pos_type: "absolute", pos_x: {(2.0+index as f32*width)*scale}, pos_y: {2.0*scale},
    } };
    let fdid = view.icons[&entry.id];
    if fdid != 0 && !view.missing_icons.contains(&fdid) {
        let WidgetChild::Widget(widget) = &mut icon[0] else {
            unreachable!("rsx texture produces a widget");
        };
        widget
            .attrs
            .push(Attr::new_dynamic("texture_fdid", fdid.to_string()));
    }
    icon
}
fn entry_button(
    node: &TalentNode,
    entry: &game_engine_core::talent_data::TalentEntry,
    layout: EntryLayout,
) -> Element {
    let EntryLayout {
        index,
        count,
        scale,
    } = layout;
    rsx! { button {
        name: {DynName(format!("TalentNode{}Entry{}Spell{}Button",node.id,entry.id,entry.spell_id))},
        width: {BUTTON_SIZE/count as f32*scale}, height: {BUTTON_SIZE*scale}, button_default_skin:false,
        pos_type: "absolute", pos_x: {index as f32*BUTTON_SIZE/count as f32*scale}, pos_y:0.0,
    } }
}
fn node_border(node: &TalentNode, scale: f32) -> Element {
    let choice = node.node_type == 2 && node.flags & 1 != 0;
    let shape = if choice {
        "choice"
    } else if matches!(node.entries[0].entry_type, 2 | 3 | 13) {
        "circle"
    } else {
        "square"
    };
    let color = if node.granted_ranks > 0 {
        "yellow"
    } else {
        "gray"
    };
    let atlas = format!("talents-node-{shape}-{color}");
    // These two skins share the Retail talent mechanic/art; their window chrome differs.
    // Resolve the Retail sheet explicitly, not Forever crops over the same FDID bytes.
    let region =
        resolve_region(&atlas, ActiveSkin::Modern).expect("Retail talent node atlas must exist");
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Talent borders must be DB2-backed");
    };
    let art = AtlasArt {
        fdid,
        atlas: (1.0, 1.0),
        rect: (region.left, region.right, region.top, region.bottom),
    };
    let width = region.width;
    let height = region.height;
    super::art(
        format!("TalentNode{}Border", node.id),
        &art,
        [
            (BUTTON_SIZE - width) / 2.0,
            (BUTTON_SIZE - height) / 2.0,
            width,
            height,
        ],
        scale,
    )
}
fn icon_name(node: &TalentNode, entry: u32) -> String {
    let round = node.node_type != 2 && matches!(node.entries[0].entry_type, 2 | 3 | 13);
    format!(
        "TalentNode{}Entry{entry}{}",
        node.id,
        if round { "RoundIcon" } else { "Icon" }
    )
}
pub(super) fn apply_talents_postsetup(state: &SpellbookFrameState, registry: &mut FrameRegistry) {
    let Some(view) = &state.talents else {
        return;
    };
    for node in view.nodes() {
        for entry in &node.entries {
            let name = icon_name(node, entry.id);
            if let Some(id) = registry.get_by_name(&name)
                && let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
            {
                texture.desaturated = node.granted_ranks == 0;
                let fdid = view.icons[&entry.id];
                if fdid == 0 || view.missing_icons.contains(&fdid) {
                    texture.source = TextureSource::None;
                }
            }
        }
    }
    for tree in [&view.graph.class, &view.graph.spec]
        .into_iter()
        .chain(view.graph.heroes.iter())
    {
        for edge in &tree.edges {
            if let Some(id) = registry.get_by_name(&format!("TalentEdge{}", edge.id))
                && let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
            {
                texture.source = TextureSource::SolidColor(EDGE_COLOR);
            }
        }
    }
}
