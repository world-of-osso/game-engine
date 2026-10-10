//! Retail Blizzard_Collections PetJournal: 703×606, 260px list, card and three slots.
use crate::bank_art::{WHITE, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button};
use shared::pet_battle::PetJournal;
use shared::protocol::{CollectionPetSnapshot, CollectionStateUpdate};
use std::collections::{BTreeMap, BTreeSet};
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct PetSpeciesVisual {
    pub species_id: u32,
    pub display_id: u32,
    pub family: u8,
}
impl PetSpeciesVisual {
    pub fn type_icon(&self) -> u32 {
        // DB2 PetType is zero-based; Retail GetPetTypeTexture uses the same families.
        [
            603593, 603590, 603592, 603596, 603589, 603594, 603591, 603588, 603597, 603595,
        ][usize::from(self.family)]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PetJournalView {
    pub visible: bool,
    pub viewport: [f32; 2],
    pub catalog: BTreeMap<u32, CollectionPetSnapshot>,
    pub icons: BTreeMap<u32, u32>,
    pub visuals: BTreeMap<u32, PetSpeciesVisual>,
    pub journal: PetJournal,
    pub summoned: Option<u64>,
    pub selected: Option<PetRowKey>,
    pub search: String,
    pub collected_only: bool,
    pub pending: bool,
    pub error: String,
    pub queue: shared::protocol::PetBattleQueueState,
    pub queue_pending: bool,
}
impl Default for PetJournalView {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: [1280.0, 720.0],
            catalog: BTreeMap::new(),
            icons: BTreeMap::new(),
            visuals: BTreeMap::new(),
            journal: PetJournal::default(),
            summoned: None,
            selected: None,
            search: String::new(),
            collected_only: false,
            pending: false,
            error: String::new(),
            queue: shared::protocol::PetBattleQueueState::Idle,
            queue_pending: false,
        }
    }
}
impl PetJournalView {
    pub fn queue_ready(&self) -> bool {
        matches!(
            self.queue,
            shared::protocol::PetBattleQueueState::Proposal {
                accepted: false,
                ..
            }
        )
    }
    pub fn queue_action(
        &mut self,
        action: &str,
    ) -> Option<shared::protocol::PetBattleQueueRequest> {
        use shared::protocol::{PetBattleQueueRequest as Request, PetBattleQueueState as State};
        if self.queue_pending {
            return None;
        }
        let request = match (action, self.queue) {
            ("pet:find-battle", State::Idle)
                if self.journal.battle_slots.iter().all(Option::is_some) =>
            {
                Request::Join
            }
            ("pet:find-battle", State::Queued | State::Proposal { .. }) => Request::Leave,
            (
                "pet:queue-accept",
                State::Proposal {
                    proposal_id,
                    accepted: false,
                },
            ) => Request::Accept { proposal_id },
            ("pet:queue-decline", State::Proposal { proposal_id, .. }) => {
                Request::Decline { proposal_id }
            }
            _ => return None,
        };
        self.queue_pending = true;
        Some(request)
    }
    pub fn receive_queue(&mut self, update: shared::protocol::PetBattleQueueUpdate) {
        self.queue = update.state;
        self.queue_pending = false;
        self.error = update.error.unwrap_or_default();
    }
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
            let owned_species: BTreeSet<_> =
                self.journal.pets.iter().map(|pet| pet.species_id).collect();
            rows.extend(
                self.catalog
                    .values()
                    .filter(|species| !species.name.is_empty())
                    .filter(|species| !owned_species.contains(&species.pet_id))
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
    pub fn equip_selected(&mut self, slot: usize) -> Option<shared::protocol::SetBattlePetLoadout> {
        if self.pending || slot >= 3 {
            return None;
        }
        let PetRowKey::Owned(id) = self.selected? else {
            return None;
        };
        self.journal.get(id)?;
        let mut slots = self.journal.battle_slots;
        for current in &mut slots {
            if *current == Some(id) {
                *current = None;
            }
        }
        slots[slot] = Some(id);
        self.pending = true;
        Some(shared::protocol::SetBattlePetLoadout { slots })
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
    pub fn selected_visual(&self) -> Option<&PetSpeciesVisual> {
        let species = match self.selected? {
            PetRowKey::Species(id) => id,
            PetRowKey::Owned(id) => self.journal.get(id)?.species_id,
        };
        self.visuals.get(&species)
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
    let Some(view) = ctx.get::<PetJournalView>() else {
        return vec![];
    };
    if !view.visible {
        return queue_popup(view);
    }
    let mut children = Vec::new();
    children.extend(label(
        "PetJournalPetCount".into(),
        &format!("Total Pets: {}", view.journal.count()),
        (70.0, 35.0, 160.0, 20.0),
        (12.0, WHITE, "LEFT"),
    ));
    children.extend(search_box());
    children.extend(pet_list(ctx, view));
    children.extend(pet_card(view));
    children.extend(battle_slots(view));
    children.extend(journal_buttons(view));
    let body = rsx! { r#frame { name: "PetJournal", width: 703.0, height: 606.0,
    left: 0.0, top: 0.0, pos_type: "absolute", mouse_enabled: false, {children} } };
    let mut out = crate::collections_component::collections_shell(view.viewport, 1, body);
    out.extend(queue_popup(view));
    out
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
    let child_top = -(offset as f32);
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
        left: 0.0, top: child_top, pos_type: "absolute", {content} }
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
            (510.0, 110.0, 176.0, 50.0),
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
    if let Some(visual) = view.selected_visual() {
        let fdid = visual.type_icon();
        out.extend(
            rsx! { texture { name: "PetJournalPetCardTypeIcon", texture_fdid: fdid,
            width: 28.0, height: 28.0, left: 659.0, top: 67.0, pos_type: "absolute",
            tex_coords: "0.796875,0.4921875,0.50390625,0.65625" } },
        );
    }
    out.extend(
        rsx! { r#frame { name: "PetJournalPetCardModelScene", width: 173.0, height: 135.0,
        left: 337.0, top: 81.0, pos_type: "absolute", mouse_enabled: true } },
    );
    out.extend(label(
        "PetJournalStatus".into(),
        &view.error,
        (298.0, 185.0, 390.0, 36.0),
        (12.0, "1,0.1,0.1,1", "LEFT"),
    ));
    out
}
fn battle_slots(view: &PetJournalView) -> Element {
    let mut out = Vec::new();
    let selected_owned = matches!(view.selected, Some(PetRowKey::Owned(_)));
    for slot in 0..3 {
        let top = 279.0 + slot as f32 * 100.0;
        let pet = view.journal.battle_slots[slot].and_then(|id| view.journal.get(id));
        let text = pet.map_or_else(
            || format!("Battle slot {}: choose a pet", slot + 1),
            |pet| {
                let name = &view.catalog[&pet.species_id].name;
                format!("{} · Level {} · Slot {}", name, pet.level, slot + 1)
            },
        );
        out.extend(panel_button(
            format!("PetJournalBattleSlot{slot}"),
            &text,
            &format!("pet:equip:{slot}"),
            selected_owned
                && !view.pending
                && view.queue == shared::protocol::PetBattleQueueState::Idle,
            (292.0, top, 405.0, 92.0),
        ));
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
        (292.0, 584.0, 160.0, 22.0),
    );
    out.extend(panel_button(
        "PetJournalFindBattle".into(),
        if matches!(
            view.queue,
            shared::protocol::PetBattleQueueState::Queued
                | shared::protocol::PetBattleQueueState::Proposal { .. }
        ) {
            "Leave Queue"
        } else {
            "Find Battle"
        },
        "pet:find-battle",
        !view.queue_pending
            && view.queue != shared::protocol::PetBattleQueueState::InBattle
            && view.journal.battle_slots.iter().all(Option::is_some),
        (563.0, 584.0, 140.0, 22.0),
    ));
    out
}

