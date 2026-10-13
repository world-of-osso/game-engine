//! SettingsPanel.xml: GameTab(32,27), AddOnsTab(GameTab TOPRIGHT+5),
//! SearchBox350x22 at Container TOPRIGHT+(4,20) -> (552,34).
use super::{DynName, OptionsViewModel};
use crate::merchant_frame_component::{TabGeometry, TabLabel, tab, tab_width};
use crate::quest_art::named_atlas_texture;
use ui_toolkit::{rsx, widget_def::Element};

pub(super) fn settings_header(model: &OptionsViewModel) -> Element {
    let game_width = tab_width(TabLabel::Minimal("Game"));
    let tabs = [
        (
            "OptionsGameTab",
            "Game",
            32.0,
            game_width,
            !model.addons_tab,
            "options_tab:game",
        ),
        (
            "OptionsAddOnsTab",
            "AddOns",
            32.0 + game_width + 5.0,
            tab_width(TabLabel::Minimal("AddOns")),
            model.addons_tab,
            "options_tab:addons",
        ),
    ]
    .into_iter()
    .flat_map(|(name, label, left, width, selected, action)| {
        let control = tab(
            &format!("{name}Control"),
            label,
            TabGeometry::Minimal(0.0, 0.0, width),
            selected,
            action,
        );
        rsx! { r#frame { name: DynName(name.into()), width, height:37.0,
        pos_type:"absolute", left, top:27.0, {control} } }
    });
    tabs.chain(search_box(&model.search)).collect()
}

fn search_box(text: &str) -> Element {
    let border: Element = [
        (
            "Left",
            "common-search-border-left",
            (547.0, 35.0, 8.0, 20.0),
        ),
        (
            "Middle",
            "common-search-border-middle",
            (555.0, 35.0, 339.0, 20.0),
        ),
        (
            "Right",
            "common-search-border-right",
            (894.0, 35.0, 8.0, 20.0),
        ),
    ]
    .into_iter()
    .flat_map(|(part, atlas, rect)| {
        named_atlas_texture(format!("OptionsSearchBorder{part}"), atlas, rect)
    })
    .collect();
    let clear_hidden = text.is_empty();
    let hint_hidden = !text.is_empty();
    let icon = named_atlas_texture(
        "OptionsSearchIcon".into(),
        "common-search-magnifyingglass",
        (553.0, 41.0, 10.0, 10.0),
    );
    let clear = named_atlas_texture(
        "OptionsSearchClearIcon".into(),
        "common-search-clearbutton",
        (3.0, 3.0, 10.0, 10.0),
    );
    rsx! {
        {border}
        editbox {name:"OptionsSearchBox",text,width:350.0,height:22.0,pos_type:"absolute",left:552.0,top:34.0,
            text_insets:"16,20,0,0",max_letters:64,auto_focus:false,font_size:10.0}
        fontstring {name:"OptionsSearchHint",text:"Search",hidden:hint_hidden,mouse_enabled:false,
            width:314.0,height:22.0,left:568.0,top:34.0,pos_type:"absolute",font_size:10.0,color:"0.35,0.35,0.35,1",justify_h:"LEFT"}
        {icon}
        button {name:"OptionsSearchClear",width:17.0,height:17.0,left:882.0,top:36.5,pos_type:"absolute",
            hidden:clear_hidden,onclick:"options_search_clear",button_default_skin:false,{clear}}
    }
}
