//! Retail LoadSystem rows and ClassTalentLoadoutDialogTemplate dimensions.
use super::*;
use crate::talents::LoadoutDialog;

pub(super) fn loadout_menu(view: &TalentView, y: f32, scale: f32) -> Element {
    let mut rows = Vec::new();
    let list = view.editor.loadouts.as_ref();
    let configs = list.map_or(&[][..], |list| list.configs.as_slice());
    let enabled = !view.editor.loadout_busy;
    let disabled = !enabled;
    for (index, config) in configs.iter().enumerate() {
        let action = format!("talent:loadout_select:{}", config.id);
        let edit = format!("talent:loadout_edit:{}", config.id);
        let name = format!("TalentLoadoutRow{}", config.id);
        let top = (6.0 + index as f32 * 28.0) * scale;
        let selected = list.is_some_and(|list| list.selected_id == config.id);
        let check = if selected {
            rsx! { texture { name:{DynName(format!("{name}Check"))},texture_fdid:130751,
            width:{20.0*scale},height:{20.0*scale},pos_type:"absolute",left:{142.0*scale},top:{4.0*scale} } }
        } else {
            Vec::new()
        };
        rows.extend(rsx! { button {name:{DynName(name.clone())},onclick:{action.as_str()},disabled,
            button_default_skin:false,width:{164.0*scale},height:{28.0*scale},pos_type:"absolute",left:{6.0*scale},top,
            fontstring {name:{DynName(format!("{name}Text"))},text:{config.name.as_str()},width:{136.0*scale},height:{28.0*scale},
                font_size:{12.0*scale},font_color:TAB_TEXT,justify_h:"LEFT",mouse_enabled:false} {check}
        }
        button {name:{DynName(format!("TalentLoadoutEdit{}",config.id))},onclick:{edit.as_str()},disabled,button_default_skin:false,
            width:{16.0*scale},height:{16.0*scale},pos_type:"absolute",left:{178.0*scale},top:{top+6.0*scale},
            texture {name:{DynName(format!("{name}Gear"))},texture_fdid:311226,width:{16.0*scale},height:{16.0*scale},mouse_enabled:false,
                pos_type:"absolute",left:0.0,top:0.0}
        } });
    }
    for (offset, name, label, action, available) in [
        (
            0,
            "TalentNewLoadout",
            "New Loadout",
            "talent:loadout_new",
            enabled && list.is_some(),
        ),
        (1, "TalentImportLoadout", "Import Loadout", "", false),
        (2, "TalentExportLoadout", "Export Loadout", "", false),
    ] {
        rows.extend(sentinel_row(
            name,
            label,
            action,
            available,
            configs.len() + offset,
            scale,
        ));
    }
    let height = 12.0 + (configs.len() + 3) as f32 * 28.0;
    menu(
        "TalentLoadoutMenu",
        [48.0, y - height, 200.0, height],
        rows,
        scale,
    )
}

fn sentinel_row(
    name: &str,
    text: &str,
    action: &str,
    enabled: bool,
    index: usize,
    scale: f32,
) -> Element {
    let color = if enabled { TAB_TEXT } else { "0.5,0.5,0.5,1.0" };
    let disabled = !enabled;
    rsx! { button {name:{DynName(name.into())},onclick:action,disabled,button_default_skin:false,
        width:{188.0*scale},height:{28.0*scale},pos_type:"absolute",left:{6.0*scale},top:{(6.0+index as f32*28.0)*scale},
        fontstring {name:{DynName(format!("{name}Text"))},text,width:{188.0*scale},height:{28.0*scale},
            font_size:{12.0*scale},font_color:color,justify_h:"LEFT",mouse_enabled:false}
    } }
}

