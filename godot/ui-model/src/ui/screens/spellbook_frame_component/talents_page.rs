//! Blizzard SharedTalentUtil.lua:494-509 and ClassTalentsFrame.xml:37-42.
//! Retail hero selector: one active tree on the page, two columns in its dialog.
use super::*;
use crate::talents::{TalentView, TalentVisualState};
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
const HERO_DIALOG_SIZE: [f32; 2] = [970.0, 832.0];
const HERO_DIALOG_NODES_TOP: f32 = 417.0;
#[path = "talents_footer.rs"]
mod footer;
#[path = "pvp_talent_controls.rs"]
mod pvp_talent_controls;

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
        // PlayerSpellsFrame.xml:43-45:1612px page BOTTOM in1618px parent.
        pos_type: "absolute", pos_x: {(FRAME_W - BOOK_W) / 2.0 * scale}, pos_y: {BOOK_Y * scale},
        {children}
    } }
}
fn page_contents(view: &TalentView, scale: f32) -> Element {
    let graph = &view.graph;
    let slug = |name: &str| name.to_lowercase().replace(' ', "");
    let atlas = format!(
        "talents-background-{}-{}",
        slug(&graph.class.name),
        slug(&graph.spec.name)
    );
    let mut children = retail_atlas(
        "TalentSpecBackground",
        &atlas,
        [0.0, 0.0, BOOK_W, BOOK_H - FOOTER_HEIGHT],
        scale,
    );
    children.extend(currency_header(
        view,
        4,
        "TalentClass",
        &graph.class.name,
        [372.0, 45.0],
        scale,
    ));
    children.extend(currency_header(
        view,
        8,
        "TalentSpec",
        &graph.spec.name,
        [BOOK_W - 401.0, 45.0],
        scale,
    ));
    let pan = class_pan_offset(&graph.class.name);
    children.extend(render_tree(
        &graph.class,
        view,
        main_tree_layout(&graph.class, pan),
        scale,
    ));
    children.extend(render_tree(
        &graph.spec,
        view,
        main_tree_layout(&graph.spec, pan),
        scale,
    ));
    children.extend(hero_selector(view, scale));
    if let Some(hero) = active_hero(view) {
        let layout = hero_tree_layout(
            hero,
            [hero_center_x(view), HERO_TOP - 8.0 * HERO_SCALE],
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
fn class_pan_offset(class: &str) -> [f32; 2] {
    // ClassTalentUtil.lua ClassVisuals and ClassTalentsFrame.lua UpdateClassVisuals:
    // basePanOffset = initialBasePanOffset - classVisuals.panOffset.
    let [x, y] = match class {
        "Warrior" => [60.0, 31.0],
        "Paladin" => [-60.0, -29.0],
        "Hunter" | "Monk" => [0.0, -29.0],
        "Rogue" | "Mage" | "Druid" | "Demon Hunter" | "Evoker" => [30.0, -29.0],
        "Priest" => [-30.0, -29.0],
        "Death Knight" | "Shaman" | "Warlock" => [0.0, 1.0],
        _ => panic!("No Retail class pan offset for {class}"),
    };
    [PAN[0] - x, PAN[1] - y]
}
fn main_tree_layout(tree: &TalentTree, pan: [f32; 2]) -> TreeLayout {
    // SharedTalentUtil.TranslateNodePositionsToAnchorPositions; CENTER TOPLEFT.
    // ButtonsParent has zero top/left padding; shared layout defaults zoom to1.
    let centers = tree
        .nodes
        .iter()
        .map(|node| {
            let x = node.position[0] / 10.0 - pan[0];
            let y = node.position[1] / 10.0 - pan[1];
            (node.id, [x, y])
        })
        .collect();
    TreeLayout {
        centers,
        node_scale: 1.0,
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
        .flat_map(|edge| render_edge(edge, tree, view, &layout, scale))
        .collect();
    for gate in view.gates_for_nodes(&tree.nodes) {
        let center = layout.centers[&gate.node_id];
        let node = tree
            .nodes
            .iter()
            .find(|node| node.id == gate.node_id)
            .expect("gate anchor node");
        children.extend(render_gate(
            &gate,
            center,
            node_size(node) * layout.node_scale,
            scale,
        ));
    }
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
fn render_gate(
    gate: &crate::talents::TalentGate,
    center: [f32; 2],
    node_width: f32,
    scale: f32,
) -> Element {
    // SharedTalentFrameTemplates.xml:124x40, RIGHT to node LEFT -12.
    let text = gate.remaining.to_string();
    let (width, _) = measure_text(&text, GameFont::FrizQuadrata, 24.0).expect("gate amount font");
    let mut children = retail_atlas(
        &format!("TalentGate{}Icon", gate.condition_id),
        "talents-gate",
        [65.0, 11.0, 84.0, 14.0],
        scale,
    );
    children.extend(label(
        Label {
            name: format!("TalentGate{}Text", gate.condition_id),
            text: &text,
            rect: [55.0 - width, 6.0, width, 30.0],
            size: 24.0,
            color: "1.0,0.64,0.56,1.0",
            justify: "RIGHT",
        },
        scale,
    ));
    rsx! { button { name:{DynName(format!("TalentGate{}Button",gate.condition_id))},
        width:{124.0*scale},height:{40.0*scale},mouse_enabled:true,button_default_skin:false,
        pos_type:"absolute",pos_x:{(center[0]-node_width/2.0-12.0-124.0)*scale},pos_y:{(center[1]-20.0)*scale},
        {children}
    }}
}
fn render_edge(
    edge: &game_engine_core::talent_data::TalentEdge,
    tree: &TalentTree,
    view: &TalentView,
    layout: &TreeLayout,
    scale: f32,
) -> Element {
    let from = layout.centers[&edge.from];
    let to = layout.centers[&edge.to];
    let parent = tree
        .nodes
        .iter()
        .find(|node| node.id == edge.from)
        .expect("edge parent");
    let child = tree
        .nodes
        .iter()
        .find(|node| node.id == edge.to)
        .expect("edge child");
    let color = if view.visual_state(child) == TalentVisualState::Gated {
        "locked"
    } else if view.visual_state(parent) == TalentVisualState::Maxed {
        "yellow"
    } else {
        "gray"
    };
    let delta = [to[0] - from[0], to[1] - from[1]];
    let distance = delta[0].hypot(delta[1]);
    let inset = node_size(child) * layout.node_scale / 2.0;
    let end = [
        to[0] - delta[0] / distance * inset,
        to[1] - delta[1] / distance * inset,
    ];
    let length = (distance - inset).max(0.0);
    let rotation = -delta[1].atan2(delta[0]);
    let center = [(from[0] + end[0]) / 2.0, (from[1] + end[1]) / 2.0];
    let mut children = rotated_atlas(
        &format!("TalentEdge{}", edge.id),
        &format!("talents-arrow-line-{color}"),
        [center[0] - length / 2.0, center[1] - 3.0, length, 6.0],
        rotation,
        scale,
    );
    let head_atlas = format!("talents-arrow-head-{color}");
    let head = resolve_region(&head_atlas, ActiveSkin::Modern).expect("authored arrow head");
    children.extend(rotated_atlas(
        &format!("TalentEdge{}Head", edge.id),
        &head_atlas,
        [
            end[0] - head.width / 2.0,
            end[1] - head.height / 2.0,
            head.width,
            head.height,
        ],
        rotation + std::f32::consts::FRAC_PI_2,
        scale,
    ));
    children
}
fn rotated_atlas(name: &str, atlas: &str, rect: [f32; 4], rotation: f32, scale: f32) -> Element {
    use ui_toolkit::widget_def::{Attr, WidgetChild};
    let mut art = retail_atlas(name, atlas, rect, scale);
    let WidgetChild::Widget(widget) = &mut art[0] else {
        unreachable!("atlas texture")
    };
    widget
        .attrs
        .push(Attr::new_dynamic("rotation", rotation.to_string()));
    art
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
    if let Some(atlas) = view.search_match_atlas(node) {
        children.extend(retail_atlas(
            &format!("TalentNode{}SearchMatch", node.id),
            atlas,
            [node_size(node) - 31.5, -31.5, 63.0, 63.0],
            node_scale,
        ));
    }
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
    // TalentButtonArt SpendText: BOTTOM11,4; Capstone sizing overrides to2,-10.
    // Bottom-align the glyphs too: a centred Label does not paint at that anchor.
    // Fonts.xml Roman SystemFont22_Shadow_ThickOutline is Arial Narrow18.
    let (x, y, width, height, font_size) = if is_capstone(node) {
        (8.0, 42.0, 52.0, 32.0, 18.0)
    } else {
        (13.0, 12.0, 36.0, 24.0, 16.0)
    };
    let color = match view.visual_state(node) {
        TalentVisualState::Selectable => "0.0,1.0,0.0,1.0",
        TalentVisualState::Maxed | TalentVisualState::Purchased => "1.0,0.82,0.0,1.0",
        TalentVisualState::Gated | TalentVisualState::Locked => "0.5,0.5,0.5,1.0",
    };
    let mut children = if ui_toolkit::atlas::thread_skin() == ActiveSkin::Forever {
        let rect = if is_capstone(node) {
            [22.0, 54.0, 24.0, 20.0]
        } else {
            [19.0, 16.0, 24.0, 20.0]
        };
        retail_atlas(
            &format!("TalentNode{}SpendArt", node.id),
            "talents-main-ring-box-c60",
            rect,
            scale,
        )
    } else {
        Vec::new()
    };
    children.extend(rsx! {fontstring {name:{DynName(format!("TalentNode{}Ranks",node.id))},text:ranks,
        width:{width*scale},height:{height*scale},font:GameFont::ArialNarrow,font_size:{font_size*scale},
        font_color:color,outline:"THICKOUTLINE",shadow_color:"0.0,0.0,0.0,1.0",shadow_offset:"1.0,-1.0",
        justify_h:"CENTER",pos_type:"absolute",pos_x:{x*scale},pos_y:{y*scale},
    }});
    children
}
fn node_entries(node: &TalentNode, view: &TalentView, scale: f32) -> Element {
    let choice = node.node_type == 2 && node.flags & 1 != 0;
    let selected = choice
        .then(|| {
            node.entries
                .iter()
                .find(|entry| view.entry_rank(node, entry.id) > 0)
        })
        .flatten();
    let entries: Vec<_> = match selected {
        Some(entry) => vec![entry],
        None if choice => node.entries.iter().collect(),
        None => vec![&node.entries[0]],
    };
    let count = entries.len();
    let mut children = Vec::new();
    // Icon2 is the full octagon beneath Icon's authored left-half mask.
    for (index, entry) in entries.into_iter().enumerate().rev() {
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
    let scale = layout.scale;
    let icon_size = if is_capstone(node) { 61.0 } else { ICON_SIZE };
    let inset = (node_size(node) - icon_size) / 2.0;
    // Retail overlays two full36px icons with complementary authored masks,
    // rather than squeezing/cropping each spell into an18px rectangular half.
    let width = icon_size;
    let tint = if matches!(
        view.visual_state(node),
        TalentVisualState::Locked | TalentVisualState::Gated
    ) {
        "0.75,0.75,0.75,1.0"
    } else {
        "1.0,1.0,1.0,1.0"
    };
    let mut icon = rsx! { texture {
        name: {DynName(icon_name(node,entry.id,view))},
        vertex_color: tint, width: {width*scale}, height: {icon_size*scale},
        pos_type: "absolute", pos_x: {inset*scale}, pos_y: {inset*scale},
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
    let atlas = node_border_atlas(node, view);
    let hover_alpha = match view.visual_state(node) {
        TalentVisualState::Selectable => 1.0,
        TalentVisualState::Maxed | TalentVisualState::Purchased => 0.5,
        TalentVisualState::Gated | TalentVisualState::Locked => 0.25,
    };
    rsx! { button {
        name: {DynName(format!("TalentNode{}Entry{}Spell{}Button",node.id,entry.id,entry.spell_id))},
        onclick: {action.as_str()}, enabled,button_atlas_highlight:{atlas.as_str()},button_highlight_alpha:hover_alpha,
        button_atlas_pressed:{atlas.as_str()},
        width: {node_size(node)/count as f32*scale}, height: {node_size(node)*scale}, button_default_skin:false,
        pos_type: "absolute", pos_x: {index as f32*node_size(node)/count as f32*scale}, pos_y:0.0,
    } }
}
fn node_border_atlas(node: &TalentNode, view: &TalentView) -> String {
    let choice = node.node_type == 2 && node.flags & 1 != 0;
    let shape = if choice {
        "choice"
    } else if matches!(node.entries[0].entry_type, 2 | 3 | 13) {
        "circle"
    } else {
        "square"
    };
    let color = match view.visual_state(node) {
        TalentVisualState::Maxed | TalentVisualState::Purchased => "yellow",
        TalentVisualState::Selectable => "green",
        TalentVisualState::Gated => "locked",
        TalentVisualState::Locked => "gray",
    };
    if is_capstone(node) {
        let active = if node.entries[0].entry_type == 14 {
            "active-"
        } else {
            ""
        };
        format!("talents-node-apex-{active}large-{color}")
    } else {
        format!("talents-node-{shape}-{color}")
    }
}
fn node_border(node: &TalentNode, view: &TalentView, scale: f32) -> Element {
    let atlas = node_border_atlas(node, view);
    let retail =
        resolve_region(&atlas, ActiveSkin::Modern).expect("Retail talent node atlas must exist");
    let region = resolve_region(&atlas, ui_toolkit::atlas::thread_skin()).expect("talent art skin");
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Talent borders must be DB2-backed");
    };
    let art = AtlasArt {
        fdid,
        atlas: (1.0, 1.0),
        rect: (region.left, region.right, region.top, region.bottom),
    };
    // UpdateStateBorder uses atlas draw dimensions, including override sizes.
    // Ordinary borders override to40; a capstone keeps its padded84/80px art
    // around the64px hit target. Resampling that art to64 shrinks its rim.
    let width = retail.width;
    let height = retail.height;
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
fn icon_name(node: &TalentNode, entry: u32, view: &TalentView) -> String {
    let suffix = if node.node_type == 2 && node.flags & 1 != 0 {
        if view.node_rank(node) > 0 {
            "ChoiceIcon"
        } else if node.entries[0].id == entry {
            "ChoiceLeftIcon"
        } else {
            "ChoiceRightIcon"
        }
    } else if matches!(node.entries[0].entry_type, 2 | 3 | 13) {
        "RoundIcon"
    } else {
        "Icon"
    };
    format!("TalentNode{}Entry{entry}{suffix}", node.id)
}
pub(super) fn apply_talents_postsetup(state: &SpellbookFrameState, registry: &mut FrameRegistry) {
    let Some(view) = &state.talents else {
        return;
    };
    for node in view.nodes() {
        // RSX has no justify_v attribute; set the actual FontString property.
        let rank_name = format!("TalentNode{}Ranks", node.id);
        if let Some(id) = registry.get_by_name(&rank_name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::FontString(text)) = frame.widget_data.as_mut()
        {
            text.justify_v = ui_toolkit::widgets::font_string::JustifyV::Bottom;
        }
        for entry in &node.entries {
            let name = icon_name(node, entry.id, view);
            if let Some(id) = registry.get_by_name(&name)
                && let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
            {
                texture.desaturated = matches!(
                    view.visual_state(node),
                    TalentVisualState::Locked | TalentVisualState::Gated
                );
                let fdid = view.icons[&entry.id];
                if fdid == 0 || view.missing_icons.contains(&fdid) {
                    texture.source = TextureSource::None;
                }
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
    let enabled = view.editor.snapshot.is_some()
        && (dirty
            || view
                .nodes()
                .any(|node| view.node_rank(node) > node.granted_ranks));
    children.extend(
        rsx! { button {name:{DynName(name.into())},onclick:action,enabled,
            width:{25.0*scale},height:{25.0*scale},button_default_skin:false,
            pos_type:"absolute",pos_x:{(BOOK_W/2.0+96.0)*scale},pos_y:{(BOOK_H-FOOTER_HEIGHT/2.0-3.0-12.5)*scale},
        }},
    );
    children.extend(footer::controls(view, scale));
    children.extend(war_mode_control(view, scale));
    children.extend(pvp_talent_controls::render(view, scale));
    if let Some(reason) = view.editor.error_text.as_ref().or(view.pvp_error.as_ref()) {
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
fn currency_header(
    view: &TalentView,
    flags: u32,
    prefix: &str,
    name: &str,
    center: [f32; 2],
    scale: f32,
) -> Element {
    let points = currency_amount(view, flags);
    let amount_color = if points != "0" && points != "—" {
        "0.0,1.0,0.0,1.0"
    } else {
        "0.5,0.5,0.5,1.0"
    };
    let (width, _) =
        measure_text(name, GameFont::FrizQuadrata, 18.0).expect("currency header font");
    let left = center[0] - (width + 3.0 + 54.0) / 2.0;
    let mut children = label(
        Label {
            name: format!("{prefix}Name"),
            text: name,
            rect: [left, center[1] - 12.0, width, 24.0],
            size: 18.0,
            color: TAB_TEXT_SELECTED,
            justify: "CENTER",
        },
        scale,
    );
    children.extend(label(
        Label {
            name: format!("{prefix}Points"),
            text: &points,
            rect: [left + width + 3.0, center[1] - 20.0, 54.0, 40.0],
            size: 32.0,
            color: amount_color,
            justify: "LEFT",
        },
        scale,
    ));
    children
}
fn currency_amount(view: &TalentView, flags: u32) -> String {
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
        Some(amount) => amount.to_string(),
        None => "—".to_string(),
    }
}
fn choice_flyout(view: &TalentView, scale: f32) -> Element {
    let Some(node) = view
        .editor
        .choice_node
        .and_then(|id| view.nodes().find(|node| node.id == id))
    else {
        return Vec::new();
    };
    if node.node_type == 3 {
        return Vec::new(); // UIParent-owned modal, rendered by dialogs().
    }
    let width = node.entries.len() as f32 * 64.0 + (node.entries.len() - 1) as f32 * 5.0;
    let (center, node_scale) = choice_node_center(view, node);
    let choices: Element = node
        .entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| choice_option(node, entry, view, index as f32 * 69.0, scale))
        .collect();
    // SharedTalentFrame ShowSelections: BOTTOM to node TOP with y-up -10.
    rsx! {r#frame {name:"TalentChoiceFlyout",width:{width*scale},height:{64.0*scale},
        strata:FrameStrata::Dialog,pos_type:"absolute",pos_x:{(center[0]-width/2.0)*scale},
        pos_y:{(center[1]-node_size(node)*node_scale/2.0-64.0+10.0)*scale},frame_level:200,
        {choices}
    }}
}
fn choice_node_center(view: &TalentView, node: &TalentNode) -> ([f32; 2], f32) {
    for tree in [&view.graph.class, &view.graph.spec] {
        if tree.nodes.iter().any(|candidate| candidate.id == node.id) {
            let pan = class_pan_offset(&view.graph.class.name);
            return (main_tree_layout(tree, pan).centers[&node.id], 1.0);
        }
    }
    let hero = active_hero(view).expect("choice belongs to active subtree");
    (
        hero_tree_layout(
            hero,
            [hero_center_x(view), HERO_TOP - 8.0 * HERO_SCALE],
            HERO_SCALE,
        )
        .centers[&node.id],
        HERO_SCALE,
    )
}
fn choice_option(
    node: &TalentNode,
    entry: &game_engine_core::talent_data::TalentEntry,
    view: &TalentView,
    x: f32,
    scale: f32,
) -> Element {
    let fdid = view.icons[&entry.id];
    let enabled = view
        .editor
        .can_purchase(view, view.level, node.id, entry.id);
    let selected = view.entry_rank(node, entry.id) > 0;
    let shape = if matches!(entry.entry_type, 2 | 3 | 13) {
        "circle"
    } else {
        "square"
    };
    let color = if selected {
        "yellow"
    } else if enabled {
        "green"
    } else {
        "gray"
    };
    let atlas = format!("talents-node-choiceflyout-{shape}-{color}");
    let mut children = file_texture(
        format!("TalentChoiceEntry{}Icon", entry.id),
        fdid,
        [x, 0.0, 64.0, 64.0],
        scale,
    );
    children.extend(retail_atlas(
        &format!("TalentChoiceEntry{}Border", entry.id),
        &atlas,
        [x - 8.0, -8.0, 80.0, 80.0],
        scale,
    ));
    let action = format!("talent:choice:{}:{}", node.id, entry.id);
    children.extend(rsx! {button {
        name:{DynName(format!("TalentChoiceNode{}Entry{}Spell{}Button",node.id,entry.id,entry.spell_id))},
        onclick:{action.as_str()},enabled,button_default_skin:false,button_atlas_highlight:{atlas.as_str()},button_highlight_alpha:0.25,
        width:{64.0*scale},height:{64.0*scale},pos_type:"absolute",pos_x:{x*scale},pos_y:0.0,
    }});
    children
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
    let region =
        resolve_region(atlas, ui_toolkit::atlas::thread_skin()).expect("talent atlas art skin");
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
fn hero_center_x(view: &TalentView) -> f32 {
    // ClassTalentUtil.SpecializationVisuals: authored heroContainerOffset.
    let offset = match (
        view.graph.class.name.as_str(),
        view.graph.spec.name.as_str(),
    ) {
        ("Demon Hunter", "Devourer") => 0.0,
        ("Demon Hunter", "Vengeance") | ("Paladin", "Holy" | "Retribution") | ("Rogue", _) => -45.0,
        ("Druid", "Restoration") => -15.0,
        ("Druid" | "Priest" | "Shaman", _) => 15.0,
        (
            "Death Knight" | "Demon Hunter" | "Evoker" | "Hunter" | "Mage" | "Monk" | "Paladin"
            | "Warlock" | "Warrior",
            _,
        ) => -15.0,
        (class, spec) => panic!("No Retail hero container position for {class}/{spec}"),
    };
    BOOK_W / 2.0 + offset
}
fn hero_selector(view: &TalentView, scale: f32) -> Element {
    let Some(selector) = &view.graph.hero_selection else {
        return Vec::new();
    };
    let active = active_hero(view);
    let mut children = Vec::new();
    let center_x = hero_center_x(view);
    let button_x = center_x - 54.0;
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
    let ring_atlas = if ui_toolkit::atlas::thread_skin() == ActiveSkin::Forever {
        "talents-main-ring-c60"
    } else if active.is_none() {
        "talents-heroclass-ring-intro"
    } else {
        "talents-heroclass-ring-mainpane"
    };
    children.extend(retail_atlas(
        "HeroSpecBorder",
        ring_atlas,
        [center_x - 96.0, 62.0, 192.0, 192.0],
        scale,
    ));
    let action = format!("talent:node:{}", selector.id);
    let enabled = view.editor.snapshot.is_some();
    children.extend(
        rsx! {button { name:"HeroSpecButton",onclick:{action.as_str()},enabled,
            button_atlas_highlight:ring_atlas,button_highlight_size:"192,192",button_highlight_alpha:0.4,
            width:{108.0*scale},height:{108.0*scale},button_default_skin:false,
            pos_type:"absolute",pos_x:{button_x*scale},pos_y:{102.0*scale},
        }},
    );
    if let Some(atlas) = view.inactive_hero_search_atlas() {
        children.extend(retail_atlas(
            "HeroSpecSearchMatch",
            atlas,
            [button_x + 108.0 - 31.5, 102.0 - 31.5, 63.0, 63.0],
            scale,
        ));
    }
    if let Some(tree) = active {
        children.extend(heading(
            "TalentHeroSelectionLabel",
            &tree.name,
            [center_x, 67.0],
            scale,
        ));
    } else {
        children.extend(label(
            Label {
                name: "TalentHeroChooseLabel".into(),
                text: "CHOOSE YOUR",
                rect: [center_x - 160.0, 43.0, 320.0, 15.0],
                size: 15.0,
                color: "0.0,1.0,0.0,1.0",
                justify: "CENTER",
            },
            scale,
        ));
        children.extend(label(
            Label {
                name: "TalentHeroSelectionLabel".into(),
                text: "HERO TALENTS",
                rect: [center_x - 180.0, 60.0, 360.0, 24.0],
                size: 24.0,
                color: "0.0,1.0,0.0,1.0",
                justify: "CENTER",
            },
            scale,
        ));
    }
    let points = view.hero_points();
    if points > 0 {
        children.extend(retail_atlas(
            "TalentHeroPointsArt",
            "talents-heroclass-ring-pointsavailable",
            [center_x - 28.0, 185.0, 56.0, 56.0],
            scale,
        ));
        let text = points.to_string();
        let (width, _) =
            measure_text(&text, GameFont::FrizQuadrata, 24.0).expect("hero currency font");
        let x = center_x - width / 2.0 - if text.starts_with('1') { 2.0 } else { 0.0 };
        children.extend(label(
            Label {
                name: "TalentHeroPoints".into(),
                text: &text,
                rect: [x, 197.0, width, 30.0],
                size: 24.0,
                color: "0.0,1.0,0.0,1.0",
                justify: "CENTER",
            },
            scale,
        ));
    }
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
                    hero_center_x(view) - 54.0 + index as f32 * 54.0,
                    102.0,
                    54.0,
                    108.0,
                ],
                scale,
            )
        })
        .collect()
}
pub(super) fn dialogs(state: &SpellbookFrameState, scale: f32) -> Element {
    if state.tab != PlayerSpellsTab::Talents {
        return Vec::new();
    }
    let Some(view) = &state.talents else {
        return Vec::new();
    };
    let mut dialogs = footer::loadout_dialog(view, state.viewport);
    let Some(node) = view.editor.choice_node.and_then(|id| {
        view.nodes()
            .find(|node| node.id == id && node.node_type == 3)
    }) else {
        return dialogs;
    };
    let [width, height] = state.viewport;
    let mut children = rsx! { button {
        name:"TalentHeroDialogCover",width,height,mouse_enabled:true,button_default_skin:false,onclick:"talent:close_choice",
        background_color:"0.0,0.0,0.0,0.5",strata:FrameStrata::Dialog,frame_level:2000,
        pos_type:"absolute",pos_x:0.0,pos_y:0.0,
    }};
    children.extend(hero_choice_dialog(node, view, scale, state.viewport));
    dialogs.extend(children);
    dialogs
}
fn hero_choice_dialog(
    node: &TalentNode,
    view: &TalentView,
    scale: f32,
    viewport: [f32; 2],
) -> Element {
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
    children.extend(rsx! { button { name:"TalentCloseChoice",onclick:"talent:close_choice",
        width:{24.0*scale},height:{24.0*scale},button_default_skin:false,
        button_atlas_up:"RedButton-Exit",button_atlas_pressed:"RedButton-Exit-Pressed",button_atlas_highlight:"RedButton-Highlight",
        pos_type:"absolute",pos_x:{(width-23.0)*scale},pos_y:0.0,frame_level:2010,
    }});
    rsx! {r#frame {name:"TalentChoiceFlyout",width:{width*scale},height:{height*scale},
        background_color:"0.08,0.06,0.04,1.0",strata:FrameStrata::Dialog,pos_type:"absolute",pos_x:{(viewport[0]-width*scale)/2.0},pos_y:{70.0*scale},frame_level:2001,
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
    let mut children = label(
        Label {
            name: format!("TalentHero{}Name", tree.id),
            text: &tree.name,
            rect: [width / 2.0 - 212.5, 39.0, 425.0, 24.0],
            size: 24.0,
            color: TAB_TEXT_SELECTED,
            justify: "CENTER",
        },
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
        if view.editor.rank(node.id, entry.id) > 0 {
            "talents-heroclass-ring-selectionpane-yellow"
        } else {
            "talents-heroclass-ring-selectionpane-gray"
        },
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
    let points = view.hero_points_for(tree).to_string();
    children.extend(label(
        Label {
            name: format!("TalentHero{}DialogPoints", tree.id),
            text: &points,
            rect: [width - 212.0, HERO_DIALOG_NODES_TOP - 12.0, 54.0, 40.0],
            size: 32.0,
            color: "0.0,1.0,0.0,1.0",
            justify: "RIGHT",
        },
        scale,
    ));
    children.extend(label(
        Label {
            name: format!("TalentHero{}DialogPointsLabel", tree.id),
            text: "Points Available",
            rect: [width - 153.0, HERO_DIALOG_NODES_TOP - 9.0, 100.0, 40.0],
            size: 11.0,
            color: TAB_TEXT_SELECTED,
            justify: "LEFT",
        },
        scale,
    ));
    let layout = hero_tree_layout(tree, [width / 2.0, HERO_DIALOG_NODES_TOP], 1.0);
    children.extend(render_tree(tree, view, layout, scale));
    let action = format!("talent:choice:{}:{}", node.id, entry.id);
    let enabled = view
        .editor
        .can_purchase(view, view.level, node.id, entry.id);
    if view.editor.rank(node.id, entry.id) > 0 && view.editor.hero_dirty(view) {
        children.extend(panel_button(
            format!("TalentHero{}Apply", tree.id),
            "Apply Changes",
            "talent:apply",
            view.editor.apply().is_some(),
            (
                (width / 2.0 - 80.0) * scale,
                (height - 70.0) * scale,
                160.0 * scale,
                22.0 * scale,
            ),
        ));
    } else if view.editor.rank(node.id, entry.id) > 0 {
        children.extend(label(
            Label {
                name: format!("TalentHero{}Active", tree.id),
                text: "Active",
                rect: [width / 2.0 - 80.0, height - 70.0, 160.0, 22.0],
                size: 18.0,
                color: "0.0,1.0,0.0,1.0",
                justify: "CENTER",
            },
            scale,
        ));
    } else {
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
    }
    rsx! {r#frame {name:{DynName(format!("TalentHero{}Option",tree.id))},width:{width*scale},height:{height*scale},
        pos_type:"absolute",pos_x:{x*scale},pos_y:0.0,{children}
    }}
}

// ClassTalentsFrame.xml:326, WarmodeButtonTemplate.xml:100x100, RIGHT -6/+2.
fn war_mode_control(view: &TalentView, scale: f32) -> Element {
    let desired = view.war_mode.as_ref().is_some_and(|mode| mode.desired);
    let permitted = view.war_mode.as_ref().is_some_and(|mode| {
        if mode.desired {
            mode.can_disable
        } else {
            mode.can_enable
        }
    });
    let disabled = !permitted || view.pvp_pending;
    let action = if disabled { "" } else { "pvp:warmode" };
    let origin = [BOOK_W - 106.0, BOOK_H - FOOTER_HEIGHT / 2.0 - 52.0];
    let suffix = if desired { "" } else { "-disabled" };
    let mut children = warmode_layer(
        "WarmodeIndent",
        "talents-warmode-indent",
        origin,
        6.0,
        scale,
    );
    children.extend(warmode_layer(
        "WarmodeOrb",
        "pvptalents-warmode-orb",
        origin,
        14.0,
        scale,
    ));
    children.extend(warmode_layer(
        "WarmodeSwords",
        &format!("pvptalents-warmode-swords{suffix}"),
        origin,
        39.0,
        scale,
    ));
    children.extend(warmode_layer(
        "WarmodeRing",
        &format!("talents-warmode-ring{suffix}"),
        origin,
        5.0,
        scale,
    ));
    children.extend(
        rsx! { button { name: "WarmodeButton", onclick: action, disabled,
            width: {100.0*scale}, height: {100.0*scale}, button_default_skin: false,
            pos_type: "absolute", pos_x: {origin[0]*scale}, pos_y: {origin[1]*scale},
        } },
    );
    children
}
fn warmode_layer(name: &str, atlas: &str, origin: [f32; 2], bottom: f32, scale: f32) -> Element {
    let region = resolve_region(atlas, ActiveSkin::Modern).expect("Retail War Mode atlas");
    retail_atlas(
        name,
        atlas,
        [
            origin[0] + (100.0 - region.width) / 2.0,
            origin[1] + 100.0 - bottom - region.height,
            region.width,
            region.height,
        ],
        scale,
    )
}
