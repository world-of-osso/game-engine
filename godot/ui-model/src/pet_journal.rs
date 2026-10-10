//! Retail Blizzard_Collections PetJournal: 703×606, 260px list, card and three slots.
use crate::bank_art::{WHITE, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome};
use crate::ui::strata::FrameStrata;
use shared::pet_battle::PetJournal;
use shared::protocol::{CollectionPetSnapshot, CollectionStateUpdate};
use std::collections::BTreeMap;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "PetJournalSearchBox";
pub const LIST: &str = "PetJournalScrollBox";
const ROW_HEIGHT: f32 = 46.0;
const LIST_HEIGHT: f32 = 478.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetRowKey {
    Owned(u64),
    Species(u32),
}
impl PetRowKey {
    pub fn action(self) -> String {
        match self {
            Self::Owned(id) => format!("pet:select:owned:{id}"),
            Self::Species(id) => format!("pet:select:species:{id}"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetRequest {
    Summon(u64),
    Dismiss,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetRow {
    pub key: PetRowKey,
    pub species: u32,
    pub name: String,
    pub level: Option<u8>,
    pub detail: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetJournalView {
    pub visible: bool,
    pub catalog: BTreeMap<u32, CollectionPetSnapshot>,
    pub icons: BTreeMap<u32, u32>,
    pub journal: PetJournal,
    pub summoned: Option<u64>,
    pub selected: Option<PetRowKey>,
    pub search: String,
    pub collected_only: bool,
    pub pending: bool,
    pub error: String,
}
impl Default for PetJournalView {
    fn default() -> Self {
        Self {
            visible: false,
            catalog: BTreeMap::new(),
            icons: BTreeMap::new(),
            journal: PetJournal::default(),
            summoned: None,
            selected: None,
            search: String::new(),
            collected_only: false,
            pending: false,
            error: String::new(),
        }
    }
}
impl PetJournalView {
    pub fn receive(&mut self, update: CollectionStateUpdate) {
        if let Some(snapshot) = update.snapshot {
            self.catalog = snapshot
                .pets
                .into_iter()
                .map(|pet| (pet.pet_id, pet))
                .collect();
        }
        if let Some(journal) = update.pet_journal {
            self.journal = journal;
            self.summoned = update.summoned_pet_id;
            self.pending = false;
        }
        self.error = update.error.unwrap_or_default();
        if !self.error.is_empty() {
            self.pending = false;
        }
        self.reselect_visible();
    }
    pub fn rows(&self) -> Vec<PetRow> {
        let mut rows: Vec<_> = self
            .journal
            .pets
            .iter()
            .filter_map(|pet| {
                let species = self.catalog.get(&pet.species_id)?;
                Some(PetRow {
                    key: PetRowKey::Owned(pet.id),
                    species: pet.species_id,
                    name: pet
                        .custom_name
                        .clone()
                        .unwrap_or_else(|| species.name.clone()),
                    level: Some(pet.level),
                    detail: format!(
                        "Level {} · {:?} · Breed {}",
                        pet.level, pet.quality, pet.breed_id
                    ),
                })
            })
            .collect();
        if !self.collected_only {
            rows.extend(
                self.catalog
                    .values()
                    .filter(|species| self.journal.count_species(species.pet_id) == 0)
                    .map(|species| PetRow {
                        key: PetRowKey::Species(species.pet_id),
                        species: species.pet_id,
                        name: species.name.clone(),
                        level: None,
                        detail: "Not Collected".into(),
                    }),
            );
        }
        let query = self.search.to_lowercase();
        rows.retain(|row| row.name.to_lowercase().contains(&query));
        rows.sort_by(|left, right| {
            left.level
                .is_none()
                .cmp(&right.level.is_none())
                .then(left.name.cmp(&right.name))
                .then(left.species.cmp(&right.species))
        });
        rows
    }
    pub fn select(&mut self, key: PetRowKey) {
        if self.rows().iter().any(|row| row.key == key) {
            self.selected = Some(key);
        }
    }
    pub fn set_search(&mut self, search: String) {
        self.search = search;
        self.reselect_visible();
    }
    pub fn reselect_visible(&mut self) {
        let rows = self.rows();
        if !rows.iter().any(|row| Some(row.key) == self.selected) {
            self.selected = rows.first().map(|row| row.key);
        }
    }
    pub fn summon_or_dismiss(&mut self) -> Option<PetRequest> {
        if self.pending {
            return None;
        }
        let PetRowKey::Owned(id) = self.selected? else {
            return None;
        };
        self.journal.get(id)?;
        self.pending = true;
        self.error.clear();
        Some(if self.summoned == Some(id) {
            PetRequest::Dismiss
        } else {
            PetRequest::Summon(id)
        })
    }
}

pub fn pet_journal_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx.get::<PetJournalView>().filter(|view| view.visible) else {
        return vec![];
    };
    let mut children = window_chrome("PetJournal", (703.0, 606.0), "Pet Journal", "pet:close");
    children.extend(label(
        "PetJournalPetCount".into(),
        &format!("Total Pets: {}", view.journal.count()),
        (70.0, 35.0, 160.0, 20.0),
        (12.0, WHITE, "LEFT"),
    ));
    children.extend(search_box());
    children.extend(pet_list(ctx, view));
    children.extend(pet_card(view));
    children.extend(battle_slots());
    children.extend(journal_buttons(view));
    rsx! { r#frame { name: "PetJournal", width: 703.0, height: 606.0,
    left: 30.0, top: 86.0, pos_type: "absolute", mouse_enabled: true,
    strata: FrameStrata::Dialog, {children} } }
}
fn search_box() -> Element {
    rsx! { editbox { name: "PetJournalSearchBox", width: 145.0, height: 20.0,
    left: 19.0, top: 69.0, pos_type: "absolute", font_size: 12.0,
    font_color: WHITE, text_insets: "4,4,0,0", max_letters: 40 }
    button { name: "PetJournalFilterButton", width: 78.0, height: 20.0,
    left: 182.0, top: 69.0, pos_type: "absolute", text: "Collected",
    onclick: "pet:filter", font_size: 12.0 } }
}
fn pet_list(ctx: &SharedContext, view: &PetJournalView) -> Element {
    let rows = view.rows();
    let height = (rows.len() as f32 * ROW_HEIGHT).max(LIST_HEIGHT);
    let geometry = pixel_geometry(LIST_HEIGHT, height, LIST_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: LIST,
        left: 257.0,
        top: 0.0,
        height: LIST_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let first = offset as usize / ROW_HEIGHT as usize;
    let content: Element = rows
        .iter()
        .enumerate()
        .skip(first)
        .take(13)
        .flat_map(|(index, row)| pet_row(view, row, index as f32 * ROW_HEIGHT))
        .collect();
    rsx! { r#frame { name: "PetJournalScrollBox", width: 260.0, height: LIST_HEIGHT,
    left: 7.0, top: 96.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
    r#frame { name: "PetJournalScrollChild", width: 250.0, height,
        left: 0.0, top: {-offset as f32}, pos_type: "absolute", {content} }
    {bar.element()} } }
}
fn pet_row(view: &PetJournalView, row: &PetRow, top: f32) -> Element {
    let action = row.key.action();
    let prefix = action.replace(':', "_");
    let mut children = label(
        format!("{prefix}Name"),
        &row.name,
        (43.0, 3.0, 203.0, 20.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    );
    children.extend(label(
        format!("{prefix}Detail"),
        &row.detail,
        (43.0, 24.0, 203.0, 16.0),
        (10.0, WHITE, "LEFT"),
    ));
    if let Some(icon) = view.icons.get(&row.species) {
        children.extend(texture(
            format!("{prefix}Icon"),
            *icon,
            (3.0, 5.0, 36.0, 36.0),
            WHITE,
        ));
    }
    let selected = view.selected == Some(row.key);
    let color = if selected {
        "1.0,0.82,0.0,0.22"
    } else {
        "0.0,0.0,0.0,0.25"
    };
    rsx! { r#frame { name: {DynName(prefix)}, width: 250.0, height: ROW_HEIGHT,
    left: 0.0, top, pos_type: "absolute", mouse_enabled: true, onclick: {action.as_str()},
    texture { width: 250.0, height: ROW_HEIGHT, color, pos_type: "absolute", left: 0.0, top: 0.0 }
    {children} } }
}
fn pet_card(view: &PetJournalView) -> Element {
    let mut out = rsx! { texture { name: "PetJournalPetCardBackground", texture_atlas: "PetJournal-PetCard-BG",
    width: 405.0, height: 168.0, left: 292.0, top: 60.0, pos_type: "absolute" } };
    let selected = view
        .rows()
        .into_iter()
        .find(|row| Some(row.key) == view.selected);
    if let Some(row) = selected {
        out.extend(label(
            "PetJournalPetCardName".into(),
            &row.name,
            (342.0, 74.0, 340.0, 24.0),
            (16.0, NORMAL_FONT_COLOR, "LEFT"),
        ));
        out.extend(label(
            "PetJournalPetCardDetails".into(),
            &row.detail,
            (306.0, 132.0, 370.0, 30.0),
            (12.0, WHITE, "LEFT"),
        ));
        if let Some(icon) = view.icons.get(&row.species) {
            out.extend(texture(
                "PetJournalPetCardIcon".into(),
                *icon,
                (300.0, 70.0, 38.0, 38.0),
                WHITE,
            ));
        }
    }
    out.extend(label(
        "PetJournalStatus".into(),
        &view.error,
        (298.0, 185.0, 390.0, 36.0),
        (12.0, "1,0.1,0.1,1", "LEFT"),
    ));
    out
}
fn battle_slots() -> Element {
    let mut out = Vec::new();
    for slot in 0..3 {
        let top = 279.0 + slot as f32 * 100.0;
        out.extend(
            rsx! { texture { name: {DynName(format!("PetJournalBattleSlot{slot}"))},
            texture_atlas: "PetJournal-BattleSlot-Locked", width: 405.0, height: 92.0,
            left: 292.0, top, pos_type: "absolute" } },
        );
    }
    out
}
fn journal_buttons(view: &PetJournalView) -> Element {
    let owned = matches!(view.selected, Some(PetRowKey::Owned(_)));
    let active = matches!(view.selected, Some(PetRowKey::Owned(id)) if view.summoned == Some(id));
    let text = if active { "Dismiss" } else { "Summon" };
    let mut out = panel_button(
        "PetJournalSummonButton".into(),
        text,
        "pet:summon",
        owned && !view.pending,
        (0.0, 584.0, 160.0, 22.0),
    );
    out.extend(panel_button(
        "PetJournalFindBattle".into(),
        "Find Battle",
        "",
        false,
        (563.0, 584.0, 140.0, 22.0),
    ));
    out.extend(panel_button(
        "PetJournalPetsTab".into(),
        "Pet Journal",
        "",
        false,
        (105.0, 606.0, 110.0, 30.0),
    ));
    out
}
