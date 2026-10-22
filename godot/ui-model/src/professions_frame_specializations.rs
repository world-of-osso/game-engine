//! Mainline ProfessionsSpecPageTemplate: tree at left, selected path dial at right.
use super::{ProfessionView, art, text};
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, label, texture};
use crate::quest_art::{DynName, panel_button};
use shared::protocol::{ProfessionSpecNodeSnapshot, ProfessionSpecTreeSnapshot};
use ui_toolkit::{
    rsx,
    widget_def::{Attr, Element, WidgetChild},
};

#[path = "professions_specializations_layout.rs"]
mod layout;
#[path = "professions_frame_perks.rs"]
mod perks;

pub(super) fn page(view: &ProfessionView) -> Element {
    let state = &view.specializations;
    let mut out = art::atlas(
        "ProfessionSpecFooter",
        "Professions-Specializations-Background-Footer",
        (1.0, 602.0, 1143.0, 53.0),
    );
    out.extend(text(
        "ProfessionSpecKnowledge",
        &format!("Knowledge: {}", state.remaining_knowledge()),
        (850.0, 40.0, 265.0, 24.0),
    ));
    let Some(snapshot) = &state.snapshot else {
        out.extend(text(
            "ProfessionSpecLoading",
            if state.pending {
                "Loading specializations…"
            } else {
                "Specializations are unavailable for this profession tier."
            },
            (32.0, 110.0, 650.0, 28.0),
        ));
        out.extend(text(
            "ProfessionSpecStatus",
            &state.status,
            (32.0, 570.0, 1000.0, 24.0),
        ));
        return out;
    };
    for (i, tree) in snapshot.trees.iter().enumerate() {
        let left = 20.0 + i as f32 * 190.0;
        out.extend(art::top_tab(
            &format!("ProfessionSpecTree{}", tree.tree_id),
            &tree.name,
            &format!("profession:spec:tree:{}", tree.tree_id),
            state.selected_tree == Some(tree.tree_id),
            (left, 75.0, 180.0, 30.0),
        ));
        if !tree.unlocked {
            out.extend(art::atlas(
                &format!("ProfessionSpecTree{}Lock", tree.tree_id),
                "Professions_Specialization_Lock",
                (left + 4.0, 74.0, 26.0, 32.0),
            ));
        }
    }
    let Some(tree) = state.tree() else {
        return out;
    };
    // Professions.lua:GetProfessionSpecializationBackgroundAtlas uses the profession kit.
    let member = format!(
        "Professions-Specializations-Background-{}",
        view.profession_name
    );
    out.extend(art::atlas(
        "ProfessionSpecTreeBackground",
        &member,
        (2.0, 85.0, 706.0, 521.0),
    ));
    out.extend(art::atlas(
        "ProfessionSpecDialBackground",
        "Professions-Specializations-Background-Dial",
        (668.0, 85.0, 474.0, 521.0),
    ));
    out.extend(label(
        "ProfessionSpecTreeName".into(),
        &tree.name,
        (32.0, 97.0, 650.0, 30.0),
        (18.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    let centers = match layout::path_centers(tree) {
        Ok(centers) => centers,
        Err(error) => {
            out.extend(text(
                "ProfessionSpecLayoutError",
                &error,
                (32.0, 160.0, 650.0, 100.0),
            ));
            return out;
        }
    };
    out.extend(edges(tree, &centers));
    for node in state.path_nodes() {
        let center = centers[&node.node_id];
        out.extend(node_button(view, node, center));
    }
    out.extend(detailed_path(view));
    let has_changes = state
        .purchases
        .keys()
        .any(|(tree, _, _)| Some(*tree) == state.selected_tree);
    if tree.unlocked {
        out.extend(panel_button(
            "ProfessionSpecApply".into(),
            "Apply Changes",
            "profession:spec:apply",
            has_changes && !state.pending,
            (490.0, 625.0, 164.0, 22.0),
        ));
        out.extend(panel_button(
            "ProfessionSpecUndo".into(),
            "Undo",
            "profession:spec:undo",
            has_changes && !state.pending,
            (668.0, 625.0, 64.0, 22.0),
        ));
    } else {
        out.extend(panel_button(
            "ProfessionSpecUnlock".into(),
            "Unlock Specialization",
            "profession:spec:unlock",
            state.can_unlock(),
            (460.0, 625.0, 230.0, 22.0),
        ));
    }
    out.extend(text(
        "ProfessionSpecStatus",
        &state.status,
        (32.0, 575.0, 1080.0, 22.0),
    ));
    if state.confirming_unlock {
        out.extend(unlock_confirmation(view));
    }
    out
}
fn edges(
    tree: &ProfessionSpecTreeSnapshot,
    centers: &std::collections::BTreeMap<u32, [f32; 2]>,
) -> Element {
    let mut out = vec![];
    for (i, edge) in tree
        .edges
        .iter()
        .filter(|e| e.visual_style != 0)
        .enumerate()
    {
        let (Some(from), Some(to)) = (
            centers.get(&edge.left_node_id),
            centers.get(&edge.right_node_id),
        ) else {
            continue;
        };
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        let length = dx.hypot(dy);
        if length == 0.0 {
            continue;
        }
        let mut line = art::atlas(
            &format!("ProfessionSpecEdge{i}"),
            "Professions_Specialization_arrowline",
            (
                (from[0] + to[0] - length) * 0.5,
                (from[1] + to[1]) * 0.5 - 3.0,
                length,
                6.0,
            ),
        );
        let WidgetChild::Widget(widget) = &mut line[0] else {
            unreachable!()
        };
        widget
            .attrs
            .push(Attr::new_dynamic("rotation", (-dy.atan2(dx)).to_string()));
        out.extend(line);
    }
    out
}
fn node_button(
    view: &ProfessionView,
    node: &ProfessionSpecNodeSnapshot,
    center: [f32; 2],
) -> Element {
    let state = &view.specializations;
    let mut children = art::atlas(
        &format!("ProfessionSpecNode{}Ring", node.node_id),
        "Professions-Specialization-Node-EmptyPath",
        (-11.5, -11.5, 78.0, 78.0),
    );
    children.extend(texture(
        format!("ProfessionSpecNode{}Icon", node.node_id),
        node.icon,
        (4.5, 4.5, 46.0, 46.0),
        "1,1,1,1",
    ));
    if Some(node.node_id) == state.selected_node {
        children.extend(art::atlas(
            &format!("ProfessionSpecNode{}Selected", node.node_id),
            "Professions-Specialization-Node-SelectedGlow",
            (-11.5, -11.5, 78.0, 78.0),
        ));
    }
    if !state.path_unlocked(node.node_id) {
        children.extend(art::atlas(
            &format!("ProfessionSpecNode{}Lock", node.node_id),
            "Professions_Specialization_Lock",
            (18.5, 32.0, 18.0, 24.0),
        ));
    }
    let (rank, maximum) = state
        .display_ranks(node.node_id)
        .expect("displayed source path");
    children.extend(label(
        format!("ProfessionSpecRank{}", node.node_id),
        &format!("{rank}/{maximum}"),
        (0.0, 53.0, 55.0, 20.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    let action = format!("profession:spec:node:{}", node.node_id);
    rsx! {button{name:{DynName(format!("ProfessionSpecNode{}",node.node_id))},left:{center[0]-27.5},top:{center[1]-27.5},width:55.0,height:55.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,
    tooltip_provider:"profession_specialization",{children}}}
}
fn detailed_path(view: &ProfessionView) -> Element {
    let state = &view.specializations;
    let Some(node) = state.node() else {
        return vec![];
    };
    let mut out = label(
        "ProfessionSpecPathName".into(),
        &node.name,
        (739.0, 119.0, 373.0, 30.0),
        (18.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    );
    for member in [
        "SpecDial_Outer_Base",
        "SpecDial_Outer_TitanLineRing",
        "SpecDial_Outer_Top",
        "SpecDial_Center_InnerRing",
    ] {
        out.extend(art::atlas(
            &format!("ProfessionSpecDial{member}"),
            member,
            (757.0, 180.0, 336.0, 336.0),
        ));
    }
    out.extend(texture(
        "ProfessionSpecDetailedIcon".into(),
        node.icon,
        (893.5, 316.5, 63.0, 63.0),
        "1,1,1,1",
    ));
    let (rank, maximum) = state
        .display_ranks(node.node_id)
        .expect("selected source path");
    out.extend(label(
        "ProfessionSpecDetailedRank".into(),
        &format!("{rank}/{maximum}"),
        (848.0, 394.0, 155.0, 22.0),
        (16.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    out.extend(text(
        "ProfessionSpecDescription",
        &node.description,
        (745.0, 518.0, 365.0, 50.0),
    ));
    out.extend(perks::dial(view));
    out.extend(panel_button(
        "ProfessionSpecSpend".into(),
        if state.path_unlocked(node.node_id) {
            "Spend Point"
        } else {
            "Unlock Path"
        },
        "profession:spec:spend",
        state.can_stage(),
        (843.0, 573.0, 164.0, 22.0),
    ));
    out
}
fn unlock_confirmation(view: &ProfessionView) -> Element {
    let state = &view.specializations;
    let Some(tree) = state.tree() else {
        return vec![];
    };
    let mut children = crate::quest_art::window_chrome(
        "ProfessionSpecConfirm",
        (420.0, 180.0),
        "Unlock Specialization",
        "profession:spec:cancel-unlock",
    );
    children.extend(text(
        "ProfessionSpecConfirmText",
        &format!("Unlock {}? This choice cannot be undone.", tree.name),
        (24.0, 48.0, 370.0, 55.0),
    ));
    children.extend(panel_button(
        "ProfessionSpecConfirmYes".into(),
        "Yes",
        "profession:spec:confirm-unlock",
        state.can_unlock(),
        (85.0, 130.0, 100.0, 22.0),
    ));
    children.extend(panel_button(
        "ProfessionSpecConfirmCancel".into(),
        "Cancel",
        "profession:spec:cancel-unlock",
        true,
        (235.0, 130.0, 100.0, 22.0),
    ));
    rsx! {r#frame{name:"ProfessionSpecConfirm",left:362.0,top:230.0,width:420.0,height:180.0,pos_type:"absolute",mouse_enabled:true,strata:crate::ui::strata::FrameStrata::FullscreenDialog,{children}}}
}
