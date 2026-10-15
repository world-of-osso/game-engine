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
        let top = (8.0 + index as f32 * 20.0) * scale;
        let selected = list.is_some_and(|list| list.selected_id == config.id);
        let check = if selected {
            rsx! { texture { name:{DynName(format!("{name}Check"))},texture_fdid:130751,
            width:{20.0*scale},height:{20.0*scale},pos_type:"absolute",left:{(8.0 + ui_toolkit::text_measure::measure_text(&config.name, GameFont::FrizQuadrata, 12.0).expect("loadout font loaded").0)*scale},top:0.0 } }
        } else {
            Vec::new()
        };
        let content = rsx! {
            fontstring {name:{DynName(format!("{name}Text"))},text:{config.name.as_str()},width:{136.0*scale},height:{20.0*scale},
                font_size:{12.0*scale},font_color:TAB_TEXT,justify_h:"LEFT",mouse_enabled:false} {check}
        };
        rows.extend(crate::selection_list::row(
            name.clone(),
            &action,
            enabled,
            (8.0 * scale, top, 164.0 * scale, 20.0 * scale),
            (&crate::selection_list::MENU, false),
            content,
        ));
        rows.extend(rsx! { button {name:{DynName(format!("TalentLoadoutEdit{}",config.id))},onclick:{edit.as_str()},disabled,button_default_skin:false,
            width:{16.0*scale},height:{16.0*scale},pos_type:"absolute",left:{176.0*scale},top:{top+2.0*scale},
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
        (
            1,
            "TalentImportLoadout",
            "Import Loadout",
            "talent:loadout_import",
            enabled && list.is_some(),
        ),
        (
            2,
            "TalentExportLoadout",
            "Copy to Clipboard",
            "talent:loadout_export",
            enabled && view.editor.snapshot.is_some(),
        ),
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
    let height = 8.0 + (configs.len() + 3) as f32 * 20.0 + 15.0;
    menu(
        "TalentLoadoutMenu",
        [48.0, y + 2.5 - height, 200.0, height],
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
    menu_row(name, text, action, enabled, index, 200.0, scale)
}

pub(super) fn loadout_dialog(view: &TalentView, viewport: [f32; 2]) -> Element {
    // StaticPopup_SetUpPosition: special dialogs are UIParent children, independent of book fitting.
    let scale = 1.0;
    let Some(dialog) = &view.editor.loadout_dialog else {
        return Vec::new();
    };
    let (title, height, confirm) = match dialog {
        LoadoutDialog::Create => ("New Loadout", 150.0, false),
        LoadoutDialog::Import => ("Import Loadout", 275.0, false),
        LoadoutDialog::Edit(_) => ("Edit Loadout", 200.0, false),
        LoadoutDialog::Delete(_) => ("Delete Loadout?", 150.0, true),
        LoadoutDialog::Switch(_) => ("Discard pending talent changes?", 150.0, true),
    };
    let importing = matches!(dialog, LoadoutDialog::Import);
    let mut contents = dialog_border(height, scale);
    if importing {
        contents.extend(import_control(&view.editor.loadout_code));
    }
    contents.extend(label(
        Label {
            name: "TalentLoadoutDialogTitle".into(),
            text: title,
            rect: [105.0, 20.0, 250.0, 12.0],
            size: 12.0,
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
                rect: [40.0, if importing { 165.0 } else { 40.0 }, 380.0, 16.0],
                size: 12.0,
                color: TAB_TEXT,
                justify: "LEFT",
            },
            scale,
        ));
        let mut input = crate::bank_art::edit_box(
            "TalentLoadoutNameInput",
            (
                40.0 * scale,
                if importing { 183.0 } else { 58.0 } * scale,
                380.0 * scale,
                32.0 * scale,
            ),
        );
        // InputBoxTemplate: source NameControl bottom90, 32px input; letters30.
        let input_widget = input
            .iter_mut()
            .find_map(|child| match child {
                ui_toolkit::widget_def::WidgetChild::Widget(widget)
                    if widget.name.as_deref() == Some("TalentLoadoutNameInput") =>
                {
                    Some(widget)
                }
                _ => None,
            })
            .expect("InputBoxTemplate includes named input");
        input_widget
            .attrs
            .push(ui_toolkit::widget_def::Attr::new_dynamic(
                "text",
                view.editor.loadout_name.clone(),
            ));
        input_widget
            .attrs
            .push(ui_toolkit::widget_def::Attr::new_static(
                "max_letters",
                "30".into(),
            ));
        contents.extend(input);
    }
    let enabled = confirm
        || (!view.editor.loadout_name.trim().is_empty()
            && (!importing || !view.editor.loadout_code.is_empty()));
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
        } else if importing {
            "Import"
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
    if let Some(error) = view.editor.error_text.as_ref().filter(|_| !importing) {
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
    let error_popup = if importing {
        import_error(view.editor.error_text.as_deref(), viewport)
    } else {
        Vec::new()
    };
    rsx! { r#frame {name:"TalentLoadoutDialogBlocker",width:{viewport[0]},height:{viewport[1]},
        strata:FrameStrata::Dialog,frame_level:2090,pos_type:"absolute",left:0.0,top:0.0}
        r#frame {name:"TalentLoadoutDialog",width:{460.0*scale},height:{height*scale},
            strata:FrameStrata::Dialog,frame_level:2100,pos_type:"absolute",left:{(viewport[0]-460.0)/2.0},top:{if importing { (viewport[1]-height)/2.0 } else { 135.0 } },
            texture {name:"TalentLoadoutDialogBackground",texture_fdid:312922,width:{446.0*scale},height:{(height-14.0)*scale},horiz_tile:true,vert_tile:true,
                pos_type:"absolute",left:{7.0*scale},top:{7.0*scale}} {contents}
        }
        {error_popup}
    }
}

fn import_control(text: &str) -> Element {
    // InputScrollFrameTemplate: eight8px Common-Input-Border pieces,5px outside text.
    let textures = ["TL", "T", "TR", "L", "M", "R", "BL", "B", "BR"]
        .map(|part| format!("data/ui/Common-Input-Border-{part}.blp"));
    let mut elements = label(
        Label {
            name: "TalentLoadoutCodeLabel".into(),
            text: "Paste loadout code",
            rect: [40.0, 40.0, 380.0, 12.0],
            size: 12.0,
            color: TAB_TEXT,
            justify: "LEFT",
        },
        1.0,
    );
    elements.extend(rsx! { editbox {name:"TalentLoadoutCodeInput",text:text,width:390.0,height:88.0,
        pos_type:"absolute",left:35.0,top:57.0,font_size:13.0,text_insets:"5,5,5,5",max_letters:1000,
        nine_slice {edge_size:8,textures:textures}
    } });
    elements
}

fn import_error(error: Option<&str>, viewport: [f32; 2]) -> Element {
    let Some(error) = error else {
        return Vec::new();
    };
    // Shared StaticPopup geometry (static_popup_component):320 width,290 text,
    // 16px padding,18px lines,12px gap,128x26 button; first popup TOP135.
    let lines = (error.chars().count() as f32 / (290.0 / 7.0))
        .ceil()
        .max(1.0);
    let text_height = lines * 18.0;
    let height = 70.0 + text_height;
    rsx! { r#frame {name:"TalentImportErrorBlocker",width:{viewport[0]},height:{viewport[1]},strata:FrameStrata::FullscreenDialog,pos_type:"absolute",left:0.0,top:0.0}
        r#frame {name:"TalentImportError",width:320.0,height:height,pos_type:"absolute",left:{(viewport[0]-320.0)/2.0},top:135.0,
            strata:FrameStrata::FullscreenDialog,frame_level:2200,
            style:crate::static_popup_component::STATIC_POPUP_PANEL_STYLE,
            texture {name:"TalentImportErrorBackground",texture_fdid:6839810,width:306.0,height:{height-14.0},pos_type:"absolute",left:7.0,top:7.0}
            fontstring {name:"TalentImportErrorText",text:error,width:290.0,height:text_height,pos_type:"absolute",left:15.0,top:16.0,font_size:14.0,font_color:"1.0,0.2,0.2,1.0",justify_h:"CENTER"}
            button {name:"TalentImportErrorOkay",text:"Okay",onclick:"talent:loadout_error_ok",width:128.0,height:26.0,pos_type:"absolute",left:96.0,top:{28.0+text_height},
                button_default_skin:false,button_atlas_up:"defaultbutton-nineslice-up",button_atlas_pressed:"defaultbutton-nineslice-pressed",button_atlas_highlight:"defaultbutton-nineslice-highlight"}
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
