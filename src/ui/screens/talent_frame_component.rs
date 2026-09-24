//! Retail talent window (`PlayerSpellsFrame`): class tree, hero tree and spec
//! tree side by side, Wide window class (centered, 1000x680, top y=104).
//! Art is the Retail `talents-*` atlas members; see `docs/wiki/systems/talents-ui.md`.

use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::state_panel::{PanelState, state_panel};

use crate::talent_tree::AtlasCrop;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const PLAYER_SPELLS_FRAME: &str = "PlayerSpellsFrame";
pub const TALENT_APPLY_BUTTON: &str = "TalentApplyButton";
pub const TALENT_RESET_BUTTON: &str = "TalentResetButton";
pub const TALENT_STATE_PANEL: &str = "TalentTreeStatePanel";
pub const FRAME_W: f32 = 1000.0;
pub const FRAME_H: f32 = 680.0;
pub const FRAME_TOP: f32 = 104.0;
/// Tree area inside the frame: below the header and section labels, above the footer.
pub const TREE_TOP: f32 = 86.0;
pub const TREE_BOTTOM: f32 = FRAME_H - 52.0;
pub const TREE_INSET: f32 = 20.0;
pub const NODE_SIZE: f32 = 38.0;
pub const CHOICE_W: f32 = 44.0;
const EDGE_THICKNESS: f32 = 4.0;
const HEADER_H: f32 = 34.0;
const LABEL_Y: f32 = 56.0;
const SPEC_BUTTON_W: f32 = 118.0;
const SPEC_BUTTON_H: f32 = 24.0;
const FOOTER_BUTTON_W: f32 = 112.0;
const FOOTER_BUTTON_H: f32 = 26.0;

const FRAME_BG: &str = "0.02,0.02,0.03,0.96";
const FRAME_BORDER: &str = "1px solid 0.55,0.45,0.2,0.95";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const LABEL_COLOR: &str = "1.0,0.82,0.0,1.0";
const RANK_COLOR_MAXED: &str = "1.0,0.82,0.0,1.0";
const RANK_COLOR_OPEN: &str = "0.1,1.0,0.1,1.0";
const RANK_COLOR_LOCKED: &str = "0.6,0.6,0.6,1.0";
const RANK_BG: &str = "0.0,0.0,0.0,0.85";
const LOCKED_ICON_COLOR: &str = "0.45,0.45,0.45,1.0";
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

pub const ACTION_TALENT_NODE_PREFIX: &str = "talent_node:";
pub const ACTION_TALENT_CHOICE_PREFIX: &str = "talent_choice:";
pub const ACTION_TALENT_SPEC_PREFIX: &str = "talent_spec:";
pub const ACTION_TALENT_APPLY: &str = "talent_apply";
pub const ACTION_TALENT_RESET: &str = "talent_reset";

pub fn talent_node_name(node_id: u32) -> String {
    format!("TalentNode_{node_id}")
}

pub fn talent_choice_name(node_id: u32, entry_id: u32) -> String {
    format!("TalentNode_{node_id}Choice{entry_id}")
}

pub fn talent_edge_name(from: u32, to: u32) -> String {
    format!("TalentEdge_{from}_{to}")
}

pub fn talent_spec_button_name(index: usize) -> String {
    format!("TalentSpecButton{index}")
}

/// Node button shape: Retail squares are active spells, circles passives,
/// octagons choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeShape {
    Square,
    Circle,
    Choice,
}

