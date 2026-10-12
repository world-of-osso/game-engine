//! Retail Mainline Blizzard_MountCollection.lua/.xml, shared by both art skins.
use crate::bank_art::{WHITE, cropped, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::mount_source::readable_source;
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture, panel_button};
use shared::protocol::mount_journal_types::{AQUATIC, FLYING, GROUND, RIDE_ALONG, SKYRIDING};
use shared::protocol::{MountJournalEntry, MountJournalUpdate};
use std::collections::{BTreeMap, BTreeSet};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "MountJournalSearchBox";
pub const MODEL: &str = "MountJournalModelScene";
const LIST: &str = "MountJournalScrollBox";
const ROW_HEIGHT: f32 = 46.0;
const LIST_HEIGHT: f32 = 478.0;
// TrinityCore a352b1fa DBCEnums.h MountFlags: ServerOnly and ExcludeUntilCollected.
const SERVER_ONLY: u32 = 0x1;
const EXCLUDE_UNTIL_COLLECTED: u32 = 0x40;

#[derive(Clone, Debug, PartialEq)]
pub struct MountJournalView {
    pub visible: bool,
    pub viewport: [f32; 2],
    pub catalog: BTreeMap<u32, MountJournalEntry>,
    pub selected: Option<u32>,
    pub search: String,
    pub collected: bool,
    pub uncollected: bool,
    pub excluded_types: BTreeSet<u32>,
    pub excluded_sources: BTreeSet<i32>,
    pub filters_open: bool,
    pub submenu: Option<String>,
    pub error: String,
    pub context: Option<(u32, [f32; 2])>,
}
impl Default for MountJournalView {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: [1280.0, 720.0],
            catalog: BTreeMap::new(),
            selected: None,
            search: String::new(),
            collected: true,
            uncollected: true,
            excluded_types: BTreeSet::new(),
            excluded_sources: BTreeSet::new(),
            filters_open: false,
            submenu: None,
            error: String::new(),
            context: None,
        }
    }
}
impl MountJournalView {
    pub fn receive(&mut self, update: MountJournalUpdate) {
        self.catalog = update
            .mounts
            .into_iter()
            .map(|mount| (mount.mount_id, mount))
            .collect();
        self.error = update.error.unwrap_or_default();
        self.reselect();
    }
    pub fn rows(&self) -> Vec<&MountJournalEntry> {
        let search = self.search.to_lowercase();
        let mut rows: Vec<_> = self
            .catalog
            .values()
            .filter(|mount| {
                let owned = if mount.collected {
                    self.collected
                } else {
                    self.uncollected
                };
                let matches_type = self.excluded_types.is_empty()
                    || [GROUND, FLYING, AQUATIC, SKYRIDING, RIDE_ALONG]
                        .into_iter()
                        .any(|kind| {
                            mount.journal_types & kind != 0 && !self.excluded_types.contains(&kind)
                        });
                let shown = mount.flags & SERVER_ONLY == 0
                    && (mount.collected || mount.flags & EXCLUDE_UNTIL_COLLECTED == 0);
                owned
                    && shown
                    && matches_type
                    && !self.excluded_sources.contains(&mount.source_type)
                    && (search.is_empty() || mount.name.to_lowercase().contains(&search))
            })
            .collect();
        rows.sort_by_key(|mount| {
            (
                !mount.favourite,
                !mount.collected,
                mount.name.to_lowercase(),
                mount.mount_id,
            )
        });
        rows
    }
    pub fn selected_mount(&self) -> Option<&MountJournalEntry> {
        self.selected.and_then(|id| self.catalog.get(&id))
    }
    pub fn reselect(&mut self) {
        let rows = self.rows();
        if !rows
            .iter()
            .any(|mount| Some(mount.mount_id) == self.selected)
        {
            self.selected = rows.first().map(|mount| mount.mount_id);
        }
    }
    pub fn apply_filter(&mut self, action: &str) -> bool {
        match action {
            "mount:filters" => {
                self.filters_open = !self.filters_open;
                self.submenu = None;
            }
            "mount:collected" => self.collected = !self.collected,
            "mount:uncollected" => self.uncollected = !self.uncollected,
            "mount:types" => self.submenu = Some("types".into()),
            "mount:sources" => self.submenu = Some("sources".into()),
            _ => {
                if let Some(kind) = action
                    .strip_prefix("mount:type:")
                    .and_then(|id| id.parse().ok())
                {
                    toggle(&mut self.excluded_types, kind);
                } else if let Some(source) = action
                    .strip_prefix("mount:source:")
                    .and_then(|id| id.parse().ok())
                {
                    toggle(&mut self.excluded_sources, source);
                } else {
                    return false;
                }
            }
        }
        self.reselect();
        true
    }
}
fn toggle<T: Ord>(values: &mut BTreeSet<T>, value: T) {
    if !values.remove(&value) {
        values.insert(value);
    }
}

