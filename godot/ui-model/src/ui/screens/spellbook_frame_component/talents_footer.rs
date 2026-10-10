//! ClassTalentsFrame.xml: LoadSystem LEFT48, SearchBox LEFT20, Reset/Undo shared anchor.
use super::*;
use crate::talents::search::NOT_ON_ACTION_BAR;
use crate::ui::screens::menu_primitives::{DropdownButton, dropdown_button};

pub(super) fn controls(view: &TalentView, scale: f32) -> Element {
    let y = BOOK_H - FOOTER_HEIGHT / 2.0 - 15.0;
    let mut children = dropdown_button(DropdownButton {
        frame_name: "TalentLoadoutDropDown",
        label_name: "TalentLoadoutName",
        arrow_name: "TalentLoadoutArrow",
        text: "Default Loadout",
        width: 200.0 * scale,
        height: 30.0 * scale,
        x: 48.0 * scale,
        y: y * scale,
        background_color: "0.04,0.04,0.04,1.0",
        text_color: TAB_TEXT,
        arrow_color: TAB_TEXT,
        onclick: Some("talent:loadouts"),
    });
    children.extend(search_box(view, y, scale));
    if view.editor.reset_menu {
        children.extend(menu(
            "TalentResetMenu",
            [BOOK_W / 2.0 + 96.0, y - 106.0, 200.0, 106.0],
            vec![
                menu_row(
                    "TalentResetClass",
                    "Reset Class Talents",
                    "talent:reset:class",
                    true,
                    0,
                    200.0,
                    scale,
                ),
                menu_row(
                    "TalentResetSpec",
                    "Reset Specialization",
                    "talent:reset:spec",
                    true,
                    1,
                    200.0,
                    scale,
                ),
                menu_row(
                    "TalentResetAll",
                    "Reset All Talents",
                    "talent:reset:all",
                    true,
                    2,
                    200.0,
                    scale,
                ),
            ]
            .into_iter()
            .flatten()
            .collect(),
            scale,
        ));
    }
    if view.editor.loadout_menu {
        // Protocol exposes only the active per-spec snapshot: no saved config IDs/names.
        children.extend(menu(
            "TalentLoadoutMenu",
            [48.0, y - 106.0, 200.0, 106.0],
            vec![
                menu_row(
                    "TalentNewLoadout",
                    "New Loadout",
                    "",
                    false,
                    0,
                    200.0,
                    scale,
                ),
                menu_row(
                    "TalentImportLoadout",
                    "Import Loadout",
                    "",
                    false,
                    1,
                    200.0,
                    scale,
                ),
                menu_row(
                    "TalentExportLoadout",
                    "Export Loadout",
                    "",
                    false,
                    2,
                    200.0,
                    scale,
                ),
            ]
            .into_iter()
            .flatten()
            .collect(),
            scale,
        ));
    }
    children
}

fn search_box(view: &TalentView, y: f32, scale: f32) -> Element {
    let text = view.editor.search_text.as_str();
    let mut children = Vec::new();
    for (suffix, atlas, rect) in [
        (
            "Left",
            "common-search-border-left",
            [268.0, y + 5.0, 8.0, 20.0],
        ),
        (
            "Middle",
            "common-search-border-middle",
            [276.0, y + 5.0, 168.0, 20.0],
        ),
        (
            "Right",
            "common-search-border-right",
            [444.0, y + 5.0, 8.0, 20.0],
        ),
        (
            "Icon",
            "common-search-magnifyingglass",
            [274.0, y + 10.0, 10.0, 10.0],
        ),
    ] {
        children.extend(retail_atlas(
            &format!("TalentSearch{suffix}"),
            atlas,
            rect,
            scale,
        ));
    }
    children.extend(rsx! {
        editbox { name: "TalentSearchBox", text, width:{184.0*scale},height:{30.0*scale},
            text_insets:"18,20,0,0",font_size:{12.0*scale},pos_type:"absolute",left:{268.0*scale},top:{y*scale} }
        fontstring { name:"TalentSearchHint",text:"Search",hidden:{!text.is_empty()},mouse_enabled:false,
            width:{130.0*scale},height:{30.0*scale},font_size:{12.0*scale},font_color:"0.5,0.5,0.5,1.0",justify_h:"LEFT",
            pos_type:"absolute",left:{286.0*scale},top:{y*scale} }
    });
    if !text.is_empty() {
        let icon = retail_atlas(
            "TalentSearchClearIcon",
            "common-search-clearbutton",
            [3.0, 3.0, 10.0, 10.0],
            scale,
        );
        children.extend(rsx! { button {name:"TalentSearchClear",onclick:"talent:search_clear",button_default_skin:false,
            width:{16.0*scale},height:{16.0*scale},pos_type:"absolute",left:{432.0*scale},top:{(y+7.0)*scale},{icon}} });
    }
    if view.editor.search_preview {
        let entries = view.search_preview_entries();
        let mut rows = Vec::new();
        if text.chars().count() < 2 {
            rows.extend(menu_row(
                "TalentSearchNotOnBar",
                NOT_ON_ACTION_BAR,
                "talent:search_select_bar",
                true,
                0,
                240.0,
                scale,
            ));
        } else {
            for (index, (entry, name)) in entries.iter().take(3).enumerate() {
                rows.extend(menu_row(
                    &format!("TalentSearchPreview{entry}"),
                    name,
                    &format!("talent:search_select:{entry}"),
                    true,
                    index,
                    240.0,
                    scale,
                ));
            }
        }
        if !rows.is_empty() {
            let overflow = entries.len().saturating_sub(3);
            let height = (entries.len().clamp(1, 3) as f32) * 28.0
                + 12.0
                + if overflow > 0 { 20.0 } else { 0.0 };
            if overflow > 0 {
                rows.extend(label(
                    Label {
                        name: "TalentSearchOverflow".into(),
                        text: &format!("{overflow} more results"),
                        rect: [6.0, height - 24.0, 228.0, 20.0],
                        size: 12.0,
                        color: TAB_TEXT,
                        justify: "LEFT",
                    },
                    scale,
                ));
            }
            children.extend(menu(
                "TalentSearchPreviewContainer",
                [264.0, y - height, 240.0, height],
                rows,
                scale,
            ));
        }
    }
    children
}

fn menu(name: &str, rect: [f32; 4], children: Element, scale: f32) -> Element {
    rsx! { r#frame {name:{DynName(name.into())},width:{rect[2]*scale},height:{rect[3]*scale},
    background_color:"0.04,0.04,0.04,1.0",strata:FrameStrata::Dialog,frame_level:2000,
    pos_type:"absolute",left:{rect[0]*scale},top:{rect[1]*scale},{children}} }
}
fn menu_row(
    name: &str,
    text: &str,
    action: &str,
    enabled: bool,
    index: usize,
    width: f32,
    scale: f32,
) -> Element {
    use crate::ui::screens::quest_art::panel_button;
    panel_button(
        name.into(),
        text,
        action,
        enabled,
        (
            6.0 * scale,
            (6.0 + index as f32 * 28.0) * scale,
            (width - 12.0) * scale,
            22.0 * scale,
        ),
    )
}
