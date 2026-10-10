//! Native Retail Pet Journal; account collection messages are authoritative.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::pet_journal::{PetJournalView, PetRowKey, SEARCH, pet_journal_screen};
use godot::prelude::*;

#[derive(Default)]
pub(crate) struct CompanionJournal {
    pub(crate) view: PetJournalView,
    ui: Option<Gd<RegistryUi>>,
    icons_loaded: bool,
}
impl CompanionJournal {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
}
impl GameClient {
    pub(super) fn toggle_pet_journal(&mut self) {
        if self.pet_journal.view.visible {
            self.close_pet_journal();
        } else {
            self.pet_journal.view.visible = true;
        }
    }
    pub(super) fn close_pet_journal(&mut self) -> bool {
        let shown = self.pet_journal.view.visible;
        self.pet_journal.view.visible = false;
        if let Some(ui) = self.pet_journal.ui.take() {
            ui.free();
        }
        shown
    }
    pub(super) fn update_pet_journal(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_pet_journal();
            return Ok(());
        }
        if !self.pet_journal.view.visible {
            return Ok(());
        }
        self.load_pet_icons()?;
        self.poll_pet_journal_inputs()?;
        if self.pet_journal.view.visible {
            self.sync_pet_journal()?;
        }
        Ok(())
    }
    fn load_pet_icons(&mut self) -> Result<(), String> {
        if self.pet_journal.icons_loaded {
            return Ok(());
        }
        let path = self.data_root.join("db2/12.1.0.69933/BattlePetSpecies.csv");
        let mut icons = std::collections::BTreeMap::new();
        game_engine_core::csv_util::read_numeric_rows(
            &path,
            ["ID", "IconFileDataID"],
            |[species, fdid]| {
                icons.insert(species as u32, fdid as u32);
            },
        )?;
        for icon in icons.values_mut() {
            *icon = self.drawable_fdid(*icon);
        }
        self.pet_journal.view.icons = icons;
        self.pet_journal.icons_loaded = true;
        Ok(())
    }
    fn poll_pet_journal_inputs(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.pet_journal.ui.clone() else {
            return Ok(());
        };
        let search = ui.bind_mut().frame_text(SEARCH.into()).to_string();
        if search != self.pet_journal.view.search {
            self.pet_journal.view.set_search(search);
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.apply_pet_journal_action(&action)?;
            if !self.pet_journal.view.visible {
                break;
            }
        }
        Ok(())
    }
    fn apply_pet_journal_action(&mut self, action: &str) -> Result<(), FrameError> {
        if let Some(id) = action.strip_prefix("pet:select:owned:") {
            self.pet_journal.view.select(PetRowKey::Owned(
                id.parse()
                    .map_err(|_| format!("Invalid pet action: {action}"))?,
            ));
        } else if let Some(id) = action.strip_prefix("pet:select:species:") {
            self.pet_journal.view.select(PetRowKey::Species(
                id.parse()
                    .map_err(|_| format!("Invalid pet action: {action}"))?,
            ));
        } else {
            match action {
                "pet:close" => {
                    self.close_pet_journal();
                }
                "pet:filter" => {
                    self.pet_journal.view.collected_only = !self.pet_journal.view.collected_only;
                    self.pet_journal.view.reselect_visible();
                }
                "pet:summon" => {
                    if let Some(request) = self.pet_journal.view.summon_or_dismiss() {
                        self.account.send_pet_request(request)?;
                    }
                }
                _ => return Err(format!("Unknown Pet Journal action: {action}").into()),
            }
        }
        Ok(())
    }
    fn sync_pet_journal(&mut self) -> Result<(), String> {
        let view = self.pet_journal.view.clone();
        let scale = self.effective_ui_scale();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            pet_journal_screen,
        ));
        if let Some(ui) = &mut self.pet_journal.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("PetJournalUI");
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_pet_journal(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.pet_journal.ui = Some(ui);
        Ok(())
    }
}
