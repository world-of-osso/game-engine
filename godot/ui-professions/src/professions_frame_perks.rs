//! Mainline selected-path perk dial: source milestones, major bit and 90px flipbook cells.
use super::super::{ProfessionView, art};
use game_engine_ui_model::quest_art::DynName;
use shared::protocol::ProfessionSpecNodeSnapshot;
use ui_toolkit::{
    rsx,
    widget_def::{Attr, Element, WidgetChild},
};

const DIAL_CENTER: [f32; 2] = [925.0, 348.0];
const DIAL_BASE_ANGLE: f32 = 1.4;
const PERK_SIZE: f32 = 90.0;
const MINOR_PERK_RADIUS: f32 = 177.0;
const MAJOR_PERK_RADIUS: f32 = 189.0;
const PENDING_GLOW_OFFSET: f32 = 8.0;

#[derive(Clone, Copy)]
struct PathRanks {
    committed: u32,
    pending: u32,
    maximum: u32,
}

pub(super) fn dial(view: &ProfessionView) -> Element {
    let state = &view.specializations;
    let (Some(tree), Some(path)) = (state.tree(), state.node()) else {
        return vec![];
    };
    let (pending, maximum) = state
        .display_ranks(path.node_id)
        .expect("selected source path");
    let (committed, _) = state
        .committed_display_ranks(path.node_id)
        .expect("selected source path");
    let ranks = PathRanks {
        committed,
        pending,
        maximum,
    };
    tree.nodes
        .iter()
        .filter(|node| {
            node.perk
                .as_ref()
                .is_some_and(|perk| perk.path_node_id == path.node_id)
        })
        .flat_map(|node| perk(view, node, ranks))
        .collect()
}

fn perk(view: &ProfessionView, node: &ProfessionSpecNodeSnapshot, ranks: PathRanks) -> Element {
    let perk = node.perk.as_ref().expect("source perk");
    assert!(ranks.maximum > 0, "source perk path has no knowledge ranks");
    let rotation = perk.required_points as f32 / ranks.maximum as f32
        * (std::f32::consts::TAU - DIAL_BASE_ANGLE)
        + DIAL_BASE_ANGLE / 2.0;
    let radius = if perk.is_major_bonus {
        MAJOR_PERK_RADIUS
    } else {
        MINOR_PERK_RADIUS
    };
    let left = DIAL_CENTER[0] - radius * rotation.sin() - PERK_SIZE / 2.0;
    let top = DIAL_CENTER[1] + radius * rotation.cos() - PERK_SIZE / 2.0;
    let name = format!("ProfessionSpecPerk{}", node.node_id);
    let children = milestone_art(view, node, ranks, rotation);
    rsx! { r#frame { name: {DynName(name)}, left, top, width: PERK_SIZE, height: PERK_SIZE,
    pos_type: "absolute", mouse_enabled: true, tooltip_provider: "profession_specialization", {children} } }
}

fn milestone_art(
    view: &ProfessionView,
    node: &ProfessionSpecNodeSnapshot,
    ranks: PathRanks,
    rotation: f32,
) -> Element {
    let perk = node.perk.as_ref().expect("source perk");
    let earned = ranks.committed >= perk.required_points;
    let pending = !earned && ranks.pending >= perk.required_points;
    let member = if perk.is_major_bonus { "EndPip" } else { "Pip" };
    let atlas = format!("SpecDial_{member}_Flipbook_{}", view.profession_name);
    let name = format!("ProfessionSpecPerk{}", node.node_id);
    let texture_rotation = std::f32::consts::TAU - rotation + std::f32::consts::PI;
    let mut children = rotate(
        art::dial_perk(
            &format!("{name}Artwork"),
            &atlas,
            earned,
            (0.0, 0.0, PERK_SIZE, PERK_SIZE),
        ),
        texture_rotation,
    );
    if pending {
        children.extend(pending_glow(
            &name,
            perk.is_major_bonus,
            rotation,
            texture_rotation,
        ));
    }
    children
}

fn pending_glow(name: &str, major: bool, rotation: f32, texture_rotation: f32) -> Element {
    let member = if major {
        "SpecDial_LastPip_BorderGlow"
    } else {
        "SpecDial_Pip_BorderGlow"
    };
    let center = [
        PERK_SIZE / 2.0 - PENDING_GLOW_OFFSET * rotation.sin(),
        PERK_SIZE / 2.0 + PENDING_GLOW_OFFSET * rotation.cos(),
    ];
    let region = ui_toolkit::atlas::get_region(member).expect("authored perk pending glow");
    rotate(
        art::atlas_at_size(
            &format!("{name}PendingGlow"),
            member,
            (
                center[0] - region.width / 2.0,
                center[1] - region.height / 2.0,
            ),
        ),
        texture_rotation,
    )
}

fn rotate(mut texture: Element, rotation: f32) -> Element {
    let WidgetChild::Widget(widget) = &mut texture[0] else {
        unreachable!("atlas texture");
    };
    widget
        .attrs
        .push(Attr::new_dynamic("rotation", rotation.to_string()));
    texture
}