fn queue_popup(view: &PetJournalView) -> Element {
    if !view.queue_ready() {
        return vec![];
    }
    let left = view.viewport[0] / 2.0 - 160.0;
    let top = view.viewport[1] / 2.0 - 100.0;
    let mut children = texture(
        "PetBattleQueueBackground".into(),
        6_839_810,
        (7.0, 7.0, 306.0, 186.0),
        WHITE,
    );
    children.extend(
        rsx! { r#frame { name: "PetBattleQueueBorder", width: 320.0, height: 200.0,
        style: "static_popup", pos_type: "absolute", left: 0.0, top: 0.0, frame_level: 5.0 } },
    );
    children.extend(texture(
        "PetBattleQueueArt".into(),
        655474,
        (32.0, 20.0, 256.0, 100.0),
        WHITE,
    ));
    children.extend(label(
        "PetBattleQueueLabel".into(),
        "A pet battle is ready!",
        (10.0, 130.0, 300.0, 25.0),
        (14.0, WHITE, "CENTER"),
    ));
    for (name, text, action, x) in [
        (
            "PetBattleQueueAcceptButton",
            "Accept",
            "pet:queue-accept",
            35.0,
        ),
        (
            "PetBattleQueueDeclineButton",
            "Decline",
            "pet:queue-decline",
            165.0,
        ),
    ] {
        children.extend(panel_button(
            name.into(),
            text,
            action,
            !view.queue_pending,
            (x, 159.0, 120.0, 21.0),
        ));
    }
    rsx! { r#frame { name: "PetBattleQueueReadyFrame", width: 320.0, height: 200.0,
    strata: ui_toolkit::strata::FrameStrata::Dialog, frame_level: 10.0,
    mouse_enabled: true, pos_type: "absolute", left: left, top: top, {children} } }
}
