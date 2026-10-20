//! GameDialog.xml / GameDialog.lua confirmation: optional ItemFrame and currency markup.
#[path = "static_popup_prompt.rs"]
mod prompt_layout;
use crate::bank_art::{WHITE, label, texture};
use crate::quest_art::DynName;
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub struct ItemConfirmation<'a> {
    pub name: &'a str,
    pub pending: bool,
    /// Retail acceptDelay disables only Confirm, leaving Cancel available.
    pub accept_enabled: bool,
    /// The two text runs around the source currency texture markup.
    pub prompt: (&'a str, &'a str),
    pub currency_icon: Option<u32>,
    /// GameDialog's hasItemFrame flag; cart/clear confirmations have no item frame.
    pub item: Option<(&'a str, u32, u8)>,
    pub accept: (&'a str, &'a str),
    pub cancel_action: Option<&'a str>,
    pub screen: (f32, f32),
}

const WIDTH: f32 = 320.;
const DIALOG_LEVEL: f32 = 2002.;
const PENDING_INSET: f32 = 11.;
const TEXT_SIZE: f32 = 12.;
const ITEM_WIDTH: f32 = 37. + 10. + 103. - 4.;
const ITEM_HEIGHT: f32 = 38.;
const BUTTON_WIDTH: f32 = 128.;
const BUTTON_HEIGHT: f32 = 21.;
const BUTTON_GAP: f32 = 10.;
// GameDialog.lua561–584: the visible caption plus20px, at least120px.
const SINGLE_BUTTON_MIN_WIDTH: f32 = 120.;
const BUTTON_TEXT_MARGIN: f32 = 20.;

pub fn item_confirmation(props: ItemConfirmation<'_>) -> Element {
    let (contents, height) = confirmation_contents(&props);
    let prefix = props.name;
    let (screen_width, screen_height) = props.screen;
    // GameDialogCoverFrame: authentic half-black cover, not a substitute for dialog art.
    rsx! {
        r#frame { name:{DynName(format!("{prefix}Cover"))}, left:0.0, top:0.0,
            width:screen_width,height:screen_height,pos_type:"absolute",mouse_enabled:true,
            background_color:"0,0,0,0.5",strata:FrameStrata::FullscreenDialog,frame_level:{DIALOG_LEVEL-1.} }
        r#frame { name:{DynName(format!("{prefix}Dialog"))},left:{(screen_width-WIDTH)/2.},top:135.0,
            width:WIDTH,height,pos_type:"absolute",mouse_enabled:true,
            strata:FrameStrata::FullscreenDialog,frame_level:DIALOG_LEVEL,{contents} }
    }
}

fn confirmation_contents(props: &ItemConfirmation<'_>) -> (Element, f32) {
    let (prompt, text_height) = confirmation_prompt(props);
    // GameDialog.lua: Text→ItemFrame -5, ItemFrame→Buttons -18; XML padding16.
    let item_y = 16. + text_height + 5.;
    let buttons_y = if props.item.is_some() {
        item_y + ITEM_HEIGHT + 18.
    } else {
        // GameDialog.lua GetSpacing_ButtonsAfterText: one/two buttons use9px.
        16. + text_height + 9.
    };
    let height = buttons_y + BUTTON_HEIGHT + 16.;
    let mut contents = confirmation_background(props.name, height);
    contents.extend(prompt);
    if let Some(item) = props.item {
        contents.extend(item_frame(props.name, item, item_y));
    }
    contents.extend(confirmation_buttons(props, buttons_y));
    if props.pending {
        let prefix = props.name;
        let ring = crate::quest_art::named_atlas_texture(
            format!("{prefix}SpinnerRing"),
            "Spinner_Ring",
            (0., 0., 60., 60.),
        );
        let sparks = crate::quest_art::named_atlas_texture(
            format!("{prefix}SpinnerSparks"),
            "Spinner_Sparks",
            (0., 0., 60., 60.),
        );
        contents.extend(rsx! {
            // GameDialog.xml296–320: source500/1000 priorities above dialog content.
            r#frame{name:{DynName(format!("{prefix}DarkOverlay"))},left:PENDING_INSET,top:PENDING_INSET,width:{WIDTH-2.*PENDING_INSET},height:{height-2.*PENDING_INSET},pos_type:"absolute",frame_level:{DIALOG_LEVEL+500.},mouse_enabled:true,background_color:"0,0,0,0.8"}
            r#frame{name:{DynName(format!("{prefix}Spinner"))},left:{(WIDTH-60.)/2.},top:{(height-60.)/2.},width:60.0,height:60.0,pos_type:"absolute",frame_level:{DIALOG_LEVEL+1000.},{ring}{sparks}}
        });
    }
    (contents, height)
}

