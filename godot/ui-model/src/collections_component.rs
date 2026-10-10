//! Retail Mainline CollectionsJournal: one portrait shell and shared tab bar.
use crate::merchant_frame_component::tab_width;
use crate::quest_art::{
    DynName, named_atlas_texture, portrait_border, window_portrait_slot, window_portrait_texture,
};
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub const SIZE: [f32; 2] = [703.0, 606.0];
pub const TABS: [&str; 6] = [
    "Mounts",
    "Pets",
    "Toy Box",
    "Heirlooms",
    "Appearances",
    "Warband Scenes",
];

pub fn collections_shell(viewport: [f32; 2], selected: usize, body: Element) -> Element {
    let title = if selected == 1 {
        "Pet Journal"
    } else {
        TABS[selected]
    };
    let chrome = portrait_border(
        "CollectionsJournal",
        (SIZE[0], SIZE[1]),
        title,
        "collections:close",
    );
    let portrait =
        window_portrait_texture(&window_portrait_slot("CollectionsJournalPortrait"), 454_046);
    let tabs = tabs(selected);
    let left = (viewport[0] - SIZE[0]) / 2.0;
    let top = (viewport[1] - SIZE[1]) / 2.0;
    rsx! {
        r#frame { name: "CollectionsRoot", stretch: true,
            r#frame { name: "CollectionsJournal", width: 703.0, height: 606.0,
                mouse_enabled: true, pos_type: "absolute", left, top, strata: FrameStrata::Dialog,
                background_color: "0.08,0.07,0.06,1.0",
                {chrome} {portrait} {body} {tabs}
            }
        }
    }
}

fn tabs(selected: usize) -> Element {
    let mut left = 11.0;
    let mut tabs: Vec<_> = TABS
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            let width = tab_width(title);
            let tab = journal_tab(index, selected, title, left, width);
            // Retail AnchorTabs uses a +3px gap after SetNumTabs.
            left += width + 3.0;
            tab
        })
        .collect();
    let active = tabs.remove(selected);
    tabs.into_iter().flatten().chain(active).collect()
}

fn journal_tab(index: usize, active: usize, title: &str, left: f32, width: f32) -> Element {
    let selected = index == active;
    let name = format!("CollectionsJournalTab{}", index + 1);
    let prefix = if selected {
        "uiframe-activetab"
    } else {
        "uiframe-tab"
    };
    let height = if selected { 42.0 } else { 36.0 };
    let offset = if selected { -1.0 } else { -3.0 };
    let right = width + if selected { 8.0 } else { 7.0 } - 37.0;
    let mut art = named_atlas_texture(
        format!("{name}Left"),
        &format!("{prefix}-left"),
        (offset, 0.0, 35.0, height),
    );
    art.extend(named_atlas_texture(
        format!("{name}Middle"),
        &format!("_{prefix}-center"),
        (offset + 35.0, 0.0, right - offset - 35.0, height),
    ));
    art.extend(named_atlas_texture(
        format!("{name}Right"),
        &format!("{prefix}-right"),
        (right, 0.0, 37.0, height),
    ));
    let enabled = matches!(index, 1 | 2);
    let color = if selected {
        "1.0,1.0,1.0,1.0"
    } else if enabled {
        "1.0,0.82,0.0,1.0"
    } else {
        "0.5,0.5,0.5,1.0"
    };
    let action = if index == 1 {
        "collections:pets"
    } else {
        "collections:toys"
    };
    rsx! { button { name: {DynName(name.clone())}, width, height: 32.0, pos_type: "absolute", left, top: 604.0,
        button_default_skin: false, disabled: {!enabled}, onclick: action, {art}
        fontstring { name: {DynName(format!("{name}Text"))}, text: title, width, height: 32.0,
            pos_type: "absolute", left: 0.0, top: {if selected { 3.0 } else { -2.0 }},
            font_size: 10.0, font_color: color, justify_h: "CENTER" }
    } }
}
