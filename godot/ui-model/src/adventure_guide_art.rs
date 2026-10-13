//! Blizzard_EncounterJournal.xml texture regions and translated anchors.
//! UI-EJ-* are virtual XML textures on FDID522972, not DB2 atlas members.
use crate::adventure_guide::{Page, Tab, View};
use crate::bank_art::{WHITE, cropped, label};
use crate::quest_art::{
    DynName, named_atlas_texture, window_portrait_slot, window_portrait_texture,
};
use game_engine_core::adventure_guide::{Encounter, Instance};
use ui_toolkit::{rsx, widget_def::Element};

pub const INK: &str = "0.13,0.07,0.01,1.0";
/// QuestTitleFontBlackShadow/QuestFont_Super_Huge (GameFontStyles.xml/GameFonts.xml).
pub const GOLD: &str = "1.0,0.82,0.0,1.0";
pub const HEADER: &str = "0.902,0.788,0.671,1.0";
pub const SHEET: u32 = 522972;
/// XML Normal/Pushed/Highlight textures, driven by real registry input state.
pub fn apply_postsetup(view: &View, registry: &mut ui_toolkit::registry::FrameRegistry) {
    apply_button_art(view, registry);
    apply_text_layout(registry);
}
fn apply_text_layout(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::font_string::JustifyV};
    let styles: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_deref()?;
            let vertical = if name == "EncounterJournalInstanceTitle" {
                JustifyV::Bottom
            } else if name.starts_with("JournalBoss") && name.ends_with("Name") {
                JustifyV::Middle
            } else if (name.starts_with("JournalInstance") && name.ends_with("Title"))
                || name == "EncounterJournalInstanceLore"
                || name == "JournalEncounterLore"
                || (name.starts_with("JournalSection") && name.ends_with("Body"))
            {
                JustifyV::Top
            } else {
                return None;
            };
            Some((frame.id, vertical))
        })
        .collect();
    for (id, vertical) in styles {
        let Some(WidgetData::FontString(font)) =
            registry.get_mut(id).and_then(|f| f.widget_data.as_mut())
        else {
            continue;
        };
        font.word_wrap = true;
        font.justify_v = vertical;
    }
}
fn apply_button_art(view: &View, registry: &mut ui_toolkit::registry::FrameRegistry) {
    for instance in view.instances() {
        apply_button_state(
            registry,
            &format!("JournalInstance{}", instance.id),
            Art::DungeonUp,
            Art::DungeonDown,
        );
    }
    for encounter in &view.catalog.encounters {
        let normal = if view.encounter == Some(encounter.id) {
            Art::BossDown
        } else {
            Art::BossUp
        };
        apply_button_state(
            registry,
            &format!("JournalBoss{}", encounter.id),
            normal,
            Art::BossDown,
        );
    }
    for (index, page) in [
        (0, Page::Overview),
        (1, Page::Abilities),
        (2, Page::Loot),
        (3, Page::Model),
    ] {
        let normal = if view.page == page {
            Art::TabDown
        } else {
            Art::TabUp
        };
        apply_button_state(
            registry,
            &format!("EncounterJournalPage{index}"),
            normal,
            Art::TabDown,
        );
    }
}
fn apply_button_state(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    normal: Art,
    down: Art,
) {
    use ui_toolkit::{frame::WidgetData, widgets::button::ButtonState};
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    let Some(WidgetData::Button(button)) = registry.get(id).and_then(|f| f.widget_data.as_ref())
    else {
        return;
    };
    let hovered = button.hovered;
    let chosen = if button.state == ButtonState::Pushed {
        down
    } else {
        normal
    };
    if let Some(id) = registry.get_by_name(&format!("{name}ButtonHighlight")) {
        registry.set_hidden(id, !hovered);
        if let Some(WidgetData::Texture(texture)) =
            registry.get_mut(id).and_then(|f| f.widget_data.as_mut())
        {
            texture.blend_mode = ui_toolkit::widgets::texture::BlendMode::Additive;
        }
    }
    let Some(id) = registry.get_by_name(&format!("{name}ButtonArt")) else {
        return;
    };
    let Some(WidgetData::Texture(texture)) =
        registry.get_mut(id).and_then(|f| f.widget_data.as_mut())
    else {
        return;
    };
    let mut numbers = chosen.uv().split(',');
    texture.tex_coords = std::array::from_fn(|_| {
        numbers
            .next()
            .expect("four UV coordinates")
            .parse()
            .expect("XML UV number")
    });
}

