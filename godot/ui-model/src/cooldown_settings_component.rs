//! Blizzard_CooldownViewer/CooldownViewerSettings.xml: source shell, categories and item hosts.
use crate::bank_art::{WHITE, edit_box, label, texture};
use crate::menu_primitives::{ContextMenuItem, MenuElementKind, menu_element};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture, window_chrome};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SCROLL: &str = "CooldownSettingsScroll";
pub const SEARCH: &str = "CooldownSettingsSearch";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CooldownSettingEntry {
    pub id: u32,
    pub category: u32,
    pub label: String,
    pub icon_fdid: u32,
    pub selected: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CooldownSettingsView {
    pub open: bool,
    /// Spells=0, auras=2; source display modes contain multiple categories together.
    pub category: usize,
    pub entries: Vec<CooldownSettingEntry>,
    pub portrait_fdid: u32,
    pub status: String,
    pub query: String,
    pub search_focused: bool,
    pub menu: Option<u32>,
    pub menu_position: [f32; 2],
    pub dragging: Option<u32>,
    pub can_undo: bool,
}

/// CooldownViewerSettings.xml RevertButton inherits UIPanelButtonTemplate.
pub fn apply_settings_postsetup(
    state: &CooldownSettingsView,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    apply_search_insets(registry);
    apply_search_art(state, registry);
    apply_clear_button_art(registry);
    if ui_toolkit::atlas::thread_skin() != ui_toolkit::atlas::ActiveSkin::Modern {
        return;
    }
    let Some(id) = registry.get_by_name("CooldownSettingsUndo") else {
        return;
    };
    let Some(ui_toolkit::frame::WidgetData::Button(button)) =
        &mut registry.get_mut(id).expect("Revert frame").widget_data
    else {
        panic!("Revert must be a UIPanel button");
    };
    use ui_toolkit::widgets::texture::TextureSource;
    button.normal_texture = Some(TextureSource::FileDataId(130828));
    button.pushed_texture = Some(TextureSource::FileDataId(130825));
    button.disabled_texture = Some(TextureSource::FileDataId(130824));
    button.highlight_texture = None;
}

fn apply_search_insets(registry: &mut ui_toolkit::registry::FrameRegistry) {
    let Some(id) = registry.get_by_name(SEARCH) else {
        return;
    };
    let Some(ui_toolkit::frame::WidgetData::EditBox(edit)) =
        &mut registry.get_mut(id).expect("Search box").widget_data
    else {
        panic!("Settings search must be an EditBox");
    };
    edit.text_insets = [16.0, 20.0, 0.0, 0.0];
}

/// InputBoxTemplates.lua:175-200 dims only an empty, unfocused SearchBox glyph.
fn apply_search_art(
    state: &CooldownSettingsView,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    let Some(id) = registry.get_by_name("CooldownSettingsSearchIcon") else {
        return;
    };
    let Some(ui_toolkit::frame::WidgetData::Texture(texture)) =
        &mut registry.get_mut(id).expect("Search glyph").widget_data
    else {
        panic!("Search glyph must be a Texture");
    };
    let channel = if state.search_focused || !state.query.is_empty() {
        1.0
    } else {
        0.6
    };
    texture.vertex_color = [channel, channel, channel, 1.0];
}

/// ClearButtonMixin:215-230 hover opacity and pressed top-left displacement.
fn apply_clear_button_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, layout_values::Val, widgets::button::ButtonState};
    let Some(id) = registry.get_by_name("CooldownSettingsSearchClearButton") else {
        return;
    };
    let Some(WidgetData::Button(button)) = &registry.get(id).expect("Clear button").widget_data
    else {
        panic!("Clear must be a Button");
    };
    let alpha = if button.hovered { 1.0 } else { 0.5 };
    let inset = if button.state == ButtonState::Pushed {
        4.0
    } else {
        3.0
    };
    let icon = registry
        .get_by_name("CooldownSettingsSearchClearIcon")
        .expect("Clear icon");
    let frame = registry.get_mut(icon).expect("Clear icon frame");
    frame.position.left = Val::Px(inset);
    frame.position.top = Val::Px(inset);
    let Some(WidgetData::Texture(texture)) = &mut frame.widget_data else {
        panic!("Clear icon must be a Texture");
    };
    texture.vertex_color = [1.0, 1.0, 1.0, alpha];
}