fn confirmation_background(prefix: &str, height: f32) -> Element {
    rsx! {
        texture {
            name:{DynName(format!("{prefix}Background"))}, texture_fdid:6839810,
            left:7.0,top:7.0,width:{WIDTH-14.},height:{height-14.},
            pos_type:"absolute",draw_layer:"BACKGROUND",
        }
        r#frame {
            name:{DynName(format!("{prefix}Border"))}, width:WIDTH, height, left:0.0, top:0.0,
            pos_type:"absolute", style:"static_popup",draw_layer:"BACKGROUND",
        }
    }
}

fn confirmation_prompt(props: &ItemConfirmation<'_>) -> (Element, f32) {
    let Some(currency_icon) = props.currency_icon else {
        return plain_confirmation_prompt(props);
    };
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let (before_width, text_height) =
        measure_text(props.prompt.0, GameFont::FrizQuadrata, TEXT_SIZE).expect("dialog font");
    let (after_width, _) =
        measure_text(props.prompt.1, GameFont::FrizQuadrata, TEXT_SIZE).expect("dialog font");
    let prompt_width = before_width + 4. + 16. + 4. + after_width;
    if prompt_width > 290. {
        return prompt_layout::wrapped_currency_prompt(props.name, props.prompt, currency_icon);
    }
    let prompt_x = (WIDTH - prompt_width) / 2.;
    let prefix = props.name;
    let mut contents = label(
        format!("{prefix}TextBefore"),
        props.prompt.0,
        (prompt_x, 16., before_width, text_height),
        (TEXT_SIZE, WHITE, "LEFT"),
    );
    contents.extend(texture(
        format!("{prefix}CurrencyIcon"),
        currency_icon,
        (
            prompt_x + before_width + 4.,
            16. + (text_height - 16.) / 2.,
            16.,
            16.,
        ),
        WHITE,
    ));
    contents.extend(label(
        format!("{prefix}TextAfter"),
        props.prompt.1,
        (prompt_x + before_width + 24., 16., after_width, text_height),
        (TEXT_SIZE, WHITE, "LEFT"),
    ));
    (contents, text_height)
}

fn plain_confirmation_prompt(props: &ItemConfirmation<'_>) -> (Element, f32) {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let text = format!("{}{}", props.prompt.0, props.prompt.1);
    let line_height = measure_text(&text, GameFont::FrizQuadrata, TEXT_SIZE)
        .expect("dialog font")
        .1;
    let text_height =
        crate::quest_art::wrapped_line_count(&text, 290., TEXT_SIZE) as f32 * line_height;
    let contents = label(
        format!("{}TextBefore", props.name),
        &text,
        (15., 16., 290., text_height),
        (TEXT_SIZE, WHITE, "CENTER"),
    );
    (contents, text_height)
}