pub const INFO: (f32, f32, f32, f32) = (7., 64., 785., 425.);
pub const MODEL: (f32, f32, f32, f32) = (399., 65., 390., 423.);

#[derive(Clone, Copy)]
pub enum Art {
    BossUp,
    BossDown,
    BossHighlight,
    DungeonUp,
    DungeonDown,
    DungeonHighlight,
    TabUp,
    TabDown,
    TabHighlight,
    BossIcon(bool),
    AbilitiesIcon(bool),
    LootIcon(bool),
    ModelIcon(bool),
    LeftHeader,
    RightHeader,
    InstanceRing,
    DungeonName,
    BossName,
    LootFrame,
}
impl Art {
    pub fn uv(self) -> &'static str {
        match self {
            Self::BossUp => "0.00195313,0.63671875,0.21386719,0.26757813",
            Self::BossDown => "0.00195313,0.63671875,0.10253906,0.15625000",
            Self::BossHighlight => "0.00195313,0.63671875,0.15820313,0.21191406",
            Self::DungeonUp => "0.00195313,0.34179688,0.42871094,0.52246094",
            Self::DungeonDown => "0.00195313,0.34179688,0.33300781,0.42675781",
            Self::DungeonHighlight => "0.34570313,0.68554688,0.33300781,0.42675781",
            Self::TabUp => "0.25585938,0.37890625,0.90332031,0.95898438",
            Self::TabDown => "0.12890625,0.25195313,0.90332031,0.95898438",
            Self::TabHighlight => "0.00195313,0.12500000,0.90332031,0.95898438",
            Self::BossIcon(true) => "0.90234375,0.99609375,0.26953125,0.31152344",
            Self::BossIcon(false) => "0.85546875,0.94921875,0.52441406,0.56640625",
            Self::LootIcon(true) => "0.63281250,0.72656250,0.61816406,0.66015625",
            Self::LootIcon(false) => "0.73046875,0.82421875,0.61816406,0.66015625",
            Self::ModelIcon(true) => "0.8046875,0.900390625,0.662109375,0.705078125",
            Self::ModelIcon(false) => "0.90234375,1,0.662109375,0.705078125",
            Self::AbilitiesIcon(true) => "0.806640625,0.8984375,0.70703125,0.748046875",
            Self::AbilitiesIcon(false) => "0.904296875,0.99609375,0.70703125,0.748046875",
            Self::LeftHeader => "0,0.755859375,0.9599609375,1",
            Self::RightHeader => "0.755859375,0,0.9599609375,1",
            Self::InstanceRing => "0.50585938,0.63085938,0.02246094,0.08203125",
            Self::DungeonName => "0.34570313,0.84570313,0.42871094,0.49121094",
            Self::BossName => "0.00195313,0.77343750,0.26953125,0.33105469",
            Self::LootFrame => "0.00195313,0.62890625,0.61816406,0.66210938",
        }
    }
}
pub fn art(name: String, kind: Art, rect: (f32, f32, f32, f32)) -> Element {
    cropped(name, SHEET, kind.uv(), rect)
}
pub fn highlight(name: String, kind: Art, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    let uv = kind.uv();
    rsx! {texture {name:{DynName(name)},texture_fdid:SHEET,tex_coords:uv,left,top,width,height,pos_type:"absolute",hidden:true}}
}

pub fn search_box(value: &str) -> Element {
    let mut parts = Vec::new();
    for (part, atlas, rect) in [
        ("Left", "common-search-border-left", (580., 32., 8., 20.)),
        (
            "Middle",
            "common-search-border-middle",
            (588., 32., 194., 20.),
        ),
        ("Right", "common-search-border-right", (782., 32., 8., 20.)),
        (
            "Icon",
            "common-search-magnifyingglass",
            (586., 38., 10., 10.),
        ),
    ] {
        parts.extend(named_atlas_texture(
            format!("EncounterJournalSearch{part}"),
            atlas,
            rect,
        ));
    }
    parts.extend(rsx! {editbox {name:"EncounterJournalSearchBox",text:value,left:580.0,top:32.0,width:210.0,height:20.0,font_size:12.0,font_color:WHITE,max_letters:80,text_insets:"18,16,0,0",pos_type:"absolute"}});
    parts
}

