//! Native Retail Pet Journal; account collection messages are authoritative.
use crate::character_frame::preview::{Scene, bind_sheet_light};
use crate::world_models::UnitAppearance;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::pet_journal::{
    PetJournalView, PetRowKey, PetSpeciesVisual, SEARCH, pet_journal_screen,
};
use godot::prelude::*;

#[derive(Default)]
pub(crate) struct CompanionJournal {
    pub(crate) view: PetJournalView,
    ui: Option<Gd<RegistryUi>>,
    icons_loaded: bool,
    scene: Option<Scene>,
    pending_model: Option<(u64, u32)>,
    shown_display: Option<u32>,
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
        } else if self.toybox.model.open {
            self.close_toybox();
        } else {
            self.pet_journal.view.visible = true;
        }
    }
    pub(super) fn close_pet_journal(&mut self) -> bool {
        let shown = self.pet_journal.view.visible;
        self.pet_journal.view.visible = false;
        self.clear_pet_card_model();
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
            self.sync_pet_card_model()?;
        }
        Ok(())
    }
    fn load_pet_icons(&mut self) -> Result<(), String> {
        if self.pet_journal.icons_loaded {
            return Ok(());
        }
        self.load_pet_visuals()?;
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
    fn load_pet_visuals(&mut self) -> Result<(), String> {
        let path = self.data_root.join("battle-pets.json");
        let records: Vec<PetSpeciesVisual> = serde_json::from_slice(
            &std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", path.display()))?;
        for visual in &records {
            if visual.family > 9 {
                return Err(format!(
                    "Pet species {} has invalid family {}",
                    visual.species_id, visual.family
                ));
            }
        }
        self.pet_journal.view.visuals = records.into_iter().map(|v| (v.species_id, v)).collect();
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
        if let Some(slot) = action.strip_prefix("pet:equip:") {
            let slot = slot
                .parse()
                .map_err(|_| format!("Invalid loadout action: {action}"))?;
            if let Some(request) = self.pet_journal.view.equip_selected(slot) {
                self.account.send_pet_loadout(request)?;
            }
        } else if let Some(id) = action.strip_prefix("pet:select:owned:") {
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
                "pet:close" | "collections:close" => {
                    self.close_pet_journal();
                }
                "collections:pets" => {}
                "collections:toys" => {
                    self.close_pet_journal();
                    self.toggle_toybox()?;
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
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Pet Journal has no viewport")?
            .get_visible_rect()
            .size;
        self.pet_journal.view.viewport = [size.x / scale, size.y / scale];
        let view = self.pet_journal.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            pet_journal_screen,
        ));
        if let Some(ui) = &mut self.pet_journal.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CollectionsJournalUI");
        ui.set_layer(6);
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

    fn clear_pet_card_model(&mut self) {
        if let Some((id, _)) = self.pet_journal.pending_model.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.pet_journal.scene.take() {
            scene.free();
        }
        self.pet_journal.shown_display = None;
    }

    fn sync_pet_card_model(&mut self) -> Result<(), String> {
        let display = self
            .pet_journal
            .view
            .selected_visual()
            .map(|visual| visual.display_id)
            .filter(|id| *id != 0);
        let current = self
            .pet_journal
            .pending_model
            .map(|(_, display)| display)
            .or(self.pet_journal.shown_display);
        if display != current {
            self.clear_pet_card_model();
            if let Some(display_id) = display {
                let appearance = UnitAppearance::Creature {
                    display_id,
                    items: Default::default(),
                };
                let id = self.world.request_detached_visual(&appearance);
                self.pet_journal.pending_model = Some((id, display_id));
            }
        }
        if display.is_none() {
            return Ok(());
        }
        if self.pet_journal.scene.is_none() {
            let host = self
                .pet_journal
                .ui
                .as_ref()
                .ok_or("Collections UI missing")?
                .bind()
                .frame_control("PetJournalPetCardModelScene")
                .ok_or("Pet card model scene missing")?;
            self.pet_journal.scene = Some(Scene::new(host));
        }
        self.attach_pet_card_model()?;
        self.pet_journal
            .scene
            .as_mut()
            .expect("created above")
            .fit_viewport();
        Ok(())
    }

    fn attach_pet_card_model(&mut self) -> Result<(), String> {
        let Some((id, display)) = self.pet_journal.pending_model else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        self.pet_journal.pending_model = None;
        let model = loaded?;
        bind_sheet_light(&model);
        self.pet_journal
            .scene
            .as_mut()
            .ok_or("Pet card model scene missing")?
            .replace_model(model);
        self.pet_journal.shown_display = Some(display);
        Ok(())
    }
}
