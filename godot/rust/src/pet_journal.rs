//! Native Retail Pet Journal; account collection messages are authoritative.
use crate::character_frame::preview::{Scene, bind_sheet_light};
use crate::world_models::UnitAppearance;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::pet_journal::{
    PetJournalView, PetRowKey, PetSpeciesFilter, PetSpeciesVisual, SEARCH, pet_journal_screen,
};
use godot::prelude::*;

#[derive(Default)]
pub(crate) struct CompanionJournal {
    pub(crate) view: PetJournalView,
    ui: Option<Gd<RegistryUi>>,
    icons_loaded: bool,
    previews: [PetPreview; 4],
    scene_ids: std::collections::BTreeMap<u32, [u32; 2]>,
    model_scenes: std::collections::BTreeMap<u32, game_engine_core::pet_model_scene::PetModelScene>,
}

#[derive(Default)]
struct PetPreview {
    scene: Option<Scene>,
    pending_model: Option<(u64, PetPreviewModel)>,
    shown_model: Option<PetPreviewModel>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PetPreviewModel {
    display_id: u32,
    scene_id: u32,
}

const PREVIEW_HOSTS: [&str; 4] = [
    "PetJournalPetCardModelScene",
    "PetJournalBattleSlot0ModelScene",
    "PetJournalBattleSlot1ModelScene",
    "PetJournalBattleSlot2ModelScene",
];
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
        self.clear_pet_models();
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
        if !self.pet_journal.view.visible && !self.pet_journal.view.queue_ready() {
            if let Some(ui) = self.pet_journal.ui.take() {
                ui.free();
            }
            return Ok(());
        }
        if self.pet_journal.view.visible {
            self.load_pet_icons()?;
        }
        self.poll_pet_journal_inputs()?;
        if self.pet_journal.view.visible || self.pet_journal.view.queue_ready() {
            self.sync_pet_journal()?;
            if self.pet_journal.view.visible {
                self.sync_pet_models()?;
            } else {
                self.clear_pet_models();
            }
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
        let mut metadata = std::collections::BTreeMap::new();
        let mut scene_ids = std::collections::BTreeMap::new();
        game_engine_core::csv_util::read_numeric_rows(
            &path,
            [
                "ID",
                "IconFileDataID",
                "Flags",
                "SourceTypeEnum",
                "CardUIModelSceneID",
                "LoadoutUIModelSceneID",
            ],
            |[species, fdid, flags, source, card, loadout]| {
                scene_ids.insert(species as u32, [card as u32, loadout as u32]);
                icons.insert(species as u32, fdid as u32);
                metadata.insert(
                    species as u32,
                    PetSpeciesFilter {
                        flags: flags as u32,
                        source: source as i32,
                    },
                );
            },
        )?;
        for icon in icons.values_mut() {
            *icon = self.drawable_fdid(*icon);
        }
        let path = self.data_root.join("battle-pet-model-scenes.json");
        let scenes: Vec<game_engine_core::pet_model_scene::PetModelScene> = serde_json::from_slice(
            &std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", path.display()))?;
        self.pet_journal.model_scenes = scenes.into_iter().map(|scene| (scene.id, scene)).collect();
        self.pet_journal.scene_ids = scene_ids;
        self.pet_journal.view.species_filters = metadata;
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
        if self.pet_journal.view.apply_filter(action) {
            return Ok(());
        }
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
                "pet:find-battle" | "pet:queue-accept" | "pet:queue-decline" => {
                    if let Some(request) = self.pet_journal.view.queue_action(action) {
                        self.account.send_pet_queue(request)?;
                    }
                }
                "pet:close" | "collections:close" => {
                    self.close_pet_journal();
                }
                "collections:pets" => {}
                "collections:mounts" => {
                    self.close_pet_journal();
                    self.toggle_mount_journal();
                }
                "collections:toys" => {
                    self.close_pet_journal();
                    self.toggle_toybox()?;
                }
                "pet:search-clear" => self.pet_journal.view.set_search(String::new()),
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

    fn clear_pet_models(&mut self) {
        for index in 0..PREVIEW_HOSTS.len() {
            self.clear_pet_preview(index);
        }
    }

    fn clear_pet_preview(&mut self, index: usize) {
        let preview = &mut self.pet_journal.previews[index];
        if let Some((id, _)) = preview.pending_model.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = preview.scene.take() {
            scene.free();
        }
        preview.shown_model = None;
    }

    fn sync_pet_models(&mut self) -> Result<(), String> {
        let view = &self.pet_journal.view;
        let card_species = view.selected_visual().map(|visual| visual.species_id);
        let slot_species = view
            .journal
            .battle_slots
            .map(|id| view.journal.get(id?).map(|pet| pet.species_id));
        let species = [
            card_species,
            slot_species[0],
            slot_species[1],
            slot_species[2],
        ];
        for (index, species) in species.into_iter().enumerate() {
            let model = self.pet_preview_model(species, index == 0)?;
            self.sync_pet_preview(index, model)?;
        }
        Ok(())
    }

    fn pet_preview_model(
        &self,
        species: Option<u32>,
        card: bool,
    ) -> Result<Option<PetPreviewModel>, String> {
        let Some(visual) = species.and_then(|id| self.pet_journal.view.visuals.get(&id)) else {
            return Ok(None);
        };
        if visual.display_id == 0 {
            return Ok(None);
        }
        let scenes = self
            .pet_journal
            .scene_ids
            .get(&visual.species_id)
            .ok_or_else(|| format!("Pet species {} scene links missing", visual.species_id))?;
        let scene_id = scenes[usize::from(!card)];
        if !self.pet_journal.model_scenes.contains_key(&scene_id) {
            return Err(format!(
                "Pet species {} authored ModelScene {scene_id} missing",
                visual.species_id
            ));
        }
        Ok(Some(PetPreviewModel {
            display_id: visual.display_id,
            scene_id,
        }))
    }

    fn sync_pet_preview(
        &mut self,
        index: usize,
        model: Option<PetPreviewModel>,
    ) -> Result<(), String> {
        self.request_pet_preview(index, model);
        if model.is_none() {
            return Ok(());
        }
        if self.pet_journal.previews[index].scene.is_none() {
            let host = self
                .pet_journal
                .ui
                .as_ref()
                .ok_or("Collections UI missing")?
                .bind()
                .frame_control(PREVIEW_HOSTS[index])
                .ok_or_else(|| format!("Pet model scene missing: {}", PREVIEW_HOSTS[index]))?;
            self.pet_journal.previews[index].scene = Some(Scene::new(host));
        }
        self.attach_pet_preview(index)?;
        self.pet_journal.previews[index]
            .scene
            .as_mut()
            .expect("created above")
            .fit_viewport();
        Ok(())
    }

    fn request_pet_preview(&mut self, index: usize, model: Option<PetPreviewModel>) {
        let preview = &self.pet_journal.previews[index];
        let current = preview
            .pending_model
            .map(|(_, model)| model)
            .or(preview.shown_model);
        if model == current {
            return;
        }
        self.clear_pet_preview(index);
        if let Some(model) = model {
            let appearance = UnitAppearance::Creature {
                display_id: model.display_id,
                items: Default::default(),
            };
            let id = self.world.request_detached_visual(&appearance);
            self.pet_journal.previews[index].pending_model = Some((id, model));
        }
    }

    fn attach_pet_preview(&mut self, index: usize) -> Result<(), String> {
        let preview = &mut self.pet_journal.previews[index];
        let Some((id, key)) = preview.pending_model else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        preview.pending_model = None;
        let model = loaded?;
        bind_sheet_light(&model);
        let authored = self
            .pet_journal
            .model_scenes
            .get(&key.scene_id)
            .ok_or_else(|| format!("Pet ModelScene {} missing", key.scene_id))?;
        preview
            .scene
            .as_mut()
            .ok_or("Pet model scene missing")?
            .replace_pet_model(model, authored)?;
        preview.shown_model = Some(key);
        Ok(())
    }
}