fn confirmation_buttons(props: &ItemConfirmation<'_>, top: f32) -> Element {
    let Some(cancel_action) = props.cancel_action else {
        return single_confirmation_button(props, top);
    };
    let prefix = props.name;
    let button_x = (WIDTH - BUTTON_WIDTH * 2. - BUTTON_GAP) / 2.;
    let mut contents = dialog_button(
        &format!("{prefix}Button"),
        props.accept.0,
        props.accept.1,
        props.pending || !props.accept_enabled,
        (button_x, top, BUTTON_WIDTH),
    );
    contents.extend(dialog_button(
        &format!("{prefix}CancelButton"),
        "Cancel",
        cancel_action,
        props.pending,
        (button_x + BUTTON_WIDTH + BUTTON_GAP, top, BUTTON_WIDTH),
    ));
    contents
}

fn single_confirmation_button(props: &ItemConfirmation<'_>, top: f32) -> Element {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let caption_width = measure_text(props.accept.0, GameFont::FrizQuadrata, TEXT_SIZE)
        .expect("dialog font")
        .0;
    let width = SINGLE_BUTTON_MIN_WIDTH.max(caption_width + BUTTON_TEXT_MARGIN);
    dialog_button(
        &format!("{}Button", props.name),
        props.accept.0,
        props.accept.1,
        props.pending || !props.accept_enabled,
        ((WIDTH - width) / 2., top, width),
    )
}

fn item_frame(prefix: &str, (name, icon, quality): (&str, u32, u8), top: f32) -> Element {
    let mut contents = texture(
        format!("{prefix}ItemNameFrame"),
        136796,
        (26., -12., 137., 62.),
        WHITE,
    );
    contents.extend(label(
        format!("{prefix}ItemText"),
        name,
        (43., 0., 103., 38.),
        (12., crate::merchant_data::quality_color(quality), "LEFT"),
    ));
    contents.extend(texture(
        format!("{prefix}Item"),
        icon,
        (-4., 0.5, 37., 37.),
        WHITE,
    ));
    rsx! { r#frame { name:{DynName(format!("{prefix}ItemFrame"))},left:{(WIDTH-ITEM_WIDTH)/2.},top,
    width:ITEM_WIDTH,height:ITEM_HEIGHT,pos_type:"absolute",{contents} } }
}

pub(crate) fn dialog_button(
    name: &str,
    text: &str,
    action: &str,
    disabled: bool,
    (left, top, width): (f32, f32, f32),
) -> Element {
    let contents = rsx! {
        texture { name:{DynName(format!("{name}NormalArt"))},texture_fdid:130763,
            width,height:BUTTON_HEIGHT,pos_type:"absolute",left:0.0,top:0.0,
            tex_coords:"0,1,0,0.71875",draw_layer:"BACKGROUND" }
        texture { name:{DynName(format!("{name}PushedArt"))},texture_fdid:130761,
            width,height:BUTTON_HEIGHT,pos_type:"absolute",left:0.0,top:0.0,
            tex_coords:"0,1,0,0.71875",hidden:true,draw_layer:"BACKGROUND" }
        texture { name:{DynName(format!("{name}DisabledArt"))},texture_fdid:130760,
            width,height:BUTTON_HEIGHT,pos_type:"absolute",left:0.0,top:0.0,
            tex_coords:"0,1,0,0.71875",hidden:true,draw_layer:"BACKGROUND" }
        texture { name:{DynName(format!("{name}Highlight"))},texture_fdid:130762,
            width,height:BUTTON_HEIGHT,pos_type:"absolute",left:0.0,top:0.0,
            tex_coords:"0,1,0,0.71875",blend_mode:"ADD",draw_layer:"HIGHLIGHT" }
        fontstring { name:{DynName(format!("{name}DialogButtonText"))},text,font_size:12.0,
            font_color:"1,0.82,0,1",shadow_color:"0,0,0,1",shadow_offset:"1,-1",
            justify_h:"CENTER",width,height:BUTTON_HEIGHT,
            pos_type:"absolute",left:0.0,top:-1.0 }
    };
    rsx! { button { name:{DynName(name.into())},width,height:BUTTON_HEIGHT,
    pos_type:"absolute",left,top,button_default_skin:false,onclick:action,disabled,{contents} } }
}
