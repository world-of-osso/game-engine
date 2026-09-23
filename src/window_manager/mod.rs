//! Single owner of every in-world window's open state, class and stacking order.
//!
//! Classes follow WoW's UIPanelLayout (see `docs/plans/2026-09-23-ingame-ui.md`):
//! at most two Panels (slot L then R), one Wide window exclusive with every Panel,
//! and Containers (bags) that coexist with everything.

use bevy::prelude::*;

use crate::game_state::GameState;

mod input;
mod placement;
mod sessions;

pub use placement::{WindowPlacements, place_windows};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WindowId {
    Achievements,
    Bag(usize),
    Calendar,
    Character,
    EncounterJournal,
    Friends,
    Guild,
    Inspect,
    LootRules,
    Mail,
    Merchant,
    Professions,
    Spellbook,
    Talents,
    WorldMap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowClass {
    Panel,
    Wide,
    Container,
}

impl WindowId {
    pub fn class(self) -> WindowClass {
        match self {
            Self::Achievements | Self::EncounterJournal | Self::Talents | Self::WorldMap => {
                WindowClass::Wide
            }
            Self::Bag(_) => WindowClass::Container,
            Self::Calendar
            | Self::Character
            | Self::Friends
            | Self::Guild
            | Self::Inspect
            | Self::LootRules
            | Self::Mail
            | Self::Merchant
            | Self::Professions
            | Self::Spellbook => WindowClass::Panel,
        }
    }

    /// NPC-driven panels take slot L and push the others right.
    pub fn npc_driven(self) -> bool {
        matches!(self, Self::Mail | Self::Merchant)
    }

    /// Registry name of the window's root frame; also its saved-position key.
    pub fn root_frame_name(self) -> String {
        let name = match self {
            Self::Achievements => "AchievementFrame",
            Self::Bag(index) => return format!("ContainerFrame{index}"),
            Self::Calendar => "CalendarFrame",
            Self::Character => "CharacterFrame",
            Self::EncounterJournal => "EncounterJournal",
            Self::Friends => "FriendsFrame",
            Self::Guild => "GuildFrame",
            Self::Inspect => "InspectFrame",
            Self::LootRules => "LootRulesFrame",
            Self::Mail => "MailFrame",
            Self::Merchant => "MerchantFrame",
            Self::Professions => "ProfessionsFrame",
            Self::Spellbook => "SpellbookFrame",
            Self::Talents => "PlayerSpellsFrame",
            Self::WorldMap => "WorldMapFrame",
        };
        name.to_string()
    }
}

#[derive(Resource, Default, Debug)]
pub struct WindowManager {
    /// Open windows, oldest first.
    open: Vec<WindowId>,
    /// Open panels in slot order: index 0 is slot L.
    panel_slots: Vec<WindowId>,
    /// Raise rank per open window; higher draws on top.
    raise: Vec<(WindowId, u32)>,
    next_raise: u32,
}

const MAX_PANELS: usize = 2;

impl WindowManager {
    pub fn is_open(&self, id: WindowId) -> bool {
        self.open.contains(&id)
    }

    pub fn any_open(&self) -> bool {
        !self.open.is_empty()
    }

    /// Open windows, oldest first.
    pub fn open_windows(&self) -> &[WindowId] {
        &self.open
    }

    pub fn panel_slot(&self, id: WindowId) -> Option<usize> {
        self.panel_slots.iter().position(|entry| *entry == id)
    }

    pub fn panel_in_slot(&self, slot: usize) -> Option<WindowId> {
        self.panel_slots.get(slot).copied()
    }

    pub fn raise_rank(&self, id: WindowId) -> Option<u32> {
        self.raise
            .iter()
            .find(|(entry, _)| *entry == id)
            .map(|(_, rank)| *rank)
    }

    pub fn open(&mut self, id: WindowId) {
        if self.is_open(id) {
            self.raise(id);
            return;
        }
        match id.class() {
            WindowClass::Panel => self.open_panel(id),
            WindowClass::Wide => {
                self.close_matching(|other| other.class() != WindowClass::Container)
            }
            WindowClass::Container => {}
        }
        self.open.push(id);
        self.raise(id);
    }

    pub fn close(&mut self, id: WindowId) -> bool {
        if !self.is_open(id) {
            return false;
        }
        self.open.retain(|entry| *entry != id);
        self.panel_slots.retain(|entry| *entry != id);
        self.raise.retain(|(entry, _)| *entry != id);
        true
    }

    /// Toggles a window; returns whether it is open afterwards.
    pub fn toggle(&mut self, id: WindowId) -> bool {
        if self.close(id) {
            return false;
        }
        self.open(id);
        true
    }

    pub fn set_open(&mut self, id: WindowId, open: bool) {
        if open {
            self.open(id);
        } else {
            self.close(id);
        }
    }

    /// Closes every window (Escape step "close all panels and wide windows").
    /// Returns whether anything was open.
    pub fn close_all(&mut self) -> bool {
        let closed_any = self.any_open();
        self.open.clear();
        self.panel_slots.clear();
        self.raise.clear();
        closed_any
    }

    /// Brings an open window to the top of the stacking order.
    pub fn raise(&mut self, id: WindowId) {
        if !self.is_open(id) {
            return;
        }
        self.next_raise += 1;
        let rank = self.next_raise;
        match self.raise.iter_mut().find(|(entry, _)| *entry == id) {
            Some(entry) => entry.1 = rank,
            None => self.raise.push((id, rank)),
        }
    }

    fn open_panel(&mut self, id: WindowId) {
        self.close_matching(|other| other.class() == WindowClass::Wide);
        if id.npc_driven() {
            self.panel_slots.insert(0, id);
        } else {
            self.panel_slots.push(id);
        }
        while self.panel_slots.len() > MAX_PANELS {
            let oldest = self
                .open
                .iter()
                .copied()
                .find(|entry| entry.class() == WindowClass::Panel)
                .expect("panel slots exceed open panels");
            self.close(oldest);
        }
    }

    fn close_matching(&mut self, predicate: impl Fn(WindowId) -> bool) {
        let doomed: Vec<WindowId> = self
            .open
            .iter()
            .copied()
            .filter(|id| predicate(*id))
            .collect();
        for id in doomed {
            self.close(id);
        }
    }
}

/// Leaving the world (logout, character switch) starts the next session clean.
fn close_all_windows(mut manager: ResMut<WindowManager>) {
    if manager.any_open() {
        manager.close_all();
    }
}

pub struct WindowManagerPlugin;

impl Plugin for WindowManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindowManager>();
        app.init_resource::<WindowPlacements>();
        app.add_systems(
            Update,
            (
                sessions::sync_merchant_window,
                sessions::sync_inspect_window,
                input::raise_window_on_click,
            ),
        );
        app.add_systems(OnExit(GameState::InWorld), close_all_windows);
        app.add_systems(
            PostUpdate,
            place_windows.before(ui_toolkit::plugin::UiRenderSet::Prepare),
        );
    }
}

#[cfg(test)]
#[path = "../../tests/unit/window_manager_tests.rs"]
mod tests;