/// Border colour of a node: maxed (yellow), purchasable or partly ranked
/// (green), or not purchasable now (gray).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeLook {
    Maxed,
    Open,
    Locked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceIconView {
    pub entry_id: u32,
    pub icon_fdid: u32,
    pub chosen: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeView {
    pub node_id: u32,
    /// Center, frame coordinates.
    pub x: f32,
    pub y: f32,
    pub shape: NodeShape,
    pub look: NodeLook,
    pub rank_text: String,
    pub border: Option<AtlasCrop>,
    /// One icon; two for choice nodes.
    pub icons: Vec<ChoiceIconView>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeView {
    pub from: u32,
    pub to: u32,
    /// Center, frame coordinates.
    pub x: f32,
    pub y: f32,
    pub length: f32,
    /// Screen angle (y down), radians.
    pub angle: f32,
    pub art: Option<AtlasCrop>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpecButtonView {
    pub spec_id: u32,
    pub name: String,
    pub active: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SectionLabel {
    pub text: String,
    /// Left edge of the section, frame coordinates.
    pub x: f32,
    pub width: f32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TalentTreeView {
    pub background: Option<AtlasCrop>,
    pub class_label: Option<SectionLabel>,
    pub spec_label: Option<SectionLabel>,
    pub hero_label: Option<SectionLabel>,
    pub specs: Vec<SpecButtonView>,
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    pub has_pending: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TalentBody {
    Loading,
    Failed(String),
    Tree(TalentTreeView),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TalentFrameState {
    pub visible: bool,
    pub body: TalentBody,
}

pub fn talent_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TalentFrameState>()
        .expect("TalentFrameState must be in SharedContext");
    let hide = !state.visible;
    let body = match &state.body {
        TalentBody::Loading => state_panel(TALENT_STATE_PANEL, PanelState::Loading { label: None }),
        TalentBody::Failed(message) => state_panel(
            TALENT_STATE_PANEL,
            PanelState::Error {
                message,
                retry_action: None,
            },
        ),
        TalentBody::Tree(view) => tree_body(view),
    };
    rsx! {
        r#frame {
            name: "PlayerSpellsFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::High,
            hidden: hide,
            mouse_enabled: true,
            background_color: FRAME_BG,
            border: FRAME_BORDER,
            pos_type: "absolute",
            anchor: "screen",
            left: "50%",
            translate_x: "-50%",
            top: {FRAME_TOP},
            fontstring {
                name: "PlayerSpellsFrameTitle",
                width: {FRAME_W},
                height: {HEADER_H},
                text: "Talents",
                font: "FrizQuadrata",
                font_size: 16.0,
                font_color: TITLE_COLOR,
                justify_h: "CENTER",
                pos_type: "absolute",
                left: 0.0,
                top: 4.0,
            }
            {body}
        }
    }
}

fn tree_body(view: &TalentTreeView) -> Element {
    let mut elements = Vec::new();
    if let Some(background) = view.background {
        elements.extend(background_texture(background));
    }
    elements.extend(spec_buttons(&view.specs));
    for (name, label) in [
        ("TalentClassPointsText", &view.class_label),
        ("TalentHeroPointsText", &view.hero_label),
        ("TalentSpecPointsText", &view.spec_label),
    ] {
        if let Some(label) = label {
            elements.extend(section_label(name, label));
        }
    }
    elements.extend(view.edges.iter().flat_map(edge_texture));
    elements.extend(view.nodes.iter().flat_map(node_button));
    elements.extend(footer_button(
        TALENT_RESET_BUTTON,
        "Reset",
        ACTION_TALENT_RESET,
        FRAME_W / 2.0 - FOOTER_BUTTON_W - 8.0,
        !view.has_pending,
    ));
    elements.extend(footer_button(
        TALENT_APPLY_BUTTON,
        "Apply Changes",
        ACTION_TALENT_APPLY,
        FRAME_W / 2.0 + 8.0,
        !view.has_pending,
    ));
    elements
}

fn background_texture(background: AtlasCrop) -> Element {
    let coords = tex_coords(background.tex_coords);
    rsx! {
        texture {
            name: "PlayerSpellsFrameBackground",
            width: {FRAME_W - 2.0},
            height: {TREE_BOTTOM - HEADER_H},
            texture_fdid: {background.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: 1.0,
            top: {HEADER_H},
        }
    }
}

fn spec_buttons(specs: &[SpecButtonView]) -> Element {
    let count = specs.len() as f32;
    let total_w = count * SPEC_BUTTON_W + (count - 1.0).max(0.0) * 6.0;
    let left = FRAME_W - TREE_INSET - total_w;
    specs
        .iter()
        .enumerate()
        .flat_map(|(index, spec)| {
            let x = left + index as f32 * (SPEC_BUTTON_W + 6.0);
            let action = if spec.enabled && !spec.active {
                format!("{ACTION_TALENT_SPEC_PREFIX}{}", spec.spec_id)
            } else {
                String::new()
            };
            let disabled = !spec.enabled || spec.active;
            let label = if spec.active {
                format!("> {} <", spec.name)
            } else {
                spec.name.clone()
            };
            rsx! {
                button {
                    name: {DynName(talent_spec_button_name(index))},
                    width: {SPEC_BUTTON_W},
                    height: {SPEC_BUTTON_H},
                    text: {label.as_str()},
                    font_size: 11.0,
                    onclick: {action.as_str()},
                    disabled: {disabled},
                    button_atlas_up: BUTTON_ATLAS_UP,
                    button_atlas_pressed: BUTTON_ATLAS_PRESSED,
                    button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
                    button_atlas_disabled: BUTTON_ATLAS_DISABLED,
                    pos_type: "absolute",
                    left: {x},
                    top: 6.0,
                }
            }
        })
        .collect()
}

fn section_label(name: &str, label: &SectionLabel) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: {label.width},
            height: 20.0,
            text: {label.text.as_str()},
            font: "FrizQuadrata",
            font_size: 13.0,
            font_color: LABEL_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: {label.x},
            top: {LABEL_Y},
        }
    }
}

fn edge_texture(edge: &EdgeView) -> Element {
    let Some(art) = edge.art else {
        return Vec::new();
    };
    let coords = tex_coords(art.tex_coords);
    rsx! {
        texture {
            name: {DynName(talent_edge_name(edge.from, edge.to))},
            width: {edge.length},
            height: {EDGE_THICKNESS},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: {edge.x - edge.length / 2.0},
            top: {edge.y - EDGE_THICKNESS / 2.0},
        }
    }
}

fn node_size(shape: NodeShape) -> (f32, f32) {
    match shape {
        NodeShape::Choice => (CHOICE_W, NODE_SIZE),
        _ => (NODE_SIZE, NODE_SIZE),
    }
}

fn node_button(node: &NodeView) -> Element {
    let (w, h) = node_size(node.shape);
    let name = talent_node_name(node.node_id);
    let action = if node.shape == NodeShape::Choice {
        String::new()
    } else {
        format!("{ACTION_TALENT_NODE_PREFIX}{}", node.node_id)
    };
    let icons = node_icons(node, &name, w, h);
    let border = node.border.map(|border| node_border(&name, border, w, h));
    let rank = node_rank(&name, node, w, h);
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: {w},
            height: {h},
            mouse_enabled: true,
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: {node.x - w / 2.0},
            top: {node.y - h / 2.0},
            {icons}
            {border.unwrap_or_default()}
            {rank}
        }
    }
}

/// Square icons fill the ring's inner area; circles use a smaller inset so the
/// square icon corners stay inside the ring.
fn icon_inset(shape: NodeShape, w: f32) -> f32 {
    match shape {
        NodeShape::Square => w * 0.12,
        NodeShape::Circle => w * 0.2,
        NodeShape::Choice => w * 0.16,
    }
}

fn node_icons(node: &NodeView, name: &str, w: f32, h: f32) -> Element {
    let inset = icon_inset(node.shape, w);
    let color = if node.look == NodeLook::Locked {
        LOCKED_ICON_COLOR
    } else {
        "1.0,1.0,1.0,1.0"
    };
    if node.shape != NodeShape::Choice {
        let Some(icon) = node.icons.first() else {
            return Vec::new();
        };
        return rsx! {
            texture {
                name: {DynName(format!("{name}Icon"))},
                width: {w - 2.0 * inset},
                height: {h - 2.0 * inset},
                texture_fdid: {icon.icon_fdid},
                vertex_color: color,
                pos_type: "absolute",
                left: {inset},
                top: {inset},
            }
        };
    }
    let half = (w - 2.0 * inset) / 2.0;
    node.icons
        .iter()
        .take(2)
        .enumerate()
        .flat_map(|(index, icon)| {
            let coords = if index == 0 {
                "0.0,0.5,0.0,1.0"
            } else {
                "0.5,1.0,0.0,1.0"
            };
            let dim = node.icons.iter().any(|icon| icon.chosen) && !icon.chosen;
            let icon_color = if dim { LOCKED_ICON_COLOR } else { color };
            let action = format!(
                "{ACTION_TALENT_CHOICE_PREFIX}{}:{}",
                node.node_id, icon.entry_id
            );
            rsx! {
                texture {
                    name: {DynName(talent_choice_name(node.node_id, icon.entry_id))},
                    width: {half},
                    height: {h - 2.0 * inset},
                    texture_fdid: {icon.icon_fdid},
                    tex_coords: coords,
                    vertex_color: icon_color,
                    mouse_enabled: true,
                    onclick: {action.as_str()},
                    pos_type: "absolute",
                    left: {inset + index as f32 * half},
                    top: {inset},
                }
            }
        })
        .collect()
}

fn node_border(name: &str, border: AtlasCrop, w: f32, h: f32) -> Element {
    let coords = tex_coords(border.tex_coords);
    rsx! {
        texture {
            name: {DynName(format!("{name}Border"))},
            width: {w},
            height: {h},
            texture_fdid: {border.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

fn node_rank(name: &str, node: &NodeView, w: f32, h: f32) -> Element {
    if node.rank_text.is_empty() {
        return Vec::new();
    }
    let color = match node.look {
        NodeLook::Maxed => RANK_COLOR_MAXED,
        NodeLook::Open => RANK_COLOR_OPEN,
        NodeLook::Locked => RANK_COLOR_LOCKED,
    };
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Rank"))},
            width: 26.0,
            height: 12.0,
            text: {node.rank_text.as_str()},
            font: "FrizQuadrata",
            font_size: 9.0,
            font_color: color,
            background_color: RANK_BG,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: {w - 22.0},
            top: {h - 9.0},
        }
    }
}

fn footer_button(name: &str, text: &str, action: &str, x: f32, disabled: bool) -> Element {
    let action = if disabled { "" } else { action };
    rsx! {
        button {
            name: {DynName(name.to_string())},
            width: {FOOTER_BUTTON_W},
            height: {FOOTER_BUTTON_H},
            text: text,
            font_size: 12.0,
            onclick: action,
            disabled: {disabled},
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            left: {x},
            top: {FRAME_H - FOOTER_BUTTON_H - 12.0},
        }
    }
}

fn tex_coords(coords: [f32; 4]) -> String {
    format!("{},{},{},{}", coords[0], coords[1], coords[2], coords[3])
}