pub fn mount_journal_screen(ctx: &SharedContext) -> Element {
    let view = ctx.get::<MountJournalView>().expect("Mount Journal view");
    if !view.visible {
        return Vec::new();
    }
    let mut body = search_box();
    body.extend(label(
        "MountJournalCount".into(),
        &format!(
            "Total Mounts: {}",
            view.catalog
                .values()
                .filter(|mount| mount.collected)
                .count()
        ),
        (70.0, 35.0, 230.0, 20.0),
        (12.0, WHITE, "LEFT"),
    ));
    body.extend(mount_list(ctx, view));
    body.extend(mount_details(view));
    body.extend(filter_menu(view));
    body.extend(context_menu(view));
    crate::collections_component::collections_shell(view.viewport, 0, body)
}
fn search_box() -> Element {
    rsx! { editbox { name: "MountJournalSearchBox", width: 145.0, height: 20.0,
        left: 19.0, top: 69.0, pos_type: "absolute", font_size: 12.0,
        font_color: WHITE, text_insets: "4,4,0,0", max_letters: 40 }
        button { name: "MountJournalFilterButton", width: 78.0, height: 20.0,
        left: 182.0, top: 69.0, pos_type: "absolute", text: "Filter", onclick: "mount:filters", font_size: 12.0 }
    }
}
fn mount_list(ctx: &SharedContext, view: &MountJournalView) -> Element {
    let rows = view.rows();
    let height = (rows.len() as f32 * ROW_HEIGHT).max(LIST_HEIGHT);
    let geometry = pixel_geometry(LIST_HEIGHT, height, LIST_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let first = offset as usize / ROW_HEIGHT as usize;
    let content: Element = rows
        .iter()
        .enumerate()
        .skip(first)
        .take(12)
        .flat_map(|(index, mount)| mount_row(view, mount, index as f32 * ROW_HEIGHT))
        .collect();
    let bar = MinimalScrollBar {
        list: LIST,
        left: 257.0,
        top: 0.0,
        height: LIST_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! { r#frame { name: "MountJournalScrollBox", width: 260.0, height: LIST_HEIGHT, left: 7.0, top: 96.0,
        pos_type: "absolute", mouse_enabled: true, scroll_list: config,
        r#frame { name: "MountJournalScrollChild", width: 250.0, height, left: 0.0, top: child_top, pos_type: "absolute", {content} }
        {bar.element()} }
    }
}
fn mount_row(view: &MountJournalView, mount: &MountJournalEntry, top: f32) -> Element {
    let prefix = format!("MountJournalRow{}", mount.mount_id);
    let action = format!("mount:select:{}", mount.mount_id);
    let color = if mount.collected {
        NORMAL_FONT_COLOR
    } else {
        "0.5,0.5,0.5,1.0"
    };
    let mut children = named_atlas_texture(
        format!("{prefix}Background"),
        "PetList-ButtonBackground",
        (42.0, 0.0, 208.0, 46.0),
    );
    children.extend(texture(
        format!("{prefix}Icon"),
        mount.icon_file_data_id,
        (3.0, 4.0, 38.0, 38.0),
        WHITE,
    ));
    children.extend(label(
        format!("{prefix}Name"),
        &mount.name,
        (50.0, 4.0, 147.0, 38.0),
        (12.0, color, "LEFT"),
    ));
    if mount.favourite {
        children.extend(named_atlas_texture(
            format!("{prefix}Favorite"),
            "PetJournal-FavoritesIcon",
            (0.0, 0.0, 20.0, 20.0),
        ));
    }
    if view.selected == Some(mount.mount_id) {
        children.extend(named_atlas_texture(
            format!("{prefix}Selected"),
            "PetList-ButtonSelect",
            (42.0, 0.0, 208.0, 46.0),
        ));
    }
    rsx! { button { name: {DynName(prefix)}, width: 250.0, height: ROW_HEIGHT, left: 0.0, top,
    pos_type: "absolute", button_default_skin: false, onclick: {action.as_str()}, {children} } }
}
fn mount_details(view: &MountJournalView) -> Element {
    // Blizzard_MountCollection.xml MountDisplay.YesMountsTex, local listfile FDID651761.
    let mut out = cropped(
        "MountJournalBackground".into(),
        651761,
        "0,0.78515625,0,1",
        (292.0, 64.0, 405.0, 510.0),
    );
    out.extend(rsx! { r#frame { name: "MountJournalModelScene", width: 405.0, height: 365.0,
        left: 292.0, top: 120.0, pos_type: "absolute", mouse_enabled: true }
        button { name: "MountJournalSummonRandomFavorite", width: 206.0, height: 33.0,
        left: 484.0, top: 33.0, pos_type: "absolute", text: "Summon Random Favorite Mount", font_size: 10.0, onclick: "mount:random" }
    });
    if let Some(mount) = view.selected_mount() {
        out.extend(texture(
            "MountJournalSelectedIcon".into(),
            mount.icon_file_data_id,
            (312.0, 82.0, 38.0, 38.0),
            WHITE,
        ));
        out.extend(label(
            "MountJournalName".into(),
            &mount.name,
            (360.0, 83.0, 270.0, 35.0),
            (16.0, WHITE, "LEFT"),
        ));
        out.extend(label(
            "MountJournalDescription".into(),
            &mount.description,
            (312.0, 197.0, 345.0, 55.0),
            (12.0, NORMAL_FONT_COLOR, "LEFT"),
        ));
        out.extend(label(
            "MountJournalSource".into(),
            &readable_source(&mount.source_text),
            (312.0, 130.0, 345.0, 65.0),
            (11.0, WHITE, "LEFT"),
        ));
        let text = if mount.active { "Dismount" } else { "Mount" };
        out.extend(panel_button(
            "MountJournalMountButton".into(),
            text,
            "mount:summon",
            mount.active || mount.usable,
            (487.0, 578.0, 140.0, 22.0),
        ));
    } else {
        out.extend(label(
            "MountJournalEmpty".into(),
            "No mounts match your filters.",
            (310.0, 240.0, 360.0, 40.0),
            (12.0, WHITE, "CENTER"),
        ));
    }
    out.extend(label(
        "MountJournalError".into(),
        &view.error,
        (292.0, 65.0, 405.0, 20.0),
        (11.0, "1.0,0.15,0.15,1.0", "CENTER"),
    ));
    out
}
fn filter_menu(view: &MountJournalView) -> Element {
    if !view.filters_open {
        return Vec::new();
    }
    let entries: Vec<(String, String, bool)> = match view.submenu.as_deref() {
        Some("types") => [
            (GROUND, "Ground"),
            (FLYING, "Flying"),
            (AQUATIC, "Aquatic"),
            (SKYRIDING, "Skyriding"),
            (RIDE_ALONG, "Ride Along"),
        ]
        .into_iter()
        .map(|(kind, name)| {
            (
                format!("mount:type:{kind}"),
                name.into(),
                !view.excluded_types.contains(&kind),
            )
        })
        .collect(),
        Some("sources") => view
            .catalog
            .values()
            .map(|mount| mount.source_type)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|source| {
                (
                    format!("mount:source:{source}"),
                    source_name(source).into(),
                    !view.excluded_sources.contains(&source),
                )
            })
            .collect(),
        _ => vec![
            ("mount:collected".into(), "Collected".into(), view.collected),
            (
                "mount:uncollected".into(),
                "Not Collected".into(),
                view.uncollected,
            ),
            ("mount:types".into(), "Mount Type >".into(), false),
            ("mount:sources".into(), "Sources >".into(), false),
        ],
    };
    let children: Element=entries.iter().enumerate().flat_map(|(index,(action,title,checked))| {
        let name=DynName(format!("MountJournalFilter{index}"));
        let text=format!("{} {title}",if *checked { "✓" } else { "  " });
        let top=5.0+index as f32*22.0;
        rsx! { button { name, width: 190.0,height: 22.0,left: 5.0,top,pos_type: "absolute",text: {text.as_str()},onclick: {action.as_str()},font_size: 12.0 } }
    }).collect();
    let height = 10.0 + entries.len() as f32 * 22.0;
    rsx! { r#frame { name: "MountJournalFilterMenu", width: 200.0,height,left: 75.0,top: 91.0,pos_type: "absolute",
    strata: crate::ui::strata::FrameStrata::FullscreenDialog,mouse_enabled: true,background_color: "0.05,0.04,0.03,1.0",{children} } }
}
fn context_menu(view: &MountJournalView) -> Element {
    let Some((id, [left, top])) = view.context else {
        return Vec::new();
    };
    let Some(mount) = view.catalog.get(&id) else {
        return Vec::new();
    };
    let mount_text = if mount.active { "Dismount" } else { "Mount" };
    let favourite_text = if mount.favourite {
        "Remove Favorite"
    } else {
        "Set Favorite"
    };
    let mut children = panel_button(
        "MountJournalContextSummon".into(),
        mount_text,
        "mount:summon",
        mount.active || mount.usable,
        (5.0, 5.0, 180.0, 22.0),
    );
    children.extend(panel_button(
        "MountJournalFavoriteToggle".into(),
        favourite_text,
        "mount:favourite",
        mount.collected,
        (5.0, 29.0, 180.0, 22.0),
    ));
    children.extend(panel_button(
        "MountJournalContextCancel".into(),
        "Cancel",
        "mount:context-close",
        true,
        (5.0, 53.0, 180.0, 22.0),
    ));
    rsx! { r#frame { name: "MountJournalContextMenu",width: 190.0,height: 80.0,left,top,pos_type: "absolute",
    strata: crate::ui::strata::FrameStrata::FullscreenDialog,mouse_enabled: true,background_color: "0.05,0.04,0.03,1.0",{children} } }
}

fn source_name(source: i32) -> &'static str {
    match source {
        0 => "Drop",
        1 => "Quest",
        2 => "Vendor",
        3 => "Profession",
        4 => "Dungeon/Raid",
        5 => "Achievement",
        6 => "World Event",
        7 => "Promotion",
        8 => "Store",
        9 => "Discovery",
        _ => "Other",
    }
}