/// SearchBoxTemplate XML:206-254; GlobalStrings55224 supplies instructions.
fn settings_search(state: &CooldownSettingsView) -> Element {
    let mut children = edit_box(SEARCH, (72.0, 30.0, 184.0, 30.0));
    children.extend(named_atlas_texture(
        "CooldownSettingsSearchIcon".into(),
        "common-search-magnifyingglass",
        (73.0, 41.0, 10.0, 10.0),
    ));
    if state.query.is_empty() {
        children.extend(label(
            "CooldownSettingsSearchInstructions".into(),
            "Enter search text",
            (88.0, 30.0, 148.0, 30.0),
            (12.0, "0.35,0.35,0.35,1.0", "LEFT"),
        ));
    }
    if state.search_focused || !state.query.is_empty() {
        let icon = named_atlas_texture(
            "CooldownSettingsSearchClearIcon".into(),
            "common-search-clearbutton",
            (3.0, 3.0, 10.0, 10.0),
        );
        children.extend(rsx! { button { name:"CooldownSettingsSearchClearButton", width:17.0,height:17.0,
            left:236.0,top:"36.5",pos_type:"absolute",button_default_skin:false,onclick:"cooldown_search_clear",{icon}
        }});
    }
    // Retail settings-menu options have no backing unlearned/reset state yet.
    let icon = centered_art(
        "CooldownSettingsOptionsIcon".into(),
        "questlog-icon-setting",
        [7.5, 8.0],
    );
    children.extend(
        rsx! { button { name:"CooldownSettingsSettingsDropdown",width:15.0,height:16.0,
            left:261.0,top:37.0,pos_type:"absolute",button_default_skin:false,disabled:true,{icon}
        }},
    );
    children
}

pub fn settings_panel(ctx: &SharedContext, state: &CooldownSettingsView) -> Element {
    if !state.open {
        return Vec::new();
    }
    let mut children = window_chrome(
        "CooldownViewerSettings",
        (399.0, 609.0),
        "Cooldown Settings",
        "cooldown_close",
    );
    children.extend(settings_inset());
    children.extend(crate::quest_art::window_portrait_texture(
        &crate::quest_art::window_portrait_slot("CooldownSettingsPortrait"),
        state.portrait_fdid,
    ));
    children.extend(category_tabs(state));
    children.extend(settings_search(state));
    children.extend(category_scroll(ctx, state));
    children.extend(item_menu(state));
    children.extend(settings_footer(state));
    rsx! { r#frame {name:"CooldownViewerSettings", width:399.0,height:609.0,
        strata:crate::ui::strata::FrameStrata::Dialog,mouse_enabled:true,
        pos_type:"absolute",left:16.0,top:116.0,{children}}
    }
}

fn settings_inset() -> Element {
    // ButtonFrameTemplate Inset TOPLEFT(4,-60), BOTTOMRIGHT(-6,26).
    let mut children = rsx! { texture {
        name:"CooldownSettingsInsetBackground",texture_atlas:"character-panel-background",
        width:389.0,height:523.0,pos_type:"absolute",left:4.0,top:60.0,
    }};
    children.extend(crate::auction_house_frame_component::inset_border(
        "CooldownSettingsInset",
        (4.0, 60.0, 389.0, 523.0),
    ));
    children
}

fn settings_footer(state: &CooldownSettingsView) -> Element {
    use crate::menu_primitives::{DropdownButton, DropdownState, dropdown_button_with_state};
    let mut children = dropdown_button_with_state(
        DropdownButton {
            frame_name: "CooldownSettingsLayoutDropdown",
            label_name: "CooldownSettingsLayoutText",
            arrow_name: "CooldownSettingsLayoutArrow",
            text: "Default",
            width: 120.0,
            height: 25.0,
            x: 5.0,
            y: -581.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: None,
        },
        DropdownState {
            enabled: false,
            ..Default::default()
        },
    );
    children.extend(crate::quest_art::panel_button(
        "CooldownSettingsUndo".into(),
        "Revert",
        "cooldown_undo",
        state.can_undo,
        (232.0, 583.0, 164.0, 22.0),
    ));
    children
}

