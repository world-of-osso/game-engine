//! Retail Mainline Blizzard_MountCollection.lua/.xml, shared by both art skins.
use crate::bank_art::{WHITE, cropped, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::mount_source::readable_source;
use crate::quest_art::{NORMAL_FONT_COLOR, named_atlas_texture, panel_button};
use shared::protocol::mount_journal_types::{AQUATIC, FLYING, GROUND, RIDE_ALONG, SKYRIDING};
use shared::protocol::{MountJournalEntry, MountJournalUpdate};
use std::collections::{BTreeMap, BTreeSet};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "MountJournalSearchBox";
pub const MODEL: &str = "MountJournalModelScene";
const LIST: &str = "MountJournalScrollBox";
const ROW_HEIGHT: f32 = 46.0;
const LIST_HEIGHT: f32 = 478.0;
const FILTER_WIDTH: f32 = 90.0; // MountJournal_InitFilterDropdown.
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
    pub unusable: bool,
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
            unusable: true,
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
    pub fn item_is_known(
        &self,
        source: shared::item_data::ItemDefinitionSource,
        item: u32,
    ) -> bool {
        source == shared::item_data::ItemDefinitionSource::Retail
            && self
                .catalog
                .values()
                .any(|mount| mount.collected && mount.teaching_items.contains(&item))
    }

    pub fn item_label(
        &self,
        source: shared::item_data::ItemDefinitionSource,
        item: u32,
        name: &str,
    ) -> String {
        if self.item_is_known(source, item) {
            format!("{name} (Already known)")
        } else {
            name.to_owned()
        }
    }

    pub fn item_tooltip(
        &self,
        source: shared::item_data::ItemDefinitionSource,
        item: u32,
        mut tooltip: crate::tooltip_presentation::TooltipPresentation,
    ) -> crate::tooltip_presentation::TooltipPresentation {
        if self.item_is_known(source, item) {
            tooltip.lines.insert(
                0,
                crate::tooltip_presentation::TooltipLineState::colored(
                    "Already known",
                    crate::item_tooltip::RED_FONT_COLOR,
                ),
            );
        }
        tooltip
    }

    pub fn receive(&mut self, update: MountJournalUpdate) {
        if update.complete {
            self.catalog.clear();
        }
        self.catalog.extend(
            update
                .mounts
                .into_iter()
                .map(|mount| (mount.mount_id, mount)),
        );
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
                    && (self.unusable || mount.usable)
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
            "mount:search-clear" => self.search.clear(),
            "mount:collected" => self.collected = !self.collected,
            "mount:uncollected" => self.uncollected = !self.uncollected,
            "mount:unusable" => self.unusable = !self.unusable,
            "mount:reset" => {
                self.collected = true;
                self.uncollected = true;
                self.unusable = true;
                self.excluded_types.clear();
                self.excluded_sources.clear();
            }
            "mount:sources:all" => self.excluded_sources.clear(),
            "mount:sources:none" => {
                self.excluded_sources = self
                    .catalog
                    .values()
                    .map(|mount| mount.source_type)
                    .collect();
            }
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
    let mut body = search_box(view);
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
    body.extend(filter_dropdown(view));
    body.extend(filter_menu(view));
    body.extend(context_menu(view));
    crate::collections_component::collections_shell(view.viewport, 0, body)
}
fn search_box(view: &MountJournalView) -> Element {
    crate::search_box_template::search_box(
        "MountJournal",
        &view.search,
        "mount:search-clear",
        (19.0, 69.0, 145.0, 20.0),
    )
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
    crate::selection_list::row(
        prefix,
        &action,
        true,
        (0.0, top, 250.0, ROW_HEIGHT),
        (&crate::selection_list::MOUNT, false),
        children,
    )
}
fn mount_details(view: &MountJournalView) -> Element {
    // Blizzard_MountCollection.xml:397-400,443-466,523-526. LeftInset's bottom
    // is 606-111=495; RightInset bottom is 495+85=580, left is 4+260+20=284.
    let border = crate::ui::screens::auction_house_frame_component::inset_border(
        "MountJournalRightInset",
        (0.0, 0.0, 413.0, 520.0),
    );
    let background = cropped(
        "MountJournalBackground".into(),
        651761,
        "0,0.78515625,0,1",
        (0.0, 0.0, 407.0, 514.0),
    );
    let shadow = mount_shadow_overlay();
    let mut out = rsx! {
        r#frame { name: "MountJournalRightInset", width: 413.0, height: 520.0,
            left: 284.0, top: 60.0, pos_type: "absolute", {border}
            r#frame { name: "MountJournalMountDisplay", width: 407.0, height: 514.0,
                left: 3.0, top: 3.0, pos_type: "absolute", clip_children: true,
                {background}
                r#frame { name: "MountJournalModelScene", width: 407.0, height: 514.0,
                    left: 0.0, top: 0.0, pos_type: "absolute", mouse_enabled: true }
                {shadow}
            }
        }
    };
    out.extend(rsx! {
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
        if mount.collected {
            out.extend(label(
                "MountJournalKnown".into(),
                "Already known",
                (312.0, 258.0, 345.0, 20.0),
                (11.0, NORMAL_FONT_COLOR, "LEFT"),
            ));
        }
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
// SharedUIPanelTemplates.xml:163-224; source files give 64px corners/edges.
fn mount_shadow_overlay() -> Element {
    [
        ("TopLeft", 423810, "0,1,0,1", (0.0, 0.0, 64.0, 64.0)),
        ("TopRight", 423810, "1,0,0,1", (343.0, 0.0, 64.0, 64.0)),
        ("BottomLeft", 423810, "0,1,1,0", (0.0, 450.0, 64.0, 64.0)),
        ("BottomRight", 423810, "1,0,1,0", (343.0, 450.0, 64.0, 64.0)),
        ("Top", 423813, "0,1,0,1", (64.0, 0.0, 279.0, 64.0)),
        ("Bottom", 423809, "0,1,0,1", (64.0, 450.0, 279.0, 64.0)),
        ("Left", 423811, "0,1,0,1", (0.0, 64.0, 64.0, 386.0)),
        ("Right", 423812, "0,1,0,1", (343.0, 64.0, 64.0, 386.0)),
    ]
    .into_iter()
    .flat_map(|(name, fdid, uv, rect)| cropped(format!("MountJournalShadow{name}"), fdid, uv, rect))
    .collect()
}

fn filter_dropdown(view: &MountJournalView) -> Element {
    use crate::menu_primitives::*;
    let changed = !view.collected
        || !view.uncollected
        || !view.unusable
        || !view.excluded_types.is_empty()
        || !view.excluded_sources.is_empty();
    dropdown_button_with_state(
        DropdownButton {
            frame_name: "MountJournalFilterButton",
            label_name: "MountJournalFilterText",
            arrow_name: "MountJournalFilterArrow",
            text: "Filter",
            width: FILTER_WIDTH,
            height: 22.0,
            x: 169.0,
            y: -70.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("mount:filters"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: view.filters_open,
            reset_action: changed.then_some("mount:reset"),
            ..Default::default()
        },
    )
}

fn filter_entries(
    view: &MountJournalView,
) -> Vec<(String, String, crate::menu_primitives::MenuElementKind)> {
    use crate::menu_primitives::MenuElementKind as Kind;
    let checkbox = |checked| Kind::Checkbox { checked };
    let mut entries: Vec<(String, String, Kind)> = if view.submenu.as_deref() == Some("sources") {
        let mut rows = vec![
            ("mount:sources:all".into(), "Check All".into(), Kind::Button),
            (
                "mount:sources:none".into(),
                "Uncheck All".into(),
                Kind::Button,
            ),
        ];
        let sources: BTreeSet<_> = view
            .catalog
            .values()
            .map(|mount| mount.source_type)
            .collect();
        rows.extend(sources.into_iter().map(|source| {
            (
                format!("mount:source:{source}"),
                source_name(source).into(),
                checkbox(!view.excluded_sources.contains(&source)),
            )
        }));
        rows
    } else {
        vec![
            (
                "mount:collected".into(),
                "Collected".into(),
                checkbox(view.collected),
            ),
            (
                "mount:uncollected".into(),
                "Not Collected".into(),
                checkbox(view.uncollected),
            ),
            (
                "mount:unusable".into(),
                "Unusable".into(),
                checkbox(view.unusable),
            ),
            (String::new(), "Mount Type".into(), Kind::Title),
        ]
    };
    if view.submenu.is_none() {
        entries.extend(
            [
                (GROUND, "Ground"),
                (FLYING, "Flying"),
                (AQUATIC, "Aquatic"),
                (SKYRIDING, "Skyriding"),
                (RIDE_ALONG, "Ride Along"),
            ]
            .into_iter()
            .filter(|(kind, _)| {
                view.catalog
                    .values()
                    .any(|mount| mount.journal_types & kind != 0)
            })
            .map(|(kind, name)| {
                (
                    format!("mount:type:{kind}"),
                    name.into(),
                    checkbox(!view.excluded_types.contains(&kind)),
                )
            }),
        );
        entries.push(("mount:sources".into(), "Sources".into(), Kind::Submenu));
    }
    entries
}

fn filter_menu(view: &MountJournalView) -> Element {
    if !view.filters_open {
        return Vec::new();
    }
    use crate::menu_primitives::{ContextMenuItem, MenuElementKind, context_menu_description};
    let entries = filter_entries(view);
    let names: Vec<_> = (0..entries.len())
        .map(|index| format!("MountJournalFilter{index}"))
        .collect();
    let mut descriptions = Vec::new();
    for (index, ((action, label, kind), name)) in entries.iter().zip(&names).enumerate() {
        // Keep public row identities stable around the source's ten-pixel spacer.
        if index == 3 && view.submenu.is_none() {
            descriptions.push((
                ContextMenuItem {
                    name: "MountJournalFilterSpacer",
                    label: "",
                    action: "",
                },
                MenuElementKind::Spacer,
                true,
            ));
        }
        descriptions.push((
            ContextMenuItem {
                name,
                label,
                action,
            },
            *kind,
            true,
        ));
    }
    context_menu_description(
        "MountJournalFilterMenu",
        (169.0, 92.0),
        FILTER_WIDTH,
        &descriptions,
    )
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
    use crate::menu_primitives::{ContextMenuItem, MenuElementKind, context_menu_description};
    context_menu_description(
        "MountJournalContextMenu",
        (left, top),
        0.0,
        &[
            (
                ContextMenuItem {
                    name: "MountJournalContextSummon",
                    label: mount_text,
                    action: "mount:summon",
                },
                MenuElementKind::Button,
                mount.active || mount.usable,
            ),
            (
                ContextMenuItem {
                    name: "MountJournalFavoriteToggle",
                    label: favourite_text,
                    action: "mount:favourite",
                },
                MenuElementKind::Button,
                mount.collected,
            ),
        ],
    )
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
