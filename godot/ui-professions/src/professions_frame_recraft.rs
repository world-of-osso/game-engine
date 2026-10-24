//! Retail owned-item recraft slot. No synthetic fixed-output recraft submission.
use super::{ProfessionView, art, text};
use game_engine_ui_model::quest_art::panel_button;
use ui_toolkit::{rsx, widget_def::Element};
pub const RECRAFT_UNAVAILABLE: &str = "Recrafting unavailable: authored operation evaluator and owned-item lineage are not available. No item will be consumed.";
pub(super) fn schematic(view: &ProfessionView) -> Element {
    let mut out = art::background(&view.profession_name);
    out.extend(text(
        "ProfessionsRecraftHeading",
        "Recrafting",
        (309.0, 102.0, 400.0, 28.0),
    ));
    out.extend(art::atlas(
        "ProfessionsRecraftPanel",
        "Professions-Recrafting-Panel",
        (309.0, 151.0, 260.0, 105.0),
    ));
    let input = art::atlas(
        "ProfessionsRecraftInputArt",
        "itemupgrade_greenplusicon",
        (0.0, 0.0, 44.0, 44.0),
    );
    out.extend(rsx!{button{name:"ProfessionsRecraftInputSlot",left:326.0,top:168.0,width:44.0,height:44.0,pos_type:"absolute",disabled:true,button_default_skin:false,{input}}});
    out.extend(art::atlas(
        "ProfessionsRecraftArrow",
        "cyphersetupgrade-arrow-empty",
        (393.0, 181.0, 25.0, 25.0),
    ));
    out.extend(text(
        "ProfessionsRecraftStatus",
        RECRAFT_UNAVAILABLE,
        (309.0, 283.0, 600.0, 76.0),
    ));
    out.extend(panel_button(
        "ProfessionsRecraftSubmit".into(),
        "Recraft",
        "profession:recraft:submit",
        false,
        (853.0, 629.0, 80.0, 22.0),
    ));
    out
}