fn centered_art(name: String, atlas: &str, center: [f32; 2]) -> Element {
    let art = ui_toolkit::atlas::get_region(atlas).expect("Retail cooldown atlas loaded");
    named_atlas_texture(
        name,
        atlas,
        (
            center[0] - art.width / 2.0,
            center[1] - art.height / 2.0,
            art.width,
            art.height,
        ),
    )
}

fn category_tabs(state: &CooldownSettingsView) -> Element {
    [
        ("Spells", "icon_cooldownmanager", 0),
        ("Auras", "icon_trackedbuffs", 2),
        ("GroupBuffs", "icon_buffreorder", 4),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (name, icon, category))| {
        let prefix = format!("CooldownSettings{name}Tab");
        let mut art = centered_art(
            format!("{prefix}Background"),
            "common-sidetab",
            [21.5, 27.5],
        );
        art.extend(centered_art(format!("{prefix}Icon"), icon, [19.5, 27.5]));
        if state.category == category {
            art.extend(centered_art(
                format!("{prefix}Selected"),
                "common-sidetab-selected",
                [21.5, 27.5],
            ));
        }
        let action = format!("cooldown_category:{category}");
        let disabled = category == 4; // GroupBuffFilter has no backing model yet; never invent entries.
        rsx! {button {name:{DynName(prefix)},width:43.0,height:55.0,button_default_skin:false,
        onclick:{action.as_str()},disabled,button_atlas_highlight:"common-sidetab-hover",
        pos_type:"absolute",left:399.0,top:{28.0+index as f32*58.0},{art}}}
    })
    .collect()
}

fn category_scroll(ctx: &SharedContext, state: &CooldownSettingsView) -> Element {
    let (rows, content_height) = category_contents(state);
    let geometry = pixel_geometry(508.0, content_height, 508.0);
    let offset = geometry.clamp(ctx.scroll_first_row(SCROLL));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: SCROLL,
        left: 17.0 + 352.0,
        top: 72.0,
        height: 508.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! {r#frame {name:{DynName(SCROLL.into())},width:352.0,height:508.0,pos_type:"absolute",left:17.0,top:72.0,mouse_enabled:true,scroll_list:config,
    r#frame {name:"CooldownSettingsContent",width:344.0,height:content_height,pos_type:"absolute",left:0.0,top:{-(offset as f32)},{rows}}
    }
    {bar.element()}}
}

fn category_contents(state: &CooldownSettingsView) -> (Element, f32) {
    let categories = if state.category < 2 {
        [
            (0, "Essential Cooldowns"),
            (1, "Utility Cooldowns"),
            (4, "Not Displayed"),
        ]
    } else {
        [
            (2, "Tracked Buffs"),
            (3, "Tracked Bars"),
            (5, "Not Displayed"),
        ]
    };
    let query = state.query.to_lowercase();
    let mut children = Vec::new();
    let mut top = 0.0;
    for (category, title) in categories {
        let entries: Vec<_> = state
            .entries
            .iter()
            .filter(|entry| {
                let matches_category = if category < 4 {
                    entry.selected && entry.category == category
                } else {
                    !entry.selected && (entry.category < 2) == (category == 4)
                };
                matches_category && entry.label.to_lowercase().contains(&query)
            })
            .collect();
        let stride = if category == 3 { 1 } else { 7 };
        let count = entries.len().max(1).div_ceil(stride);
        let height = 37.0 + count as f32 * 46.0 - 8.0;
        children.extend(category_header(category, title, top));
        for (index, entry) in entries.into_iter().enumerate() {
            let rect = (
                13.0 + (index % stride) as f32 * 46.0,
                top + 37.0 + (index / stride) as f32 * 46.0,
            );
            children.extend(setting_item(entry, rect));
        }
        top += height + 18.0;
    }
    (children, top)
}

