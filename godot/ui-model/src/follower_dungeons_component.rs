//! LFDFrame.xml: follower roles, type dropdown, explanation, ScrollBox and Find Group.
use crate::bank_art::{WHITE, label, texture};
use crate::follower_dungeons::View;
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, DropdownButton, DropdownState, MenuElementKind,
    context_menu_with_rows, dropdown_button_with_state, menu_element, menu_row_rect_with_heading,
};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{
    DynName, NORMAL_FONT_COLOR, named_atlas_texture, panel_button, window_chrome,
};
use crate::ui::screens::auction_house_frame_component::inset_border;
use crate::ui::screens::merchant_frame_component::{tab, tab_width};
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::{rsx, screen::SharedContext, strata::FrameStrata, widget_def::Element};

pub const FOLLOWER_LIST: &str = "LFDQueueFrameFollowerScrollBox";
pub const ROW_HEIGHT: usize = 20;
const LIST_HEIGHT: f32 = 177.0;
const QUEUE_LEFT: f32 = 224.0;

pub(super) fn screen(shared: &SharedContext) -> Element {
    let view = shared.get::<View>().cloned().unwrap_or_default();
    if !view.visible {
        return vec![];
    }
    let mut children = window_chrome("PVEFrame", (563.0, 428.0), "Group Finder", "follower:close");
    children.extend(named_atlas_texture(
        "PVEFramePortrait".into(),
        "groupfinder-eye-frame",
        (-5.0, -7.0, 62.0, 62.0),
    ));
    children.extend(left_panel());
    children.extend(categories());
    children.extend(queue_panel(shared, &view));
    children.extend(tabs());
    rsx! { r#frame { name: "PVEFrame", width: 563.0, height: 428.0, left: 100.0, top: 84.0,
    pos_type: "absolute", strata: FrameStrata::Dialog, mouse_enabled: true, {children} } }
}
/// PVEFrame.xml PanelTabButtonTemplate: BOTTOMLEFT19,-30; subsequent LEFT-to-RIGHT -16.
fn tabs() -> Element {
    let mut left = 19.0;
    [
        ("Group Finder", ""),
        ("Player vs. Player", "follower:pvp"),
        ("Mythic+ Dungeons", ""),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (title, action))| {
        let width = tab_width(title);
        let elements = tab(
            &format!("PVEFrameTab{}", index + 1),
            title,
            (left, 428.0 + 30.0 - 32.0, width),
            index == 0,
            action,
        );
        left += width - 16.0;
        elements
    })
    .collect()
}
fn categories() -> Element {
    [
        ("Dungeon Finder", 133_076, true),
        ("Raid Finder", 341_547, false),
        ("Premade Groups", 464_820, false),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (title, icon, enabled))| category(index, title, icon, enabled))
    .collect()
}
/// GroupFinderGroupButtonTemplate; PVEFrame.lua no-scenarios anchors101/191/281.
fn category(index: usize, title: &str, icon: u32, enabled: bool) -> Element {
    let name = format!("GroupFinderFrameGroupButton{}", index + 1);
    let mut art = category_surfaces(&name, icon, enabled);
    let color = if enabled {
        NORMAL_FONT_COLOR
    } else {
        "0.5,0.5,0.5,1"
    };
    art.extend(label(
        format!("{name}Name"),
        title,
        (83.0, 0.0, 106.0, 60.0),
        (16.0, color, "LEFT"),
    ));
    let top = 101.0 + index as f32 * 90.0;
    let disabled = !enabled;
    rsx! { button { name: {DynName(name)}, width: 203.0, height: 60.0,
    left: 10.0, top, pos_type: "absolute", button_default_skin: false,
    onclick: "follower:type:follower", disabled, {art} } }
}
fn category_surfaces(name: &str, icon: u32, enabled: bool) -> Element {
    let coords = if enabled {
        "0.00390625,0.87890625,0.75195313,0.83007813"
    } else {
        "0.00390625,0.87890625,0.671875,0.75000000"
    };
    let mut art = blue_texture(
        format!("{name}Background"),
        593_918,
        (-10.5, -10.0, 224.0, 80.0),
        coords,
    );
    art.extend(named_atlas_texture(
        format!("{name}Ring"),
        "bluemenu-Ring",
        (-12.0, -17.0, 95.0, 96.0),
    ));
    art.extend(texture(
        format!("{name}Icon"),
        icon,
        (2.5, -2.0, 66.0, 66.0),
        WHITE,
    ));
    art.extend(category_highlight(name));
    art
}
fn category_highlight(name: &str) -> Element {
    rsx! { texture { name: {DynName(format!("{name}Highlight"))},
    width: 224.0, height: 80.0, left: {-10.5}, top: -10.0, pos_type: "absolute",
    texture_fdid: 593_918, tex_coords: "0.00390625,0.87890625,0.75195313,0.83007813",
    draw_layer: "HIGHLIGHT", blend_mode: "ADD", alpha: "0.8" } }
}
/// PVEFrame.xml BlueBg, corners, lines and filigree. UVs are source TexCoords.
fn left_panel() -> Element {
    let inset = inset_border("PVEFrameLeftInset", (0.0, 0.0, 217.0, 400.0));
    let mut art = rsx! { r#frame { name: "PVEFrameLeftInset", width: 217.0, height: 400.0,
    left: 4.0, top: 24.0, pos_type: "absolute", {inset} } };
    art.extend(left_panel_textures());
    art
}
fn left_panel_textures() -> Element {
    [
        (
            "BlueBg",
            593_918,
            (7.0, 23.0, 209.0, 399.0),
            "0.00390625,0.82421875,0.18554688,0.58984375",
        ),
        (
            "TLCorner",
            593_918,
            (7.0, 23.0, 64.0, 64.0),
            "0.00390625,0.25390625,0.00097656,0.06347656",
        ),
        (
            "TRCorner",
            593_918,
            (151.0, 23.0, 64.0, 64.0),
            "0.51953125,0.76953125,0.00097656,0.06347656",
        ),
        (
            "BRCorner",
            593_918,
            (151.0, 357.0, 64.0, 64.0),
            "0.00390625,0.25390625,0.06542969,0.12792969",
        ),
        (
            "BLCorner",
            593_918,
            (7.0, 357.0, 64.0, 64.0),
            "0.26171875,0.51171875,0.00097656,0.06347656",
        ),
        (
            "LLVert",
            593_919,
            (7.0, 87.0, 43.0, 270.0),
            "0.0625,0.3984375,0,1",
        ),
        (
            "RLVert",
            593_919,
            (172.0, 87.0, 43.0, 270.0),
            "0.4140625,0.75,0,1",
        ),
        (
            "BottomLine",
            593_917,
            (71.0, 378.0, 80.0, 43.0),
            "0,1,0.359375,0.6953125",
        ),
        (
            "TopLine",
            593_917,
            (71.0, 23.0, 80.0, 43.0),
            "0,1,0.0078125,0.34375",
        ),
        (
            "TopFiligree",
            593_918,
            (19.0, 29.0, 185.0, 55.0),
            "0.00390625,0.7265625,0.12988281,0.18359375",
        ),
        (
            "BottomFiligree",
            593_918,
            (19.0, 363.0, 185.0, 55.0),
            "0.26171875,0.984375,0.06542969,0.11914063",
        ),
    ]
    .into_iter()
    .flat_map(|(part, fdid, rect, uv)| blue_texture(format!("PVEFrame{part}"), fdid, rect, uv))
    .collect()
}
fn blue_texture(
    name: String,
    fdid: u32,
    (left, top, width, height): (f32, f32, f32, f32),
    uv: &str,
) -> Element {
    rsx! { texture { name: {DynName(name)}, texture_fdid: fdid, tex_coords: uv,
    left, top, width, height, pos_type: "absolute" } }
}
fn queue_panel(shared: &SharedContext, view: &View) -> Element {
    let mut children = queue_background();
    children.extend(roles(view));
    children.extend(dungeon_type(view));
    children.extend(label(
        "LFDQueueFrameFollowerTitle".into(),
        "Follower Dungeons",
        (15.0, 156.0, 280.0, 24.0),
        (18.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    children.extend(label(
        "LFDQueueFrameFollowerDescription".into(),
        &description(view),
        (15.0, 183.0, 280.0, 37.0),
        (12.0, WHITE, "LEFT"),
    ));
    children.extend(dungeon_list(shared, view));
    children.extend(panel_button(
        "LFDQueueFrameFindGroupButton".into(),
        "Find Group",
        "follower:queue",
        view.can_queue(),
        (101.5, 402.0, 135.0, 22.0),
    ));
    rsx! { r#frame { name: "LFDParentFrame", width: 338.0, height: 428.0,
    left: QUEUE_LEFT, top: 0.0, pos_type: "absolute", {children} } }
}
/// LFDFrame.xml:512x256 BOTTOMLEFT6,26; clip by Inset TOPLEFT4,-60/BOTTOMRIGHT-6,26.
fn queue_background() -> Element {
    let paper = texture(
        "LFDQueueFrameBackground".into(),
        337_492,
        (2.0, 86.0, 512.0, 256.0),
        WHITE,
    );
    let mut contents = texture(
        "LFDParentFrameInsetBg".into(),
        374_154,
        (0.0, 0.0, 328.0, 342.0),
        WHITE,
    );
    contents.extend(paper);
    let mut art = rsx! { r#frame { name: "LFDParentFrameInset", width: 328.0, height: 342.0,
    left: 4.0, top: 60.0, pos_type: "absolute", {contents} } };
    art.extend(texture(
        "LFDParentFrameRoleBackground".into(),
        517_165,
        (2.0, 25.0, 512.0, 128.0),
        WHITE,
    ));
    art.extend(inset_border(
        "LFDParentFrameInsetBorder",
        (4.0, 60.0, 328.0, 342.0),
    ));
    art
}
fn roles(view: &View) -> Element {
    let mut buttons: Element = [
        (
            "Tank",
            "tank",
            GroupRoleSnapshot::Tank,
            "UI-LFG-RoleIcon-Tank",
            50.0,
        ),
        (
            "Healer",
            "healer",
            GroupRoleSnapshot::Healer,
            "UI-LFG-RoleIcon-Healer",
            123.0,
        ),
        (
            "DPS",
            "damage",
            GroupRoleSnapshot::Damage,
            "UI-LFG-RoleIcon-DPS",
            194.0,
        ),
    ]
    .into_iter()
    .flat_map(|(name, action, role, atlas, left)| {
        role_button(view, name, action, role, atlas, left)
    })
    .collect();
    buttons.extend(leader_button(view));
    buttons
}
/// Leader inherits the base template, not the three combat-role backgrounds.
fn leader_button(view: &View) -> Element {
    let name = "LFDQueueFrameRoleButtonLeader";
    let disabled = !view.leader_available || view.pending || view.matched();
    let atlas = if disabled {
        "UI-LFG-RoleIcon-Leader-Disabled"
    } else {
        "UI-LFG-RoleIcon-Leader"
    };
    let mut art = named_atlas_texture(format!("{name}Icon"), atlas, (0.0, 0.0, 48.0, 48.0));
    if !disabled {
        art.extend(role_checkbox(name, view.volunteer_leader));
    }
    rsx! { button { name: "LFDQueueFrameRoleButtonLeader", width: 48.0, height: 48.0,
    left: 274.0, top: 46.0, pos_type: "absolute", button_default_skin: false,
    onclick: "follower:leader", disabled, {art} } }
}
fn role_checkbox(name: &str, checked: bool) -> Element {
    let mut art = named_atlas_texture(
        format!("{name}CheckBox"),
        "checkbox-minimal",
        (-3.5, 30.5, 21.0, 20.3),
    );
    if checked {
        art.extend(named_atlas_texture(
            format!("{name}Check"),
            "checkmark-minimal",
            (-3.5, 30.5, 21.0, 20.3),
        ));
    }
    art
}
fn role_button(
    view: &View,
    key: &str,
    action: &str,
    role: GroupRoleSnapshot,
    atlas: &str,
    left: f32,
) -> Element {
    let name = format!("LFDQueueFrameRoleButton{key}");
    let background = format!("{atlas}-Background");
    let mut art = rsx! { texture { name: {DynName(format!("{name}Background"))},
    texture_atlas: background, width: 80.0, height: 80.0, left: -16.0, top: -16.0,
    pos_type: "absolute", draw_layer: "BACKGROUND" } };
    art.extend(named_atlas_texture(
        format!("{name}Icon"),
        atlas,
        (0.0, 0.0, 48.0, 48.0),
    ));
    art.extend(role_checkbox(&name, view.role == role));
    let disabled = view.pending || view.matched();
    let action = format!("follower:role:{action}");
    rsx! { button { name: {DynName(name)}, width: 48.0, height: 48.0, left, top: 46.0,
    pos_type: "absolute", button_default_skin: false, onclick: action, disabled, {art} } }
}
fn dungeon_type(view: &View) -> Element {
    let mut elements = label(
        "LFDQueueFrameTypeDropdownName".into(),
        "Type:",
        (6.0, 116.5, 115.0, 24.0),
        (13.0, NORMAL_FONT_COLOR, "RIGHT"),
    );
    elements.extend(type_dropdown(view.type_open));
    if view.type_open {
        elements.extend(type_menu());
    }
    elements
}
fn type_dropdown(open: bool) -> Element {
    dropdown_button_with_state(
        DropdownButton {
            frame_name: "LFDQueueFrameTypeDropdown",
            label_name: "LFDQueueFrameTypeDropdownText",
            arrow_name: "LFDQueueFrameTypeDropdownArrow",
            text: "Follower Dungeons",
            width: 200.0,
            height: 25.0,
            x: 131.0,
            y: -116.0,
            background_color: "",
            text_color: NORMAL_FONT_COLOR,
            arrow_color: "",
            onclick: Some("follower:type"),
        },
        DropdownState {
            open,
            ..Default::default()
        },
    )
}
fn type_menu() -> Element {
    let item = ContextMenuItem {
        name: "LFDQueueFrameTypeFollower",
        label: "Follower Dungeons",
        action: "follower:type:follower",
    };
    let width = 200.0;
    let rows = menu_element(
        item,
        MenuElementKind::Radio { checked: true },
        true,
        menu_row_rect_with_heading(width, 0, false),
    );
    context_menu_with_rows(
        ContextMenu {
            frame_name: "LFDQueueFrameTypeDropdownMenu",
            title_name: "LFDQueueFrameTypeTitle",
            divider_name: "LFDQueueFrameTypeDivider",
            title: "",
            hidden: false,
            width,
            x: 131.0,
            y: 141.0,
            items: &[item],
        },
        rows,
    )
}
fn description(view: &View) -> String {
    if let Some(error) = view.state.as_ref().and_then(|state| state.error.as_ref()) {
        return error.clone();
    }
    if view.pending {
        return "Waiting for server admission…".into();
    }
    if view.matched() {
        return "Follower party ready.".into();
    }
    match &view.catalog {
        None => "Loading available follower dungeons…".into(),
        Some(catalog) if catalog.entries.is_empty() => {
            "No follower dungeons have prepared instance data.".into()
        }
        Some(_) => "NPC companions fill the remaining tank, healer and damage roles.".into(),
    }
}
fn dungeon_list(shared: &SharedContext, view: &View) -> Element {
    let count = view
        .catalog
        .as_ref()
        .map_or(0, |catalog| catalog.entries.len());
    let height = (count as f32 * ROW_HEIGHT as f32).max(LIST_HEIGHT);
    let geometry = pixel_geometry(LIST_HEIGHT, height, LIST_HEIGHT);
    let offset = geometry.clamp(shared.scroll_first_row(FOLLOWER_LIST));
    let child_top = -(offset as f32);
    let config = scroll_list_attr(&geometry);
    let rows: Element = view
        .catalog
        .iter()
        .flat_map(|catalog| &catalog.entries)
        .enumerate()
        .flat_map(|(index, entry)| dungeon_row(view, entry.id, &entry.name, index))
        .collect();
    let bar = MinimalScrollBar {
        list: FOLLOWER_LIST,
        left: 303.0,
        top: 4.0,
        height: LIST_HEIGHT - 8.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! { r#frame { name: "LFDQueueFrameFollowerScrollBox", width: 317.0, height: LIST_HEIGHT,
        left: 5.0, top: 225.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
        r#frame { name: "LFDQueueFrameFollowerScrollChild", width: 295.0, height,
            left: 0.0, top: child_top, pos_type: "absolute", {rows} }
        {bar.element()}
    } }
}
fn dungeon_row(view: &View, id: u32, title: &str, index: usize) -> Element {
    let name = format!("LFDQueueFrameFollowerDungeon{id}");
    let top = index as f32 * ROW_HEIGHT as f32;
    let enabled = !view.pending && !view.matched();
    let mut art = texture(
        format!("{name}CheckBox"),
        130_755,
        (15.0, 0.0, 20.0, 20.0),
        "1,1,1,1",
    );
    if view.selected == Some(id) {
        art.extend(texture(
            format!("{name}Check"),
            130_751,
            (15.0, 0.0, 20.0, 20.0),
            "1,1,1,1",
        ));
    }
    art.extend(label(
        format!("{name}InstanceName"),
        title,
        (40.0, 0.0, 255.0, 20.0),
        (13.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    crate::selection_list::row(
        name,
        &format!("follower:select:{id}"),
        enabled,
        (0.0, top, 295.0, ROW_HEIGHT as f32),
        (&crate::selection_list::AUTHORED, view.selected == Some(id)),
        art,
    )
}
