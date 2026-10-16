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
/// Reset details (EncounterJournal_ClearDetails) and new lore text (ScrollingFontMixin:SetText).
/// The boss list retains its position as Retail's BossesScrollBox data provider does.
pub fn reset_changed_scrolls(
    previous: &View,
    next: &View,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    let lists: &[&str] = if previous.instance != next.instance {
        &[crate::adventure_guide::LIST, "EncounterJournalLore"]
    } else if previous.encounter != next.encounter
        || previous.page != next.page
        || previous.tab != next.tab
        || (next.tab == Tab::ItemSets && previous.set_filters != next.set_filters)
    {
        &[crate::adventure_guide::LIST]
    } else {
        &[]
    };
    for list in lists {
        registry.scroll_lists.scroll_to(list, 0);
    }
}

pub fn apply_postsetup(view: &View, registry: &mut ui_toolkit::registry::FrameRegistry) {
    apply_pane_texture_layers(registry);
    sync_search_state(registry);
    apply_button_art(view, registry);
    crate::adventure_guide_sections::sync_header_art(view, registry);
    apply_text_layout(registry);
    crate::adventure_journeys::apply_postsetup(registry);
    crate::adventure_tutorials::apply_postsetup(registry);
    crate::adventure_suggestions::apply_postsetup(view, registry);
    crate::adventure_sets::apply_postsetup(view, registry);
}
/// XML layers keep the ModelScene viewport above its BACKGROUND and below paper OVERLAY.
fn apply_pane_texture_layers(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::strata::DrawLayer::{Background, Overlay};
    for (name, layer) in [
        ("EncounterJournalInstanceSelectBackground", Background),
        ("EncounterJournalInstanceLoreArt", Background),
        ("EncounterJournalInstanceBackground", Background),
        ("EncounterJournalModelPaper", Overlay),
        ("EncounterJournalModelTitleBG", Overlay),
        ("EncounterJournalModelTitle", Overlay),
        ("EncounterJournalDifficultyIcon", Overlay),
        ("EncounterJournalLeftTitle", Overlay),
    ] {
        if let Some(id) = registry.get_by_name(name) {
            registry.get_mut(id).unwrap().draw_layer = layer;
        }
    }
}