fn category_header(category: u32, title: &str, top: f32) -> Element {
    let prefix = format!("CooldownSettingsCategory{category}");
    let left =
        ui_toolkit::atlas::get_region("Options_ListExpand_Left").expect("Retail list header");
    let right =
        ui_toolkit::atlas::get_region("Options_ListExpand_Right").expect("Retail list header");
    let mut children = named_atlas_texture(
        format!("{prefix}Left"),
        "Options_ListExpand_Left",
        (0.0, top, left.width, left.height),
    );
    children.extend(named_atlas_texture(
        format!("{prefix}Right"),
        "Options_ListExpand_Right",
        (344.0 - right.width, top, right.width, right.height),
    ));
    children.extend(named_atlas_texture(
        format!("{prefix}Middle"),
        "_Options_ListExpand_Middle",
        (left.width, top, 344.0 - left.width - right.width, 22.0),
    ));
    children.extend(label(
        format!("{prefix}Name"),
        title,
        (10.0, top + 3.5, 300.0, 15.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    children
}

fn setting_item(entry: &CooldownSettingEntry, (left, top): (f32, f32)) -> Element {
    let name = format!("CooldownSettingsItem{}", entry.id);
    let width = if entry.category == 3 && entry.selected {
        317.0
    } else {
        38.0
    };
    let mut art = texture(
        format!("{name}Icon"),
        entry.icon_fdid,
        (0.0, 0.0, 38.0, 38.0),
        WHITE,
    );
    if width > 38.0 {
        art.extend(named_atlas_texture(
            format!("{name}BarBackground"),
            "UI-HUD-CoolDownManager-Bar-BG",
            (41.0, 6.0, 282.0, 30.0),
        ));
        // CooldownViewerSettings.lua:6,597: known/unlocked default bar tint.
        art.extend(rsx! { texture {
            name:{DynName(format!("{name}Bar"))},texture_atlas:"UI-HUD-CoolDownManager-Bar",
            width:274.0,height:19.0,pos_type:"absolute",left:43.0,top:9.5,
            vertex_color:"1,0.5,0.25,1",
        }});
        art.extend(label(
            format!("{name}Name"),
            &entry.label,
            (48.0, 9.5, 244.0, 19.0),
            (12.0, WHITE, "LEFT"),
        ));
    }
    let action = format!("cooldown_pick:{}", entry.id);
    let template = crate::selection_list::RowTemplate {
        normal: None,
        selected: None,
        hover: Some(crate::selection_list::RowArt {
            art: crate::selection_list::Art::File(130718, [0.0, 1.0, 0.0, 1.0]),
            insets: [0.0, 0.0, width - 38.0, 0.0],
            alpha: 1.0,
            additive: true,
        }),
    };
    crate::selection_list::row(
        name,
        &action,
        true,
        (left, top, width, 38.0),
        (&template, false),
        art,
    )
}

fn item_menu(state: &CooldownSettingsView) -> Element {
    let Some(id) = state.menu else {
        return Vec::new();
    };
    let Some(entry) = state.entries.iter().find(|entry| entry.id == id) else {
        return Vec::new();
    };
    let name = "CooldownSettingsVisibility";
    let action = format!("cooldown_select:{id}");
    // GetCategoryAssignmentText uses these GlobalStrings, not a generic checkbox caption.
    let label = if entry.selected {
        "Remove from Bar".to_string()
    } else {
        let category = [
            "Essential Cooldowns",
            "Utility Cooldowns",
            "Tracked Buffs",
            "Tracked Bars",
        ][entry.category as usize];
        format!("Assign to {category}")
    };
    let width = ui_toolkit::text_measure::measure_text(
        &label,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .expect("menu font metrics")
    .0
    .ceil()
        + 32.0;
    let mut children = crate::selection_list::menu_background(
        "CooldownSettingsMenuBackground".into(),
        (width, 43.0),
    );
    children.extend(menu_element(
        ContextMenuItem {
            name,
            label: &label,
            action: &action,
        },
        MenuElementKind::Button,
        true,
        (8.0, 8.0, width - 16.0, 20.0),
    ));
    rsx! {r#frame {name:"CooldownSettingsMenu",width,height:43.0,frame_level:600,
    pos_type:"absolute",left:{state.menu_position[0]},top:{state.menu_position[1]},mouse_enabled:true,{children}}}
}
