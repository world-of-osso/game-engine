//! Retail outfit row context menu and IconSelectorPopupFrameTemplate (525×495).
use super::*;

pub(super) const EDITBOX: &str = "TransmogOutfitIconName";
const ICONS_PER_PAGE: usize = 80;

pub(super) fn outfit_popups(book: &TransmogBook) -> Element {
    let mut out = Vec::new();
    if let Some(id) = book.outfit_menu {
        let index = book
            .outfit_rows()
            .iter()
            .position(|row| row.id == id)
            .expect("outfit menu entry");
        let action = format!("transmog:edit-outfit:{id}");
        let row = skinned_button(
            "TransmogOutfitEditMenu",
            "Edit Outfit",
            &action,
            true,
            (0.0, 0.0, 180.0, 28.0),
            &TERTIARY,
            Vec::new(),
        );
        out.extend(rsx! { r#frame { name: "TransmogOutfitContextMenu", width: 180.0, height: 28.0,
        left: 65.0, top: {177.0 + index as f32 * 47.0}, pos_type: "absolute", mouse_enabled: true,
        strata: FrameStrata::FullscreenDialog, frame_level: 500.0, {row} } });
    }
    let Some(draft) = &book.outfit_info else {
        return out;
    };
    let mut children = rsx! { r#frame { name: "TransmogOutfitPopupBackground", width: 511.0,
    height: 481.0, left: 7.0, top: 7.0, pos_type: "absolute", background_color: "0,0,0,0.8" } };
    children.extend(label(
        "TransmogOutfitPopupNameLabel".into(),
        "Outfit Name",
        (24.0, 21.0, 182.0, 18.0),
        (12.0, WHITE, "LEFT"),
    ));
    children.extend(
        rsx! { editbox { name: {DynName(EDITBOX.into())}, width: 182.0, height: 20.0,
        left: 29.0, top: 35.0, pos_type: "absolute", max_letters: 16, font_size: 13.0,
        text: {draft.name.as_str()}, font_color: WHITE, frame_level: 52.0 } },
    );
    children.extend(label(
        "TransmogOutfitPopupIconLabel".into(),
        "Choose an Icon",
        (24.0, 79.0, 220.0, 18.0),
        (12.0, WHITE, "LEFT"),
    ));
    children.extend(label(
        "TransmogOutfitPopupCurrentLabel".into(),
        "Current Icon",
        (280.0, 23.0, 170.0, 18.0),
        (12.0, GOLD, "RIGHT"),
    ));
    if draft.icon_fdid != 0 {
        let icon = draft.icon_fdid;
        children.extend(
            rsx! { texture { name: "TransmogOutfitSelectedIcon", width: 36.0, height: 36.0,
            left: 471.5, top: 16.5, pos_type: "absolute", texture_fdid: icon } },
        );
    }
    let icons = book.outfit_icons();
    for (index, icon) in icons
        .iter()
        .skip(draft.page * ICONS_PER_PAGE)
        .take(ICONS_PER_PAGE)
        .enumerate()
    {
        let action = format!("transmog:outfit-icon:{icon}");
        let art = rsx! { texture { name: {DynName(format!("TransmogIcon{icon}Texture"))}, width: 36.0,
        height: 36.0, left: 2.0, top: 2.0, pos_type: "absolute", texture_fdid: {*icon} } };
        let border = if *icon == draft.icon_fdid {
            "common-button-square-gray-down"
        } else {
            "common-button-square-gray-up"
        };
        children.extend(skinned_button(
            &format!("TransmogIcon{icon}"),
            "",
            &action,
            true,
            (
                21.0 + (index % 10) as f32 * 47.0,
                97.0 + (index / 10) as f32 * 44.0,
                40.0,
                40.0,
            ),
            &Skin::States {
                up: border,
                pressed: "common-button-square-gray-down",
                disabled: border,
                highlight: "common-button-square-gray-up",
            },
            art,
        ));
    }
    for (name, caption, action, x, enabled) in [
        (
            "TransmogOutfitIconOkay",
            "Okay",
            "transmog:icon-okay",
            248.0,
            !draft.name.trim().is_empty(),
        ),
        (
            "TransmogOutfitIconCancel",
            "Cancel",
            "transmog:icon-cancel",
            386.0,
            true,
        ),
        (
            "TransmogOutfitIconPrevious",
            "<",
            "transmog:icon-previous",
            21.0,
            draft.page > 0,
        ),
        (
            "TransmogOutfitIconNext",
            ">",
            "transmog:icon-next",
            86.0,
            (draft.page + 1) * ICONS_PER_PAGE < icons.len(),
        ),
    ] {
        children.extend(skinned_button(
            name,
            caption,
            action,
            enabled,
            (x, 465.0, if x < 100.0 { 55.0 } else { 128.0 }, 24.0),
            &RED_BUTTON,
            Vec::new(),
        ));
    }
    out.extend(
        rsx! { r#frame { name: "TransmogOutfitIconSelector", width: 525.0, height: 495.0,
        left: {(WIDTH - 525.0) * 0.5}, top: 60.0, pos_type: "absolute", mouse_enabled: true,
        strata: FrameStrata::FullscreenDialog, frame_level: 500.0, {children} } },
    );
    out
}
