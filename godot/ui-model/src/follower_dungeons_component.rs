//! LFDFrame.xml: follower roles, type dropdown, explanation, ScrollBox and Find Group.
use crate::bank_art::{WHITE, label, texture};
use crate::follower_dungeons::View;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{
    DynName, NORMAL_FONT_COLOR, named_atlas_texture, panel_button, window_chrome,
};
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
    children.extend(categories());
    children.extend(queue_panel(shared, &view));
    children.extend(panel_button(
        "PVEFrameTab1".into(),
        "Group Finder",
        "follower:type:follower",
        true,
        (19.0, 428.0, 120.0, 28.0),
    ));
    children.extend(panel_button(
        "PVEFrameTab2".into(),
        "Player vs. Player",
        "follower:pvp",
        true,
        (139.0, 428.0, 140.0, 28.0),
    ));
    rsx! { r#frame { name: "PVEFrame", width: 563.0, height: 456.0, left: 100.0, top: 84.0,
    pos_type: "absolute", strata: FrameStrata::Dialog, mouse_enabled: true, {children} } }
}
fn categories() -> Element {
    [
        ("Dungeon Finder", true),
        ("Raid Finder", false),
        ("Premade Groups", false),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (title, enabled))| {
        crate::selection_list::text_row(
            format!("GroupFinderFrameGroupButton{}", index + 1),
            title,
            "follower:type:follower",
            (enabled, index == 0),
            (10.0, 101.0 + index as f32 * 90.0, 203.0, 60.0),
            &crate::selection_list::PVP_CATEGORY,
        )
    })
    .collect()
}
fn queue_panel(shared: &SharedContext, view: &View) -> Element {
    let mut children = texture(
        "LFDQueueFrameBackground".into(),
        337_489,
        (6.0, 26.0, 326.0, 376.0),
        "1,1,1,1",
    );
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
fn roles(view: &View) -> Element {
    [
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
    .collect()
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
    let mut art = named_atlas_texture(format!("{name}Icon"), atlas, (0.0, 0.0, 48.0, 48.0));
    art.extend(named_atlas_texture(
        format!("{name}CheckBox"),
        "checkbox-minimal",
        (-3.5, 30.5, 21.0, 20.3),
    ));
    if view.role == role {
        art.extend(named_atlas_texture(
            format!("{name}Check"),
            "checkmark-minimal",
            (-3.5, 30.5, 21.0, 20.3),
        ));
    }
    let disabled = view.pending || view.matched();
    let action = format!("follower:role:{action}");
    rsx! { button { name: {DynName(name)}, width: 48.0, height: 48.0, left, top: 46.0,
    pos_type: "absolute", button_default_skin: false, onclick: action, disabled, {art} } }
}
fn dungeon_type(view: &View) -> Element {
    let mut elements = label(
        "LFDQueueFrameTypeDropdownName".into(),
        "Type:",
        (15.0, 117.0, 100.0, 24.0),
        (13.0, NORMAL_FONT_COLOR, "RIGHT"),
    );
    elements.extend(panel_button(
        "LFDQueueFrameTypeDropdown".into(),
        "Follower Dungeons",
        "follower:type",
        true,
        (131.0, 117.0, 200.0, 24.0),
    ));
    if view.type_open {
        let mut menu = named_atlas_texture(
            "LFDQueueFrameTypeDropdownMenuBackground".into(),
            "common-dropdown-bg",
            (0.0, 0.0, 200.0, 32.0),
        );
        menu.extend(crate::selection_list::text_row(
            "LFDQueueFrameTypeFollower".into(),
            "Follower Dungeons",
            "follower:type:follower",
            (true, true),
            (4.0, 4.0, 192.0, 24.0),
            &crate::selection_list::MENU,
        ));
        elements.extend(
            rsx! { r#frame { name: "LFDQueueFrameTypeDropdownMenu", width: 200.0, height: 32.0,
            left: 131.0, top: 141.0, pos_type: "absolute", strata: FrameStrata::Tooltip, {menu} } },
        );
    }
    elements
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