pub fn grid_background(view: &View) -> Element {
    let expansion = view
        .catalog
        .tiers
        .iter()
        .find(|t| t.id == view.tier)
        .map(|t| t.expansion);
    // JournalTier.Expansion is an authored ordering value (100..1200; Current Season9000).
    let atlas = match expansion {
        Some(100) => "UI-EJ-Classic",
        Some(200) => "UI-EJ-BurningCrusade",
        Some(300) => "UI-EJ-WrathoftheLichKing",
        Some(400) => "UI-EJ-Cataclysm",
        Some(500) => "UI-EJ-MistsofPandaria",
        Some(600) => "UI-EJ-WarlordsofDraenor",
        Some(700) => "ui-ej-legion",
        Some(800) => "ui-ej-battleforazeroth",
        Some(900) => "ui-ej-shadowlands",
        Some(1000) => "ui-ej-dragonflight",
        Some(1100) => "ui-ej-thewarwithin",
        Some(1200) => "ui-ej-midnight",
        // EJ_ContentTab_Select: no GetEJTierData entry -> evergreenBg.
        Some(9000) => "activities-background",
        _ => return vec![],
    };
    named_atlas_texture(
        "EncounterJournalInstanceSelectBackground".into(),
        atlas,
        (7., 63., 786., 425.),
    )
}

/// EJInstanceSelect_UpdateTitle: only Journeys overrides ADVENTURE_JOURNAL.
pub fn content_title(tab: Tab) -> &'static str {
    if tab == Tab::Journeys {
        "Journeys"
    } else {
        "Adventure Guide"
    }
}

pub fn portrait(view: &View) -> Element {
    let fdid = view
        .instance
        .and_then(|id| view.catalog.instance(id))
        .map_or(521753, |i| i.button);
    window_portrait_texture(&window_portrait_slot("EncounterJournalPortrait"), fdid)
}

