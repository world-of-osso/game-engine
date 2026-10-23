//! Mainline GuildRegistrarFrame.xml and PetitionFrame.xml. Forever changes art only.
use crate::bank_art::label;
use crate::guild_charters::{GuildCharters, PURCHASE_NAME, RENAME_NAME};
use crate::quest_art::{
    DynName, named_atlas_texture, panel_button, window_chrome, window_portrait,
    window_portrait_slot, window_portrait_texture,
};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};
pub const PORTRAIT: crate::inworld_unit_frames_component::PortraitSlot =
    window_portrait_slot("GuildRegistrarFramePortrait");

const INK: &str = "0.15,0.09,0.04,1";
const WIDTH: f32 = 338.0;
const HEIGHT: f32 = 424.0;

pub fn charter_screen(shared: &SharedContext) -> Element {
    let Some(view) = shared.get::<GuildCharters>() else {
        return vec![];
    };
    let mut children = if view.registrar.is_some() {
        registrar(view)
    } else {
        petition(view)
    };
    if view.renaming {
        children.extend(rename_popup(view));
    }
    children
}
fn parchment(prefix: &str) -> Element {
    named_atlas_texture(
        format!("{prefix}Bg"),
        "QuestBG-Parchment",
        (7.0, 62.0, 299.0, 334.0),
    )
}
fn body(name: &str, text: &str, y: f32) -> Element {
    label(
        name.into(),
        text,
        (20.0, y, 278.0, 20.0),
        (13.0, INK, "LEFT"),
    )
}
fn input(name: &str, y: f32) -> Element {
    rsx! { editbox { name: {DynName(name.into())}, width: 230.0, height: 28.0, left: 20.0, top: y,
    pos_type: "absolute", mouse_enabled: true, text_insets: "4,4,0,0", font_size: 13.0,
    max_letters: 24, bg_color: "0.1,0.07,0.04,0.8" } }
}
fn window(name: &str, children: Element) -> Element {
    crate::panel_lifecycle::bind_close(
        rsx! { r#frame { name: {DynName(name.into())}, width: WIDTH, height: HEIGHT,
        pos_type: "absolute", left: 20.0, top: 100.0, mouse_enabled: true, strata: FrameStrata::Dialog,
        {children} } },
        "guild_charter",
    )
}
fn registrar(view: &GuildCharters) -> Element {
    let Some(registrar) = &view.registrar else {
        return vec![];
    };
    let mut children = window_chrome(
        "GuildRegistrarFrame",
        (WIDTH, HEIGHT),
        &registrar.npc_name,
        "charter:close",
    );
    children.extend(parchment("GuildRegistrarFrame"));
    children.extend(window_portrait(&PORTRAIT));
    children.extend(panel_button(
        "GuildRegistrarFrameCancelButton".into(),
        "Cancel",
        "charter:close",
        true,
        (257.0, 398.0, 75.0, 22.0),
    ));
    if view.purchasing {
        children.extend(body(
            "GuildRegistrarPurchaseText",
            "Choose a name for your guild.",
            80.0,
        ));
        children.extend(body("GuildRegistrarCostLabel", "Cost:", 118.0));
        children.extend(body(
            "GuildRegistrarMoneyFrame",
            &crate::bank_data::money_text(registrar.charter_cost),
            140.0,
        ));
        children.extend(input(PURCHASE_NAME, 166.0));
        children.extend(panel_button(
            "GuildRegistrarFramePurchaseButton".into(),
            "Purchase",
            "charter:purchase",
            !view.waiting,
            (6.0, 398.0, 85.0, 22.0),
        ));
    } else {
        children.extend(body("AvailableServicesText", "Available Services", 80.0));
        children.extend(panel_button(
            "GuildRegistrarButton1".into(),
            "Purchase a Guild Charter",
            "charter:purchase_open",
            !view.waiting,
            (20.0, 110.0, 278.0, 24.0),
        ));
        children.extend(panel_button(
            "GuildRegistrarButton2".into(),
            "Register a Guild Charter",
            "charter:turn_in",
            registrar.charter_guid.is_some() && !view.waiting,
            (20.0, 138.0, 278.0, 24.0),
        ));
    }
    append_error(view, &mut children);
    window("GuildRegistrarFrame", children)
}
fn petition(view: &GuildCharters) -> Element {
    let Some(petition) = &view.petition else {
        return vec![];
    };
    let title = format!("{} Guild Charter", petition.guild_name);
    let mut children = window_chrome("PetitionFrame", (WIDTH, HEIGHT), &title, "charter:close");
    children.extend(parchment("PetitionFrame"));
    children.extend(window_portrait_texture(
        &window_portrait_slot("PetitionFramePortrait"),
        136583,
    ));
    children.extend(body("PetitionFrameCharterTitle", "Guild Name", 80.0));
    children.extend(body(
        "PetitionFrameCharterName",
        &petition.guild_name,
        100.0,
    ));
    children.extend(body("PetitionFrameMasterTitle", "Guild Master", 125.0));
    children.extend(body("PetitionFrameMasterName", &petition.owner_name, 145.0));
    children.extend(body("PetitionFrameMemberTitle", "Members", 170.0));
    for index in 0..usize::from(petition.required_signatures) {
        children.extend(body(
            &format!("PetitionFrameMemberName{}", index + 1),
            petition
                .signatures
                .get(index)
                .map_or("Not yet signed", String::as_str),
            190.0 + index as f32 * 18.0,
        ));
    }
    let instructions = if petition.is_owner {
        "Target a player and request a signature."
    } else {
        "Sign this charter to help create the guild."
    };
    children.extend(body(
        "PetitionFrameInstructions",
        instructions,
        206.0 + f32::from(petition.required_signatures) * 18.0,
    ));
    children.extend(panel_button(
        "PetitionFrameCancelButton".into(),
        "Close",
        "charter:close",
        true,
        (257.0, 398.0, 75.0, 22.0),
    ));
    if petition.is_owner {
        children.extend(panel_button(
            "PetitionFrameRequestButton".into(),
            "Request Signature",
            "charter:offer",
            !view.waiting && petition.signatures.len() < usize::from(petition.required_signatures),
            (4.0, 398.0, 140.0, 22.0),
        ));
        children.extend(panel_button(
            "PetitionFrameRenameButton".into(),
            "Rename Guild",
            "charter:rename_open",
            !view.waiting,
            (141.0, 398.0, 112.0, 22.0),
        ));
    } else {
        children.extend(panel_button(
            "PetitionFrameSignButton".into(),
            "Sign Charter",
            "charter:sign",
            petition.can_sign && !view.waiting,
            (4.0, 398.0, 110.0, 22.0),
        ));
    }
    append_error(view, &mut children);
    window("PetitionFrame", children)
}
fn append_error(view: &GuildCharters, children: &mut Element) {
    if let Some(error) = &view.error {
        children.extend(label(
            "GuildCharterError".into(),
            error,
            (20.0, 350.0, 288.0, 40.0),
            (12.0, "0.8,0.1,0.1,1", "LEFT"),
        ));
    }
}
fn rename_popup(view: &GuildCharters) -> Element {
    let mut children = label(
        "GuildCharterRenamePrompt".into(),
        "Rename Guild",
        (20.0, 20.0, 280.0, 24.0),
        (13.0, "1,0.82,0,1", "CENTER"),
    );
    children.extend(input(RENAME_NAME, 55.0));
    children.extend(panel_button(
        "GuildCharterRenameAccept".into(),
        "Accept",
        "charter:rename",
        !view.waiting,
        (35.0, 108.0, 110.0, 22.0),
    ));
    children.extend(panel_button(
        "GuildCharterRenameCancel".into(),
        "Cancel",
        "charter:rename_cancel",
        true,
        (175.0, 108.0, 110.0, 22.0),
    ));
    rsx! { r#frame { name: {DynName("GuildCharterRenamePopup".into())}, width: 320.0, height: 150.0,
    left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", pos_type: "absolute",
    mouse_enabled: true, strata: FrameStrata::Dialog, frame_level: 500.0, style: "static_popup", {children} } }
}