pub(super) fn loadout_dialog(view: &TalentView, scale: f32) -> Element {
    let Some(dialog) = &view.editor.loadout_dialog else {
        return Vec::new();
    };
    let (title, height, confirm) = match dialog {
        LoadoutDialog::Create => ("New Loadout", 150.0, false),
        LoadoutDialog::Edit(_) => ("Edit Loadout", 200.0, false),
        LoadoutDialog::Delete(_) => ("Delete Loadout?", 150.0, true),
        LoadoutDialog::Switch(_) => ("Discard pending talent changes?", 150.0, true),
    };
    let mut contents = dialog_border(height, scale);
    contents.extend(label(
        Label {
            name: "TalentLoadoutDialogTitle".into(),
            text: title,
            rect: [40.0, 20.0, 380.0, 20.0],
            size: 14.0,
            color: TAB_TEXT,
            justify: "CENTER",
        },
        scale,
    ));
    if confirm {
        contents.extend(label(
            Label {
                name: "TalentLoadoutConfirmationText".into(),
                text: if matches!(dialog, LoadoutDialog::Delete(_)) {
                    &view.editor.loadout_name
                } else {
                    "Your unapplied changes will be lost."
                },
                rect: [40.0, 50.0, 380.0, 32.0],
                size: 12.0,
                color: TAB_TEXT,
                justify: "CENTER",
            },
            scale,
        ));
    } else {
        contents.extend(label(
            Label {
                name: "TalentLoadoutNameLabel".into(),
                text: "Loadout Name",
                rect: [40.0, 40.0, 380.0, 16.0],
                size: 12.0,
                color: TAB_TEXT,
                justify: "LEFT",
            },
            scale,
        ));
        contents.extend(rsx! { editbox {name:"TalentLoadoutNameInput",text:{view.editor.loadout_name.as_str()},max_letters:30,
            width:{380.0*scale},height:{32.0*scale},font_size:{12.0*scale},text_insets:"6,6,0,0",
            background_color:"0.04,0.04,0.04,1.0",pos_type:"absolute",left:{40.0*scale},top:{56.0*scale}} });
    }
    let enabled = confirm || !view.editor.loadout_name.trim().is_empty();
    let is_edit = matches!(dialog, LoadoutDialog::Edit(_));
    if is_edit {
        contents.extend(dialog_button(
            "TalentLoadoutDelete",
            "Delete",
            "talent:loadout_delete",
            true,
            [170.0, height - 47.0],
            scale,
        ));
    }
    let save_x = if is_edit { 40.0 } else { 105.0 };
    let cancel_x = if is_edit { 300.0 } else { 235.0 };
    contents.extend(dialog_button(
        "TalentLoadoutSave",
        if matches!(dialog, LoadoutDialog::Delete(_)) {
            "Delete"
        } else if confirm {
            "Confirm"
        } else if is_edit {
            "Accept"
        } else {
            "Save"
        },
        if confirm {
            "talent:loadout_confirm"
        } else {
            "talent:loadout_save"
        },
        enabled,
        [save_x, height - 47.0],
        scale,
    ));
    contents.extend(dialog_button(
        "TalentLoadoutCancel",
        "Cancel",
        "talent:loadout_cancel",
        true,
        [cancel_x, height - 47.0],
        scale,
    ));
    if let Some(error) = &view.editor.error_text {
        contents.extend(label(
            Label {
                name: "TalentLoadoutNameError".into(),
                text: error,
                rect: [40.0, 90.0, 380.0, 24.0],
                size: 12.0,
                color: "1.0,0.2,0.2,1.0",
                justify: "LEFT",
            },
            scale,
        ));
    }
    rsx! { r#frame {name:"TalentLoadoutDialogBlocker",width:{BOOK_W*scale},height:{BOOK_H*scale},
        strata:FrameStrata::Dialog,frame_level:2090,pos_type:"absolute",left:0.0,top:0.0}
        r#frame {name:"TalentLoadoutDialog",width:{460.0*scale},height:{height*scale},
            strata:FrameStrata::Dialog,frame_level:2100,pos_type:"absolute",left:{(BOOK_W-460.0)/2.0*scale},top:{(BOOK_H-height)/2.0*scale},
            texture {name:"TalentLoadoutDialogBackground",texture_fdid:312922,width:{446.0*scale},height:{(height-14.0)*scale},
                pos_type:"absolute",left:{7.0*scale},top:{7.0*scale}} {contents}
        }
    }
}
fn dialog_button(
    name: &str,
    text: &str,
    action: &str,
    enabled: bool,
    at: [f32; 2],
    scale: f32,
) -> Element {
    crate::ui::screens::quest_art::panel_button(
        name.into(),
        text,
        action,
        enabled,
        (at[0] * scale, at[1] * scale, 120.0 * scale, 22.0 * scale),
    )
}
fn dialog_border(height: f32, scale: f32) -> Element {
    let side_height = (height - 128.0).abs();
    [
        (
            "TopLeft",
            "UI-Frame-DiamondMetal-CornerTopLeft",
            [0.0, 0.0, 64.0, 64.0],
        ),
        (
            "TopRight",
            "UI-Frame-DiamondMetal-CornerTopRight",
            [396.0, 0.0, 64.0, 64.0],
        ),
        (
            "BottomLeft",
            "UI-Frame-DiamondMetal-CornerBottomLeft",
            [0.0, height - 64.0, 64.0, 64.0],
        ),
        (
            "BottomRight",
            "UI-Frame-DiamondMetal-CornerBottomRight",
            [396.0, height - 64.0, 64.0, 64.0],
        ),
        (
            "Top",
            "_UI-Frame-DiamondMetal-EdgeTop",
            [64.0, 0.0, 332.0, 64.0],
        ),
        (
            "Bottom",
            "_UI-Frame-DiamondMetal-EdgeBottom",
            [64.0, height - 64.0, 332.0, 64.0],
        ),
        (
            "Left",
            "!UI-Frame-DiamondMetal-EdgeLeft",
            [0.0, 64.0, 64.0, side_height],
        ),
        (
            "Right",
            "!UI-Frame-DiamondMetal-EdgeRight",
            [396.0, 64.0, 64.0, side_height],
        ),
    ]
    .into_iter()
    .flat_map(|(name, atlas, rect)| {
        retail_atlas(
            &format!("TalentLoadoutDialogBorder{name}"),
            atlas,
            rect,
            scale,
        )
    })
    .collect()
}