fn sync_search_state(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, layout_values::Val, widgets::button::ButtonState};
    let Some(id) = registry.get_by_name(crate::adventure_guide::SEARCH) else {
        return;
    };
    let focused = registry.focused_frame == Some(id);
    let Some(WidgetData::EditBox(edit)) = &mut registry.get_mut(id).unwrap().widget_data else {
        return;
    };
    edit.max_letters = Some(40);
    let empty = edit.text.is_empty();
    let show_clear = focused || !empty;
    if let Some(clear) = registry.get_by_name("EncounterJournalSearchBoxClearButton") {
        registry.set_hidden(clear, !show_clear);
        let Some(WidgetData::Button(button)) = &registry.get(clear).unwrap().widget_data else {
            return;
        };
        let hovered = button.hovered;
        let inset = if button.state == ButtonState::Pushed {
            4.
        } else {
            3.
        };
        if let Some(icon) = registry.get_by_name("EncounterJournalSearchBoxClearIcon") {
            let frame = registry.get_mut(icon).unwrap();
            frame.alpha = if hovered { 1. } else { 0.5 };
            frame.position.left = Val::Px(inset);
            frame.position.top = Val::Px(inset);
        }
    }
    if let Some(icon) = registry.get_by_name("EncounterJournalSearchIcon")
        && let Some(WidgetData::Texture(texture)) = &mut registry.get_mut(icon).unwrap().widget_data
    {
        let color = if show_clear { 1. } else { 0.6 };
        texture.vertex_color = [color, color, color, 1.];
    }
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
    for creature in view.model_creatures() {
        apply_button_state(
            registry,
            &format!("JournalCreature{}", creature.id),
            Art::InstanceRing,
            Art::InstanceRing,
        );
    }
    apply_side_tab_art(view, registry);
}
fn apply_side_tab_art(view: &View, registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    for (index, page) in [
        (0, Page::Overview),
        (1, Page::Abilities),
        (2, Page::Loot),
        (3, Page::Model),
    ] {
        let name = format!("EncounterJournalPage{index}");
        let normal = if view.page == page {
            Art::TabDown
        } else {
            Art::TabUp
        };
        apply_button_state(registry, &name, normal, Art::TabDown);
        let disabled = !view.page_enabled(page);
        for suffix in ["ButtonArt", "Icon"] {
            let Some(id) = registry.get_by_name(&format!("{name}{suffix}")) else {
                continue;
            };
            if let Some(WidgetData::Texture(texture)) = registry
                .get_mut(id)
                .and_then(|frame| frame.widget_data.as_mut())
            {
                texture.desaturated = disabled;
            }
        }
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
    DungeonLootFrame,
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
            Self::DungeonLootFrame => "0.00195313,0.72265625,0.52441406,0.58691406",
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
        ("Left", "common-search-border-left", (575., 32., 8., 20.)),
        (
            "Middle",
            "common-search-border-middle",
            (583., 32., 199., 20.),
        ),
        ("Right", "common-search-border-right", (782., 32., 8., 20.)),
        (
            "Icon",
            "common-search-magnifyingglass",
            (581., 38., 10., 10.),
        ),
    ] {
        parts.extend(named_atlas_texture(
            format!("EncounterJournalSearch{part}"),
            atlas,
            rect,
        ));
    }
    parts.extend(rsx! {editbox {name:"EncounterJournalSearchBox",text:value,left:580.0,top:32.0,width:210.0,height:20.0,font_size:12.0,font_color:WHITE,max_letters:40,text_insets:"16,20,0,0",pos_type:"absolute"}});
    let instructions_hidden = !value.is_empty();
    parts.extend(rsx! {fontstring {name:"EncounterJournalSearchBoxInstructions",text:crate::auction_mail::text("SEARCH"),left:596.0,top:32.0,width:174.0,height:20.0,pos_type:"absolute",font_size:12.0,font_color:"0.35,0.35,0.35,1",justify_h:"LEFT",hidden:instructions_hidden}});
    // RSX static float literals truncate fractions; expressions preserve the XML center anchor.
    // Remove this binding when ui-toolkit-macros preserves numeric literal precision.
    let clear_top = 32.0 + (20.0 - 17.0) / 2.0;
    parts.extend(rsx! {button {name:"EncounterJournalSearchBoxClearButton",left:770.0,top:clear_top,width:17.0,height:17.0,
        pos_type:"absolute",button_default_skin:false,onclick:"journal:search-clear",
        texture {name:"EncounterJournalSearchBoxClearIcon",left:3.0,top:3.0,width:10.0,height:10.0,pos_type:"absolute",texture_atlas:"common-search-clearbutton",alpha:0.5}}});
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
        (3., 1., 786., 425.),
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

/// Retail NavBarTemplate owns sizing, arrow placement and real interaction regions.
pub fn navigation(view: &View) -> Element {
    use crate::navigation_bar::{NavBar, NavButton, nav_bar};
    let instance = view.instance.and_then(|id| view.catalog.instance(id));
    let encounter = view.encounter.and_then(|id| view.catalog.encounter(id));
    let instance_action = instance.map(|i| format!("journal:instance:{}", i.id));
    let encounter_action = encounter.map(|e| format!("journal:encounter:{}", e.id));
    let journey = view.renown.as_ref().and_then(|snapshot| {
        (view.tab == Tab::Journeys)
            .then_some(snapshot)?
            .factions
            .iter()
            .find(|faction| Some(faction.id) == view.journeys.selected)
    });
    let journey_action = journey.map(|faction| format!("journal:journey:{}", faction.id));
    let mut buttons = vec![NavButton {
        name: "EncounterJournalBack",
        text: crate::auction_mail::text("HOME"),
        action: "journal:back",
        text_width: navigation_text_width(crate::auction_mail::text("HOME")),
    }];
    if let Some(instance) = instance {
        buttons.push(NavButton {
            name: "EncounterJournalNavInstance",
            text: &instance.name,
            action: instance_action.as_deref().unwrap(),
            text_width: navigation_text_width(&instance.name),
        });
    }
    if let Some(encounter) = encounter {
        buttons.push(NavButton {
            name: "EncounterJournalNavEncounter",
            text: &encounter.name,
            action: encounter_action.as_deref().unwrap(),
            text_width: navigation_text_width(&encounter.name),
        });
    }
    if let Some(journey) = journey {
        buttons.push(NavButton {
            name: "JourneyProgressNav",
            text: &journey.name,
            action: journey_action.as_deref().unwrap(),
            text_width: navigation_text_width(&journey.name),
        });
    }
    nav_bar(NavBar {
        name: "EncounterJournalNavBar",
        rect: (61., 22., 500., 34.),
        buttons: &buttons,
        overflow_action: "journal:nav-overflow",
        overflow_open: view.nav_overflow_open,
    })
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

/// Shared PanelTabButtonTemplate; source names/order and Lua +3px spacing.
pub fn bottom_tabs(view: &View) -> Element {
    use crate::auction_mail::text as global_string;
    let mut left = 11.;
    [
        ("Journeys", "JOURNEYS_LABEL", "journeys", Tab::Journeys),
        (
            "MonthlyActivities",
            "MONTHLY_ACTIVITIES_TAB",
            "monthly",
            Tab::Monthly,
        ),
        (
            "Suggest",
            "AJ_SUGGESTED_CONTENT_TAB",
            "suggested",
            Tab::Suggested,
        ),
        ("Dungeon", "DUNGEONS", "dungeons", Tab::Dungeons),
        ("Raid", "RAIDS", "raids", Tab::Raids),
        (
            "LootJournal",
            "LOOT_JOURNAL_ITEM_SETS",
            "sets",
            Tab::ItemSets,
        ),
        ("Tutorials", "EJ_TUTORIALS", "tutorials", Tab::Tutorials),
    ]
    .into_iter()
    .flat_map(|(key, title, action, tab)| {
        let title = global_string(title);
        let width = crate::merchant_frame_component::tab_width(title);
        let out = crate::merchant_frame_component::tab(
            &format!("EncounterJournal{key}Tab"),
            title,
            (left, 494., width),
            view.tab == tab,
            &format!("journal:tab:{action}"),
        );
        left += width + 3.;
        out
    })
    .collect()
}

pub fn side_tabs(view: &View) -> Element {
    let page = view.page;
    let has_overview = view.instance_has_overview();
    [
        (
            0,
            Page::Overview,
            "overview",
            Art::BossIcon(page == Page::Overview),
        ),
        (2, Page::Loot, "loot", Art::LootIcon(page == Page::Loot)),
        (
            1,
            Page::Abilities,
            "abilities",
            Art::AbilitiesIcon(page == Page::Abilities),
        ),
        (3, Page::Model, "model", Art::ModelIcon(page == Page::Model)),
    ]
    .into_iter()
    .filter(|(_, value, _, _)| *value != Page::Abilities || has_overview)
    .enumerate()
    .flat_map(|(index, choice)| side_tab(view, index, choice))
    .collect()
}
fn side_tab(view: &View, index: usize, (id, page, key, icon): (u8, Page, &str, Art)) -> Element {
    let name = format!("EncounterJournalPage{id}");
    let selected = view.page == page;
    let disabled = !view.page_enabled(page);
    let mut parts = art(
        format!("{name}ButtonArt"),
        if selected { Art::TabDown } else { Art::TabUp },
        (0., 0., 63., 57.),
    );
    parts.extend(highlight(
        format!("{name}ButtonHighlight"),
        Art::TabHighlight,
        (0., 0., 63., 57.),
    ));
    parts.extend(art(format!("{name}Icon"), icon, (9., 7., 48., 43.)));
    let action = format!("journal:page:{key}");
    let top = 99. + index as f32 * 55.;
    rsx! {
        button {
            name: {DynName(name)}, left: 780., top, width: 63., height: 57.,
            pos_type: "absolute", frame_level: 510., disabled,
            onclick: {action.as_str()}, button_default_skin: false, {parts}
        }
    }
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
    parts.extend(crate::adventure_defeated::overlay(view, encounter));
    let top = 10. + index as f32 * 70.;
    let action = format!("journal:encounter:{}", encounter.id);
    rsx! {button {name:{DynName(name)},left:0.0,top,width:325.0,height:55.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,{parts}}}
}

/// XML1611–1624 / EJ.lua1189–1196,1261–1270; SetFlagIcon selects published atlases.
fn difficulty_glyph(difficulty: u32) -> (Element, f32, f32) {
    let atlas = match difficulty {
        15 => Some("icons_16x16_heroic"),
        16 => Some("icons_16x16_mythic"),
        _ => None,
    };
    if let Some(atlas) = atlas {
        let (width, height) =
            crate::inworld_unit_frames_component::inworld_unit_frames_art::atlas_size(
                atlas,
                ui_toolkit::atlas::thread_skin(),
            )
            .expect("Journal difficulty atlas must be published for this skin");
        let icon = named_atlas_texture(
            "EncounterJournalDifficultyIcon".into(),
            atlas,
            (INFO.0 + 60., INFO.1 + 16., width, height),
        );
        (
            icon,
            INFO.0 + 60. + width + 3.,
            INFO.1 + 16. + height / 2. - 6.,
        )
    } else {
        (vec![], INFO.0 + 65., INFO.1 + 20.)
    }
}

pub(crate) fn instance_header(view: &View, instance: &Instance) -> Element {
    let (mut content, left, top) = difficulty_glyph(view.difficulty);
    // Preserve the source12px FontString rectangle; center the native intrinsic glyph height.
    content.extend(rsx! {
        frame {
            name: "EncounterJournalLeftTitleBounds", left, top, width: 290., height: 12.,
            pos_type: "absolute", layout: "flex-row", align: "center",
            fontstring {
                name: "EncounterJournalLeftTitle", text: {instance.name.as_str()}, width: 290., height: "auto",
                pos_type: "relative", font_size: 16., font_color: HEADER, justify_h: "LEFT",
                word_wrap: false, max_lines: 1, shadow_color: "0,0,0,1", shadow_offset: "1,-1"
            }
        }
    });
    content
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
        (3., 9., 390., 330.),
    );
    out.extend(art(
        "EncounterJournalInstanceTitleBG".into(),
        Art::DungeonName,
        (67., 47., 256., 64.),
    ));
    out.extend(label(
        "EncounterJournalInstanceTitle".into(),
        &instance.name,
        (35., 48., 320., 35.),
        (24., GOLD, "CENTER"),
    ));
    out
}

/// Shared WowStyle1DropdownTemplate, including open/pressed/disabled regions.
pub fn dropdown(
    name: &str,
    title: &str,
    action: &str,
    rect: (f32, f32, f32, f32),
    open: bool,
) -> Element {
    use crate::menu_primitives::{DropdownButton, DropdownState, dropdown_button_with_state};
    dropdown_button_with_state(
        DropdownButton {
            frame_name: name,
            label_name: &format!("{name}Text"),
            arrow_name: &format!("{name}Arrow"),
            text: title,
            width: rect.2,
            height: rect.3,
            x: rect.0,
            y: -rect.1,
            background_color: "",
            text_color: WHITE,
            arrow_color: "",
            onclick: Some(action),
        },
        DropdownState {
            open,
            ..Default::default()
        },
    )
}

pub fn model_art(instance: &Instance, name: &str) -> Element {
    let mut out = cropped(
        "EncounterJournalInstanceBackground".into(),
        instance.background,
        "0.76953125,0,0,0.830078125",
        (0., 0., 394., 425.),
    );
    out.extend(cropped(
        "EncounterJournalModelPaper".into(),
        527690,
        "0.767578125,0,0,0.828125",
        (0., -1., 393., 424.),
    ));
    out.extend(art(
        "EncounterJournalModelTitleBG".into(),
        Art::BossName,
        (-2.5, 362., 395., 63.),
    ));
    out.extend(label(
        "EncounterJournalModelTitle".into(),
        name,
        (5., 407., 380., 10.),
        (14., GOLD, "CENTER"),
    ));
    out
}