/// NavBar500×34 TOPLEFT61,-22, NavBarTemplate tiled CS_HelpTextures background.
pub fn navigation(view: &View) -> Element {
    let mut children = cropped(
        "EncounterJournalNavBackground".into(),
        516764,
        "0,1,0.1875,0.25390625",
        (0., 0., 500., 34.),
    );
    let home_width = (navigation_text_width("Home") + 50.).min(128.);
    children.extend(nav_link(
        "EncounterJournalBack",
        "Home",
        "journal:back",
        (0., 2., home_width, 30.),
        true,
    ));
    if let Some(instance) = view.instance.and_then(|id| view.catalog.instance(id)) {
        let instance_left = home_width - 15.;
        let instance_width = navigation_text_width(&instance.name) + 53.;
        children.extend(nav_link(
            "EncounterJournalNavInstance",
            &instance.name,
            &format!("journal:instance:{}", instance.id),
            (instance_left, 2., instance_width, 30.),
            false,
        ));
        if let Some(encounter) = view.encounter.and_then(|id| view.catalog.encounter(id)) {
            let left = instance_left + instance_width;
            let width = (navigation_text_width(&encounter.name) + 53.).min(500. - left);
            children.extend(nav_link(
                "EncounterJournalNavEncounter",
                &encounter.name,
                &format!("journal:encounter:{}", encounter.id),
                (left, 2., width, 30.),
                false,
            ));
        }
    }
    rsx! {r#frame {name:"EncounterJournalNavBar",left:61.0,top:22.0,width:500.0,height:34.0,pos_type:"absolute",{children}}}
}
fn navigation_text_width(value: &str) -> f32 {
    ui_toolkit::text_measure::measure_text(
        value,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.,
    )
    .expect("Retail navigation font is loaded")
    .0
}
fn nav_link(
    name: &str,
    title: &str,
    action: &str,
    rect: (f32, f32, f32, f32),
    home: bool,
) -> Element {
    let (left, top, width, height) = rect;
    let mut art = if home {
        cropped(
            format!("{name}Background"),
            516763,
            "0.453125,0.703125,0.0078125,0.2421875",
            (0., 0., width, height),
        )
    } else {
        cropped(
            format!("{name}Background"),
            516764,
            "0,1,0.0625,0.12109375",
            (0., 0., width, height),
        )
    };
    art.extend(cropped(
        format!("{name}Arrow"),
        516763,
        "0.88867188,0.92968750,0.296875,0.53125",
        (width - 21., 0., 21., 30.),
    ));
    art.extend(label(
        format!("{name}Text"),
        title,
        (10., 0., width - 32., height),
        (12., "1,0.82,0,1", "LEFT"),
    ));
    rsx! {button {name:{DynName(name.into())},left,top,width,height,pos_type:"absolute",onclick:action,button_default_skin:false,{art}}}
}

/// PanelTabButtonTemplate32px high; Lua AnchorTabs applies +3px after sizing.
pub fn bottom_tabs(view: &View) -> Element {
    let mut left = 11.;
    [
        ("Journeys", "Journeys", "journeys", Tab::Journeys),
        (
            "MonthlyActivities",
            "Traveler's Log",
            "monthly",
            Tab::Monthly,
        ),
        ("Suggest", "Suggested", "suggested", Tab::Suggested),
        ("Dungeon", "Dungeons", "dungeons", Tab::Dungeons),
        ("Raid", "Raids", "raids", Tab::Raids),
        ("LootJournal", "Item Sets", "sets", Tab::ItemSets),
        ("Tutorials", "Tutorials", "tutorials", Tab::Tutorials),
    ]
    .into_iter()
    .flat_map(|(key, title, action, tab)| {
        let width = crate::merchant_frame_component::tab_width(title);
        let out = bottom_tab(key, title, action, view.tab == tab, left, width);
        left += width + 3.;
        out
    })
    .collect()
}
fn bottom_tab(
    key: &str,
    title: &str,
    action: &str,
    active: bool,
    left: f32,
    width: f32,
) -> Element {
    let name = format!("EncounterJournal{key}Tab");
    let prefix = if active {
        "uiframe-activetab"
    } else {
        "uiframe-tab"
    };
    let height = if active { 42. } else { 36. };
    let offset = if active { -1. } else { -3. };
    let right = width + if active { 8. } else { 7. } - 37.;
    let mut parts = named_atlas_texture(
        format!("{name}Left"),
        &format!("{prefix}-left"),
        (offset, 0., 35., height),
    );
    parts.extend(named_atlas_texture(
        format!("{name}Middle"),
        &format!("_{prefix}-center"),
        (offset + 35., 0., right - offset - 35., height),
    ));
    parts.extend(named_atlas_texture(
        format!("{name}Right"),
        &format!("{prefix}-right"),
        (right, 0., 37., height),
    ));
    let color = if active { WHITE } else { "1,0.82,0,1" };
    parts.extend(label(
        format!("{name}Text"),
        title,
        (0., 9., width, 10.),
        (10., color, "CENTER"),
    ));
    let action = format!("journal:tab:{action}");
    rsx! {button {name:{DynName(name)},left,top:494.0,width,height:32.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,{parts}}}
}

pub fn side_tabs(page: Page) -> Element {
    [(0,Page::Overview,"overview",Art::BossIcon(page==Page::Overview)),
     (2,Page::Loot,"loot",Art::LootIcon(page==Page::Loot)),
     (1,Page::Abilities,"abilities",Art::AbilitiesIcon(page==Page::Abilities)),
     (3,Page::Model,"model",Art::ModelIcon(page==Page::Model))]
     .into_iter().enumerate().flat_map(|(index,(id,value,action,icon))| {
        let name=format!("EncounterJournalPage{id}");
        let mut parts=art(format!("{name}ButtonArt"),if page==value {Art::TabDown}else{Art::TabUp},(0.,0.,63.,57.));
        parts.extend(highlight(format!("{name}ButtonHighlight"),Art::TabHighlight,(0.,0.,63.,57.)));
        parts.extend(art(format!("{name}Icon"),icon,(9.,7.,48.,43.)));
        let action=format!("journal:page:{action}");
        let top=99.+index as f32*55.;
        rsx! {button {name:{DynName(name)},left:780.0,top,width:63.0,height:57.0,pos_type:"absolute",frame_level:510.0,onclick:{action.as_str()},button_default_skin:false,{parts}}}
     }).collect()
}

pub fn boss_row(view: &View, encounter: &Encounter, index: usize) -> Element {
    let name = format!("JournalBoss{}", encounter.id);
    let active = view.encounter == Some(encounter.id);
    let mut parts = art(
        format!("{name}ButtonArt"),
        if active { Art::BossDown } else { Art::BossUp },
        (0., 0., 325., 55.),
    );
    parts.extend(highlight(
        format!("{name}ButtonHighlight"),
        Art::BossHighlight,
        (0., 0., 325., 55.),
    ));
    let portrait = view
        .catalog
        .creatures
        .iter()
        .find(|c| c.encounter == encounter.id)
        .map_or(0, |c| c.portrait);
    parts.extend(cropped(
        format!("{name}Portrait"),
        portrait,
        "0,1,0,1",
        (-4., -13., 128., 64.),
    ));
    parts.extend(label(
        format!("{name}Name"),
        &encounter.name,
        (105., 4.5, 160., 40.),
        (14., "0.827,0.659,0.463,1", "LEFT"),
    ));
    let top = 10. + index as f32 * 70.;
    let action = format!("journal:encounter:{}", encounter.id);
    rsx! {button {name:{DynName(name)},left:0.0,top,width:325.0,height:55.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,{parts}}}
}

pub fn instance_button(instance: &Instance) -> Element {
    let mut parts = cropped(
        "EncounterJournalInstanceButtonIcon".into(),
        instance.button,
        "0,0.68359375,0,0.7421875",
        (12., 10.5, 40., 40.),
    );
    parts.extend(art(
        "EncounterJournalInstanceButtonRing".into(),
        Art::InstanceRing,
        (0., 0., 64., 61.),
    ));
    let action = format!("journal:instance:{}", instance.id);
    rsx! {button {name:"EncounterJournalInstanceButton",left:7.0,top:67.0,width:64.0,height:61.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,{parts}}}
}

pub fn instance_lore(instance: &Instance) -> Element {
    let mut out = cropped(
        "EncounterJournalInstanceLoreArt".into(),
        instance.lore,
        "0,0.7617187,0,0.65625",
        (405., 73., 390., 330.),
    );
    out.extend(art(
        "EncounterJournalInstanceTitleBG".into(),
        Art::DungeonName,
        (469., 111., 256., 64.),
    ));
    out.extend(label(
        "EncounterJournalInstanceTitle".into(),
        &instance.name,
        (437., 112., 320., 35.),
        (24., GOLD, "CENTER"),
    ));
    out.extend(label(
        "EncounterJournalInstanceLore".into(),
        &instance.description,
        (437., 389., 315., 95.),
        (12., INK, "LEFT"),
    ));
    out
}

/// WowStyle1DropdownTemplate at the dimensions set by EncounterJournal_OnLoad.
pub fn dropdown(name: &str, title: &str, action: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    let mut parts = named_atlas_texture(
        format!("{name}Background"),
        "common-dropdown-textholder",
        (-8., -7., width + 16., 41.),
    );
    parts.extend(named_atlas_texture(
        format!("{name}Arrow"),
        "common-dropdown-a-button",
        (width - 26., 2., 27., 27.),
    ));
    parts.extend(label(
        format!("{name}Text"),
        title,
        (8., 8., width - 36., 10.),
        (12., WHITE, "LEFT"),
    ));
    rsx! {button {name:{DynName(name.into())},left,top,width,height,pos_type:"absolute",onclick:action,button_default_skin:false,{parts}}}
}

pub fn model_art(instance: &Instance, name: &str) -> Element {
    let mut out = cropped(
        "EncounterJournalInstanceBackground".into(),
        instance.background,
        "0.76953125,0,0,0.830078125",
        (399., 65., 394., 425.),
    );
    out.extend(cropped(
        "EncounterJournalModelPaper".into(),
        527690,
        "0.767578125,0,0,0.828125",
        (399., 64., 393., 424.),
    ));
    out.extend(art(
        "EncounterJournalModelTitleBG".into(),
        Art::BossName,
        (396.5, 427., 395., 63.),
    ));
    out.extend(label(
        "EncounterJournalModelTitle".into(),
        name,
        (404., 472., 380., 10.),
        (14., GOLD, "CENTER"),
    ));
    out
}
