//! Blizzard SharedTalentUtil.lua:494-509 and ClassTalentsFrame.xml:37-42.
//! Retail hero selector: one active tree on the page, two columns in its dialog.
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
const FOOTER_HEIGHT: f32 = 82.0;
const MAIN_FIRST_ROW: f32 = 126.0;
const HERO_DIALOG_SIZE: [f32; 2] = [970.0, 832.0];
const HERO_DIALOG_NODES_TOP: f32 = 417.0;
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
    let mut children = heading(
        "TalentClassName",
        &currency_text(view, 4, &graph.class.name),
        [372.0, 45.0],
        scale,
    );
    children.extend(heading(
        "TalentSpecName",
        &currency_text(view, 8, &graph.spec.name),
        [BOOK_W - 401.0, 45.0],
        scale,
    ));
    let zoom = main_tree_zoom(view);
    children.extend(render_tree(
        &graph.class,
        view,
        main_tree_layout(&graph.class, zoom),
        scale,
    ));
    children.extend(render_tree(
        &graph.spec,
        view,
        main_tree_layout(&graph.spec, zoom),
        scale,
    ));
    children.extend(hero_selector(view, scale));
    if let Some(hero) = active_hero(view) {
        let layout = hero_tree_layout(
            hero,
            [BOOK_W / 2.0, HERO_TOP - 8.0 * HERO_SCALE],
            HERO_SCALE,
        );
        children.extend(render_tree(hero, view, layout, scale));
    }
    children.extend(talent_controls(view, scale));
    children.extend(choice_flyout(view, scale));
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
fn main_tree_zoom(view: &TalentView) -> f32 {
    // Reserve Retail's ButtonsParent bottomPadding. Include painted spend text,
    // not only the button hit rect, in the existing uniform zoom transform.
    let bottom = [&view.graph.class, &view.graph.spec]
        .into_iter()
        .map(|tree| {
            let first = first_row_y(tree);
            tree.nodes
                .iter()
                .map(|node| {
                    (node.position[1] - first) / 10.0 + if is_capstone(node) { 42.0 } else { 20.0 }
                })
                .fold(0.0, f32::max)
        })
        .fold(0.0, f32::max);
    ((BOOK_H - FOOTER_HEIGHT - MAIN_FIRST_ROW) / bottom).min(1.0)
}
fn first_row_y(tree: &TalentTree) -> f32 {
    tree.nodes
        .iter()
        .map(|node| node.position[1])
        .fold(f32::INFINITY, f32::min)
}
fn main_tree_layout(tree: &TalentTree, zoom: f32) -> TreeLayout {
    let first = first_row_y(tree);
    let centers = tree
        .nodes
        .iter()
        .map(|node| {
            let x = node.position[0] / 10.0 - PAN[0];
            let y = MAIN_FIRST_ROW + (node.position[1] - first) / 10.0 * zoom;
            (node.id, [BOOK_W / 2.0 + (x - BOOK_W / 2.0) * zoom, y])
        })
        .collect();
    TreeLayout {
        centers,
        node_scale: zoom,
    }
}
fn hero_tree_layout(tree: &TalentTree, origin: [f32; 2], node_scale: f32) -> TreeLayout {
    let min_y = tree
        .nodes
        .iter()
        .map(|node| node.position[1])
        .fold(f32::INFINITY, f32::min);
    let min_x = tree
        .nodes
        .iter()
        .map(|node| node.position[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = tree
        .nodes
        .iter()
        .map(|node| node.position[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let center_x = (min_x + max_x) / 2.0;
    let centers = tree
        .nodes
        .iter()
        .map(|node| {
            let [x, y] = node.position;
            let center = [
                origin[0] + (x - center_x) / 10.0 * node_scale,
                origin[1] + (y - min_y) / 10.0 * node_scale + node_size(node) * node_scale / 2.0,
            ];
            (node.id, center)
        })
        .collect();
    TreeLayout {
        centers,
        node_scale,
    }
}
fn render_tree(tree: &TalentTree, view: &TalentView, layout: TreeLayout, scale: f32) -> Element {
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
    // TextureData angles are counter-clockwise; node positions use y-down.
    let rotation = -delta[1].atan2(delta[0]);
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
    let size = node_size(node) * node_scale;
    let node_scale = node_scale * scale;
    let mut children = node_entries(node, view, node_scale);
    children.extend(node_border(node, view, node_scale));
    let max = if node.node_type == 1 {
        node.entries.iter().map(|entry| entry.max_ranks).sum()
    } else {
        node.entries[0].max_ranks
    };
    let ranks = format!("{}/{max}", view.node_rank(node));
    children.extend(rank_badge(node, view, &ranks, node_scale));
    rsx! { r#frame {
        name: {DynName(format!("TalentNode{}",node.id))}, width: {size*scale}, height: {size*scale},
        pos_type: "absolute", pos_x: {(center[0]-size/2.0)*scale}, pos_y: {(center[1]-size/2.0)*scale},
        {children}
    } }
}
fn rank_badge(node: &TalentNode, view: &TalentView, ranks: &str, scale: f32) -> Element {
    // TalentButtonArt SpendText: bottom anchor, overridden by SizingAdjustment.
    // Thick black glyph outline is Retail's badge, not a new background panel.
    let (x, y, width, height, font_size) = if is_capstone(node) {
        (8.0, 42.0, 52.0, 32.0, 22.0)
    } else {
        (16.0, 10.0, 36.0, 24.0, 16.0)
    };
    let color = if view.node_rank(node) > 0 {
        TAB_TEXT
    } else {
        TAB_TEXT_SELECTED
    };
    rsx! {fontstring {name:{DynName(format!("TalentNode{}Ranks",node.id))},text:ranks,
        width:{width*scale},height:{height*scale},font:GameFont::FrizQuadrata,font_size:{font_size*scale},
        font_color:color,outline:"THICKOUTLINE",shadow_color:"0.0,0.0,0.0,1.0",shadow_offset:"1.0,-1.0",
        justify_h:"CENTER",pos_type:"absolute",pos_x:{x*scale},pos_y:{y*scale},
    }}
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
        children.extend(entry_button(node, entry, view, layout));
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
    let icon_size = if is_capstone(node) { 61.0 } else { ICON_SIZE };
    let inset = (node_size(node) - icon_size) / 2.0;
    let width = icon_size / count as f32;
    let left = index as f32 / count as f32;
    let coords = format!("{},{},0.0,1.0", left, left + 1.0 / count as f32);
    let tint = if view.entry_rank(node, entry.id) > 0 {
        "1.0,1.0,1.0,1.0"
    } else {
        "0.55,0.55,0.55,1.0"
    };
    let mut icon = rsx! { texture {
        name: {DynName(icon_name(node,entry.id))}, tex_coords: {coords.as_str()},
        vertex_color: tint, width: {width*scale}, height: {icon_size*scale},
        pos_type: "absolute", pos_x: {(inset+index as f32*width)*scale}, pos_y: {inset*scale},
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
    view: &TalentView,
    layout: EntryLayout,
) -> Element {
    let EntryLayout {
        index,
        count,
        scale,
    } = layout;
    let action = format!("talent:node:{}", node.id);
    let enabled = view.editor.snapshot.is_some();
    rsx! { button {
        name: {DynName(format!("TalentNode{}Entry{}Spell{}Button",node.id,entry.id,entry.spell_id))},
        onclick: {action.as_str()}, enabled,
        width: {node_size(node)/count as f32*scale}, height: {node_size(node)*scale}, button_default_skin:false,
        pos_type: "absolute", pos_x: {index as f32*node_size(node)/count as f32*scale}, pos_y:0.0,
    } }
}
fn node_border(node: &TalentNode, view: &TalentView, scale: f32) -> Element {
    let choice = node.node_type == 2 && node.flags & 1 != 0;
    let shape = if choice {
        "choice"
    } else if matches!(node.entries[0].entry_type, 2 | 3 | 13) {
        "circle"
    } else {
        "square"
    };
    let color = if view.node_rank(node) > 0 {
        "yellow"
    } else if node.entries.iter().any(|entry| {
        view.editor
            .can_purchase(view, view.level, node.id, entry.id)
    }) {
        "green"
    } else {
        "gray"
    };
    let atlas = if is_capstone(node) {
        let active = if node.entries[0].entry_type == 14 {
            "active-"
        } else {
            ""
        };
        format!("talents-node-apex-{active}large-{color}")
    } else {
        format!("talents-node-{shape}-{color}")
    };
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
    // SizingAdjustment sets StateBorder to the button size; atlas pixel size
    // is not its draw size (apex's 84px source draws inside a 64px button).
    let width = node_size(node);
    let height = node_size(node);
    super::art(
        format!("TalentNode{}Border", node.id),
        &art,
        [
            (node_size(node) - width) / 2.0,
            (node_size(node) - height) / 2.0,
            width,
            height,
        ],
        scale,
    )
}
// TalentButtonCapstoneCircle/SquareTemplate:64px, icon sizing adjustment -3.
fn is_capstone(node: &TalentNode) -> bool {
    matches!(node.entries[0].entry_type, 13 | 14)
}
fn node_size(node: &TalentNode) -> f32 {
    if is_capstone(node) { 64.0 } else { BUTTON_SIZE }
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
                texture.desaturated = view.entry_rank(node, entry.id) == 0;
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

fn talent_controls(view: &TalentView, scale: f32) -> Element {
    use crate::ui::screens::quest_art::panel_button;
    let dirty = view.editor.dirty();
    let footer_top = BOOK_H - FOOTER_HEIGHT;
    let mut children = retail_atlas(
        "TalentFooter",
        "talents-background-bottombar",
        [0.0, footer_top, BOOK_W, FOOTER_HEIGHT],
        scale,
    );
    children.extend(panel_button(
        "TalentApply".into(),
        "",
        "talent:apply",
        dirty,
        (
            (BOOK_W / 2.0 - 82.0) * scale,
            (BOOK_H - FOOTER_HEIGHT / 2.0 - 3.0 - 11.0) * scale,
            164.0 * scale,
            22.0 * scale,
        ),
    ));
    let caption_color = if dirty { TAB_TEXT } else { "0.5,0.5,0.5,1.0" };
    children.extend(rsx! {fontstring {name:"TalentApplyText",text:"Apply Changes",
        width:{164.0*scale},height:{22.0*scale},font:GameFont::FrizQuadrata,font_size:{12.0*scale},
        font_color:caption_color,shadow_color:"0.0,0.0,0.0,1.0",shadow_offset:"1.0,-1.0",
        justify_h:"CENTER",strata:FrameStrata::Dialog,frame_level:102,
        pos_type:"absolute",pos_x:{(BOOK_W/2.0-82.0)*scale},pos_y:{(BOOK_H-FOOTER_HEIGHT/2.0-3.0-11.0)*scale},
    }});
    // ClassTalentsFrame.xml:293-322: Undo and Reset occupy the same anchor.
    let (name, atlas, action) = if dirty {
        ("TalentUndo", "talents-button-undo", "talent:undo")
    } else {
        ("TalentReset", "talents-button-reset", "talent:reset")
    };
    let region = resolve_region(atlas, ActiveSkin::Modern).expect("Retail talent control atlas");
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Talent controls must be DB2-backed");
    };
    let icon = AtlasArt {
        fdid,
        atlas: (1.0, 1.0),
        rect: (region.left, region.right, region.top, region.bottom),
    };
    children.extend(super::art(
        format!("{name}Icon"),
        &icon,
        [
            BOOK_W / 2.0 + 96.0 + (25.0 - region.width) / 2.0,
            BOOK_H - FOOTER_HEIGHT / 2.0 - 3.0 - 12.5 + (25.0 - region.height) / 2.0,
            region.width,
            region.height,
        ],
        scale,
    ));
    let enabled = view.editor.snapshot.is_some();
    children.extend(
        rsx! { button {name:{DynName(name.into())},onclick:action,enabled,
            width:{25.0*scale},height:{25.0*scale},button_default_skin:false,
            pos_type:"absolute",pos_x:{(BOOK_W/2.0+96.0)*scale},pos_y:{(BOOK_H-FOOTER_HEIGHT/2.0-3.0-12.5)*scale},
        }},
    );
    if let Some(reason) = &view.editor.error_text {
        children.extend(label(
            Label {
                name: "TalentError".into(),
                text: reason,
                rect: [200.0, BOOK_H - 76.0, 1212.0, 24.0],
                size: 16.0,
                color: "1.0,0.2,0.2,1.0",
                justify: "CENTER",
            },
            scale,
        ));
    }
    children
}
fn currency_text(view: &TalentView, flags: u32, name: &str) -> String {
    let currency = view.rules.as_ref().and_then(|tree| {
        tree.currencies
            .iter()
            .find(|currency| currency.flags & flags != 0)
    });
    let points = currency.and_then(|currency| {
        view.editor
            .unspent(view)
            .into_iter()
            .find(|&(id, _)| id == currency.id)
            .map(|(_, amount)| amount)
    });
    match points {
        Some(amount) => format!("{name} Points Available: {amount}"),
        None => format!("{name} Points Available: —"),
    }
}
fn choice_flyout(view: &TalentView, scale: f32) -> Element {
    use crate::ui::screens::quest_art::panel_button;
    let Some(node) = view
        .editor
        .choice_node
        .and_then(|id| view.nodes().find(|node| node.id == id))
    else {
        return Vec::new();
    };
    if node.node_type == 3 {
        return hero_choice_dialog(node, view, scale);
    }
    let mut choices = Vec::new();
    for (index, entry) in node.entries.iter().enumerate() {
        let action = format!("talent:choice:{}:{}", node.id, entry.id);
        let name = format!(
            "TalentChoiceNode{}Entry{}Spell{}Button",
            node.id, entry.id, entry.spell_id
        );
        let enabled = view
            .editor
            .can_purchase(view, view.level, node.id, entry.id);
        let text = view.names.get(&entry.id).map(String::as_str).unwrap_or("");
        choices.extend(panel_button(
            name,
            text,
            &action,
            enabled,
            (
                20.0 * scale,
                (20.0 + index as f32 * 42.0) * scale,
                280.0 * scale,
                32.0 * scale,
            ),
        ));
        let fdid = view.icons[&entry.id];
        if fdid != 0 && !view.missing_icons.contains(&fdid) {
            choices.extend(file_texture(
                format!("TalentChoiceEntry{}Icon", entry.id),
                fdid,
                [24.0, 23.0 + index as f32 * 42.0, 26.0, 26.0],
                scale,
            ));
        }
    }
    let height = 62.0 + node.entries.len() as f32 * 42.0;
    choices.extend(panel_button(
        "TalentCloseChoice".into(),
        "Close",
        "talent:close_choice",
        true,
        (
            110.0 * scale,
            (height - 32.0) * scale,
            100.0 * scale,
            22.0 * scale,
        ),
    ));
    rsx! {r#frame {name:"TalentChoiceFlyout",width:{320.0*scale},height:{height*scale},
        background_color:"0.08,0.06,0.04,1.0",strata:FrameStrata::Dialog,pos_type:"absolute",pos_x:{(BOOK_W/2.0-160.0)*scale},pos_y:{100.0*scale},frame_level:200,
        {choices}
    }}
}

fn active_hero(view: &TalentView) -> Option<&TalentTree> {
    let selector = view.graph.hero_selection.as_ref()?;
    let entry = selector
        .entries
        .iter()
        .find(|entry| view.editor.rank(selector.id, entry.id) > 0)?;
    view.graph
        .heroes
        .iter()
        .find(|tree| tree.id == entry.subtree_id)
}
fn retail_atlas(name: &str, atlas: &str, rect: [f32; 4], scale: f32) -> Element {
    let region = resolve_region(atlas, ActiveSkin::Modern).expect("Retail talent atlas");
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Retail talent FDID");
    };
    let art = AtlasArt {
        fdid,
        atlas: (1.0, 1.0),
        rect: (region.left, region.right, region.top, region.bottom),
    };
    super::art(name.into(), &art, rect, scale)
}
fn hero_icon(tree: &TalentTree, name: &str, rect: [f32; 4], scale: f32) -> Element {
    let atlas = ui_toolkit::atlas::get_name_by_element_id(tree.icon_atlas_element_id)
        .expect("TraitSubTree authored atlas element");
    retail_atlas(name, atlas, rect, scale)
}
fn hero_selector(view: &TalentView, scale: f32) -> Element {
    let Some(selector) = &view.graph.hero_selection else {
        return Vec::new();
    };
    let active = active_hero(view);
    let mut children = Vec::new();
    let button_x = BOOK_W / 2.0 - 54.0;
    if let Some(tree) = active {
        children.extend(hero_icon(
            tree,
            "HeroSpecRoundIcon",
            [button_x, 102.0, 108.0, 108.0],
            scale,
        ));
    } else {
        children.extend(hero_split_icons(view, scale));
    }
    children.extend(retail_atlas(
        "HeroSpecBorder",
        "talents-heroclass-ring-mainpane",
        [BOOK_W / 2.0 - 96.0, 62.0, 192.0, 192.0],
        scale,
    ));
    let action = format!("talent:node:{}", selector.id);
    let enabled = view.editor.snapshot.is_some();
    children.extend(
        rsx! {button { name:"HeroSpecButton",onclick:{action.as_str()},enabled,
            width:{108.0*scale},height:{108.0*scale},button_default_skin:false,
            pos_type:"absolute",pos_x:{button_x*scale},pos_y:{102.0*scale},
        }},
    );
    let text = match active {
        Some(tree) => tree.name.clone(),
        None => selector
            .entries
            .iter()
            .map(|entry| view.names[&entry.id].as_str())
            .collect::<Vec<_>>()
            .join(" / "),
    };
    children.extend(heading(
        "TalentHeroSelectionLabel",
        &text,
        [BOOK_W / 2.0, 238.0],
        scale,
    ));
    children
}
fn hero_split_icons(view: &TalentView, scale: f32) -> Element {
    let selector = view.graph.hero_selection.as_ref().expect("hero selector");
    selector
        .entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            let tree = view
                .graph
                .heroes
                .iter()
                .find(|tree| tree.id == entry.subtree_id)
                .expect("eligible hero tree");
            let atlas = ui_toolkit::atlas::get_name_by_element_id(tree.icon_atlas_element_id)
                .expect("hero atlas");
            let region = resolve_region(atlas, ActiveSkin::Modern).expect("hero atlas region");
            let AtlasSource::FileDataId(fdid) = region.source else {
                panic!("hero FDID");
            };
            let mid = (region.left + region.right) / 2.0;
            let crop = if index == 0 {
                (region.left, mid)
            } else {
                (mid, region.right)
            };
            let art = AtlasArt {
                fdid,
                atlas: (1.0, 1.0),
                rect: (crop.0, crop.1, region.top, region.bottom),
            };
            super::art(
                format!("HeroSpecOption{}Icon", tree.id),
                &art,
                [
                    BOOK_W / 2.0 - 54.0 + index as f32 * 54.0,
                    102.0,
                    54.0,
                    108.0,
                ],
                scale,
            )
        })
        .collect()
}
fn hero_choice_dialog(node: &TalentNode, view: &TalentView, scale: f32) -> Element {
    use crate::ui::screens::quest_art::panel_button;
    let [width, height] = HERO_DIALOG_SIZE;
    let column_width = width / node.entries.len() as f32;
    let mut children = retail_atlas(
        "HeroChoiceBackground",
        "talents-heroclass-choicepopup-background",
        [0.0, 0.0, width, height],
        scale,
    );
    for (index, entry) in node.entries.iter().enumerate() {
        let tree = view
            .graph
            .heroes
            .iter()
            .find(|tree| tree.id == entry.subtree_id)
            .expect("eligible hero tree");
        children.extend(hero_choice_column(
            node,
            entry,
            tree,
            view,
            [index as f32 * column_width, column_width],
            scale,
        ));
    }
    children.extend(panel_button(
        "TalentCloseChoice".into(),
        "Close",
        "talent:close_choice",
        true,
        (
            (width - 90.0) * scale,
            8.0 * scale,
            80.0 * scale,
            22.0 * scale,
        ),
    ));
    rsx! {r#frame {name:"TalentChoiceFlyout",width:{width*scale},height:{height*scale},
        background_color:"0.08,0.06,0.04,1.0",strata:FrameStrata::Dialog,pos_type:"absolute",pos_x:{(BOOK_W/2.0-width/2.0)*scale},pos_y:{70.0*scale},frame_level:200,
        {children}
    }}
}
fn hero_choice_column(
    node: &TalentNode,
    entry: &game_engine_core::talent_data::TalentEntry,
    tree: &TalentTree,
    view: &TalentView,
    column: [f32; 2],
    scale: f32,
) -> Element {
    use crate::ui::screens::quest_art::panel_button;
    let [x, width] = column;
    let height = HERO_DIALOG_SIZE[1];
    let mut children = heading(
        &format!("TalentHero{}Name", tree.id),
        &tree.name,
        [width / 2.0, 51.0],
        scale,
    );
    children.extend(hero_icon(
        tree,
        &format!("TalentHero{}RoundIcon", tree.id),
        [width / 2.0 - 81.0, 91.0, 162.0, 162.0],
        scale,
    ));
    children.extend(retail_atlas(
        &format!("TalentHero{}Ring", tree.id),
        "talents-heroclass-ring-selectionpane-gray",
        [width / 2.0 - 124.0, 50.0, 248.0, 248.0],
        scale,
    ));
    children.extend(label(
        Label {
            name: format!("TalentHero{}Description", tree.id),
            text: &tree.description,
            rect: [40.0, 284.0, width - 80.0, 80.0],
            size: 16.0,
            color: TAB_TEXT,
            justify: "CENTER",
        },
        scale,
    ));
    let layout = hero_tree_layout(tree, [width / 2.0, HERO_DIALOG_NODES_TOP], 1.0);
    children.extend(render_tree(tree, view, layout, scale));
    let action = format!("talent:choice:{}:{}", node.id, entry.id);
    let enabled = view
        .editor
        .can_purchase(view, view.level, node.id, entry.id);
    children.extend(panel_button(
        format!(
            "TalentChoiceNode{}Entry{}Spell{}Button",
            node.id, entry.id, entry.spell_id
        ),
        "Activate",
        &action,
        enabled,
        (
            (width / 2.0 - 80.0) * scale,
            (height - 70.0) * scale,
            160.0 * scale,
            22.0 * scale,
        ),
    ));
    rsx! {r#frame {name:{DynName(format!("TalentHero{}Option",tree.id))},width:{width*scale},height:{height*scale},
        pos_type:"absolute",pos_x:{x*scale},pos_y:0.0,{children}
    }}
}
