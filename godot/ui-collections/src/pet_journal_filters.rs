//! Blizzard_PetCollection.lua:118-214: checkbox groups and sorting submenus.
use super::{PetJournalView, PetRow};
use game_engine_ui_model::menu_primitives::*;
use std::collections::BTreeSet;
use ui_toolkit::widget_def::Element;

const FILTER_WIDTH: f32 = 90.0; // PetJournal_InitFilterDropdown.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PetSpeciesFilter {
    pub flags: u32,
    pub source: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PetSort {
    #[default]
    Name,
    Level,
    Rarity,
    Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PetFilters {
    pub collected: bool,
    pub uncollected: bool,
    pub battle: bool,
    pub non_combat: bool,
    pub excluded_families: BTreeSet<u8>,
    pub excluded_sources: BTreeSet<i32>,
    pub sort: PetSort,
    pub open: bool,
    pub submenu: Option<String>,
}
impl Default for PetFilters {
    fn default() -> Self {
        Self {
            collected: true,
            uncollected: true,
            battle: true,
            non_combat: true,
            excluded_families: BTreeSet::new(),
            excluded_sources: BTreeSet::new(),
            sort: PetSort::Name,
            open: false,
            submenu: None,
        }
    }
}
impl PetFilters {
    pub fn matches(&self, view: &PetJournalView, row: &PetRow) -> bool {
        if let Some(visual) = view.visuals.get(&row.species)
            && self.excluded_families.contains(&visual.family)
        {
            return false;
        }
        if let Some(meta) = view.species_filters.get(&row.species) {
            // TrinityCore a352b1fa BattlePetSpeciesFlags: HideFromJournal and HideUntilLearned.
            let owned = matches!(row.key, super::PetRowKey::Owned(_));
            let hidden = meta.flags & 0x20 != 0 || (!owned && meta.flags & 0x4000 != 0);
            if hidden {
                return false;
            }
            // TrinityCore DBCEnums.h BattlePetSpeciesFlags::CantBattle = 0x80.
            let combat = meta.flags & 0x80 == 0;
            let category = if combat { self.battle } else { self.non_combat };
            return category && !self.excluded_sources.contains(&meta.source);
        }
        self.battle && self.non_combat && self.excluded_sources.is_empty()
    }
    pub fn changed(&self) -> bool {
        !self.collected
            || !self.uncollected
            || !self.battle
            || !self.non_combat
            || !self.excluded_families.is_empty()
            || !self.excluded_sources.is_empty()
            || self.sort != PetSort::Name
    }
}
impl PetJournalView {
    pub fn apply_filter(&mut self, action: &str) -> bool {
        let filters = &mut self.filters;
        match action {
            "pet:filters" => {
                filters.open = !filters.open;
                filters.submenu = None;
            }
            "pet:collected" => filters.collected = !filters.collected,
            "pet:uncollected" => filters.uncollected = !filters.uncollected,
            "pet:battle" => filters.battle = !filters.battle,
            "pet:non-combat" => filters.non_combat = !filters.non_combat,
            "pet:families" | "pet:sources" | "pet:sort" => {
                filters.submenu = Some(action[4..].into())
            }
            "pet:families:all" => filters.excluded_families.clear(),
            "pet:families:none" => filters.excluded_families = (0..10).collect(),
            "pet:sources:all" => filters.excluded_sources.clear(),
            "pet:sources:none" => {
                filters.excluded_sources = self
                    .species_filters
                    .values()
                    .map(|meta| meta.source)
                    .collect()
            }
            "pet:sort:name" => filters.sort = PetSort::Name,
            "pet:sort:level" => filters.sort = PetSort::Level,
            "pet:sort:rarity" => filters.sort = PetSort::Rarity,
            "pet:sort:type" => filters.sort = PetSort::Type,
            "pet:reset" => {
                let open = filters.open;
                *filters = PetFilters::default();
                filters.open = open;
            }
            _ => {
                if let Some(family) = action
                    .strip_prefix("pet:family:")
                    .and_then(|id| id.parse().ok())
                {
                    toggle(&mut filters.excluded_families, family);
                } else if let Some(source) = action
                    .strip_prefix("pet:source:")
                    .and_then(|id| id.parse().ok())
                {
                    toggle(&mut filters.excluded_sources, source);
                } else {
                    return false;
                }
            }
        }
        self.reselect_visible();
        true
    }
}
fn toggle<T: Ord>(values: &mut BTreeSet<T>, value: T) {
    if !values.remove(&value) {
        values.insert(value);
    }
}

pub(super) fn controls(view: &PetJournalView) -> Element {
    let mut out = dropdown_button_with_state(
        DropdownButton {
            frame_name: "PetJournalFilterButton",
            label_name: "PetJournalFilterText",
            arrow_name: "PetJournalFilterArrow",
            text: "Filter",
            width: FILTER_WIDTH,
            height: 22.0,
            x: 169.0,
            y: -70.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("pet:filters"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: view.filters.open,
            reset_action: view.filters.changed().then_some("pet:reset"),
            ..Default::default()
        },
    );
    if view.filters.open {
        out.extend(menu(view));
    }
    out
}

type Row = (String, String, MenuElementKind);
fn checkbox(label: &str, action: &str, checked: bool) -> Row {
    (
        label.into(),
        action.into(),
        MenuElementKind::Checkbox { checked },
    )
}
fn button(label: &str, action: &str) -> Row {
    (label.into(), action.into(), MenuElementKind::Button)
}
fn rows(view: &PetJournalView) -> Vec<Row> {
    let f = &view.filters;
    match f.submenu.as_deref() {
        Some("families") => {
            let mut rows = vec![
                button("Check All", "pet:families:all"),
                button("Uncheck All", "pet:families:none"),
            ];
            rows.extend(
                [
                    "Humanoid",
                    "Dragonkin",
                    "Flying",
                    "Undead",
                    "Critter",
                    "Magic",
                    "Elemental",
                    "Beast",
                    "Aquatic",
                    "Mechanical",
                ]
                .into_iter()
                .enumerate()
                .map(|(family, name)| {
                    checkbox(
                        name,
                        &format!("pet:family:{family}"),
                        !f.excluded_families.contains(&(family as u8)),
                    )
                }),
            );
            rows
        }
        Some("sources") => source_rows(view),
        Some("sort") => [
            ("Name", "name", PetSort::Name),
            ("Level", "level", PetSort::Level),
            ("Rarity", "rarity", PetSort::Rarity),
            ("Type", "type", PetSort::Type),
        ]
        .into_iter()
        .map(|(label, action, sort)| {
            (
                label.into(),
                format!("pet:sort:{action}"),
                MenuElementKind::Radio {
                    checked: f.sort == sort,
                },
            )
        })
        .collect(),
        _ => vec![
            checkbox("Collected", "pet:collected", f.collected),
            checkbox("Not Collected", "pet:uncollected", f.uncollected),
            (String::new(), String::new(), MenuElementKind::Divider),
            ("Pet Types".into(), String::new(), MenuElementKind::Title),
            checkbox("Battle Pets", "pet:battle", f.battle),
            checkbox("Non-Combat Pets", "pet:non-combat", f.non_combat),
            (
                "Pet Families".into(),
                "pet:families".into(),
                MenuElementKind::Submenu,
            ),
            (
                "Sources".into(),
                "pet:sources".into(),
                MenuElementKind::Submenu,
            ),
            (
                "Sort By".into(),
                "pet:sort".into(),
                MenuElementKind::Submenu,
            ),
        ],
    }
}
fn source_rows(view: &PetJournalView) -> Vec<Row> {
    let mut rows = vec![
        button("Check All", "pet:sources:all"),
        button("Uncheck All", "pet:sources:none"),
    ];
    let sources: BTreeSet<_> = view
        .species_filters
        .values()
        .map(|meta| meta.source)
        // Retail enumerates BattlePetSources; -1 is a species' no-source sentinel, not a source option.
        .filter(|source| *source >= 0)
        .collect();
    rows.extend(sources.into_iter().map(|source| {
        let label = [
            "Drop",
            "Quest",
            "Vendor",
            "Profession",
            "Pet Battle",
            "Achievement",
            "World Event",
            "Promotion",
            "Trading Card Game",
            "Pet Store",
            "Discovery",
            "Trading Post",
        ]
        .get(source as usize)
        .expect("BattlePetSpecies SourceTypeEnum");
        checkbox(
            label,
            &format!("pet:source:{source}"),
            !view.filters.excluded_sources.contains(&source),
        )
    }));
    rows
}
fn menu(view: &PetJournalView) -> Element {
    let rows = rows(view);
    let names: Vec<_> = (0..rows.len())
        .map(|index| format!("PetJournalFilter{index}"))
        .collect();
    let entries: Vec<_> = rows
        .iter()
        .zip(&names)
        .map(|((label, action, kind), name)| {
            (
                ContextMenuItem {
                    name,
                    label,
                    action,
                },
                *kind,
                true,
            )
        })
        .collect();
    context_menu_description(
        "PetJournalFilterMenu",
        (169.0, 92.0),
        FILTER_WIDTH,
        &entries,
    )
}
