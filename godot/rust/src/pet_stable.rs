//! Native Retail Stable Master host; authoritative records and existing M2 portrait/model renderers.
use crate::account::{NpcMessage, StableMessage};
use crate::character_frame::preview::{Scene, bind_sheet_light};
use crate::frame_error::{FrameError, SessionError};
use crate::unit_portraits::Portrait;
use crate::world_models::UnitAppearance;
use crate::{GameClient, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_collections::pet_stable::{StableSession, first_unused_stable_slot};
use game_engine_ui_collections::pet_stable_component::{self as art, StableFrameView};
use game_engine_ui_model::inworld_unit_frames_component::{FULL_PORTRAIT, PortraitSlot};
use game_engine_ui_model::popup::{PopupOutcome, PopupResult, PopupSpec};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;
use shared::protocol::{InteractionKind, NpcRole, PetStableFailed, PetStableRequest};
use std::collections::HashMap;

const BEAST_MASTERY: u32 = 253; // local chr_specialization Class3, Name Beast Mastery
const RELEASE_POPUP: &str = "RELEASE_PET";
const DRAG_DISTANCE: f32 = 4.0;

#[derive(Default)]
pub(crate) struct CombatPetStable {
    pub session: StableSession,
    pub ui: Option<Gd<RegistryUi>>,
    portraits: HashMap<u16, Portrait>,
    scene: Option<Scene>,
    pending_model: Option<(u64, u32)>,
    shown_display: Option<u32>,
    releasing: Option<u16>,
    rename_prefill: Option<String>,
    drag_from: Option<u16>,
    drag_start: Vector2,
    dragging: bool,
    queued: Vec<PetStableRequest>,
}

impl CombatPetStable {
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
    pub(crate) fn stable_npc_message(&mut self, message: &NpcMessage) -> bool {
        match message {
            NpcMessage::Opened(opened) => {
                if opened.kind == InteractionKind::Role(NpcRole::StableMaster) {
                    let request = self.combat_stable.session.open(opened.npc);
                    self.combat_stable.queued.push(request);
                    self.merchant.session.close();
                    self.mailbox.session.close();
                    self.auction_interaction_closed_any();
                    true
                } else {
                    self.reset_combat_stable();
                    false
                }
            }
            NpcMessage::Closed(npc) if self.combat_stable.session.npc == Some(*npc) => {
                self.reset_combat_stable();
                false
            }
            _ => false,
        }
    }

    pub(crate) fn receive_stable(&mut self, message: StableMessage) -> Result<(), FrameError> {
        match message {
            StableMessage::Contents(contents) => self.combat_stable.session.receive(contents),
            StableMessage::Failed(PetStableFailed { error }) => {
                self.combat_stable.session.receive_failed();
                self.add_world_error(error.message())?;
            }
        }
        Ok(())
    }

    pub(super) fn close_combat_stable(&mut self) -> Result<bool, SessionError> {
        let Some(npc) = self.combat_stable.session.npc else {
            return Ok(false);
        };
        self.reset_combat_stable();
        self.account.send_close_interaction(npc)?;
        Ok(true)
    }

    fn clear_stable_model(&mut self) {
        if let Some((id, _)) = self.combat_stable.pending_model.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.combat_stable.scene.take() {
            scene.free();
        }
        self.combat_stable.shown_display = None;
    }

    fn reset_combat_stable(&mut self) {
        self.group_frames.popups.hide(RELEASE_POPUP);
        self.clear_stable_model();
        for portrait in self.combat_stable.portraits.values_mut() {
            portrait.clear(&mut self.world);
        }
        self.combat_stable.portraits.clear();
        if let Some(ui) = self.combat_stable.ui.take() {
            ui.free();
        }
        self.combat_stable = CombatPetStable::default();
    }

    pub(super) fn update_combat_stable(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld
            || self.combat_stable.session.npc.is_none()
        {
            self.reset_combat_stable();
            return Ok(());
        }
        self.poll_stable_actions()?;
        if self.combat_stable.session.npc.is_none() {
            return Ok(());
        }
        for request in std::mem::take(&mut self.combat_stable.queued) {
            self.account.send_stable_request(request)?;
        }
        self.sync_stable_ui()?;
        self.sync_stable_portraits()?;
        self.sync_stable_model()?;
        Ok(())
    }

    fn poll_stable_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.combat_stable.ui.clone() else {
            return Ok(());
        };
        let search = ui.bind_mut().frame_text(art::SEARCH.into()).to_string();
        self.combat_stable.session.set_search(search);
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.apply_stable_action(&action)?;
            if self.combat_stable.ui.is_none() {
                break;
            }
        }
        Ok(())
    }

    fn apply_stable_action(&mut self, action: &str) -> Result<(), FrameError> {
        let request = if let Some(slot) = action.strip_prefix("stable_slot:") {
            self.combat_stable
                .session
                .select(slot.parse().map_err(|_| "Invalid stable slot action")?);
            None
        } else if let Some(spec) = action.strip_prefix("stable_spec:") {
            self.combat_stable
                .session
                .choose_spec(spec.parse().map_err(|_| "Invalid stable spec action")?)
        } else {
            match action {
                "stable_close" => {
                    self.close_combat_stable()?;
                    None
                }
                "stable_toggle" => self.combat_stable.session.toggle(),
                "stable_rename" => {
                    self.combat_stable.rename_prefill = self.combat_stable.session.begin_rename();
                    None
                }
                "stable_rename_cancel" => {
                    self.combat_stable.session.rename_slot = None;
                    None
                }
                "stable_rename_accept" => {
                    let ui = self.combat_stable.ui.as_mut().ok_or("Stable UI missing")?;
                    let name = ui
                        .bind_mut()
                        .frame_text(art::RENAME_INPUT.into())
                        .to_string();
                    self.combat_stable.session.rename(name)
                }
                "stable_spec_picker" => {
                    self.combat_stable.session.spec_picker =
                        !self.combat_stable.session.spec_picker;
                    None
                }
                "stable_release" => {
                    self.show_stable_release();
                    None
                }
                "stable_scroll_up" => {
                    self.scroll_stable(-1);
                    None
                }
                "stable_scroll_down" => {
                    self.scroll_stable(1);
                    None
                }
                _ => return Err(format!("Unknown stable action: {action}").into()),
            }
        };
        if let Some(request) = request {
            self.combat_stable.queued.push(request);
        }
        Ok(())
    }

    fn show_stable_release(&mut self) {
        let Some(pet) = self.combat_stable.session.selected_pet() else {
            return;
        };
        self.combat_stable.releasing = Some(pet.slot);
        self.group_frames.popups.push(PopupSpec {
            key: RELEASE_POPUP.into(),
            text: format!("Do you want to permanently release {}?", pet.name),
            accept_label: "Yes".into(),
            cancel_label: Some("No".into()),
            timeout: None,
            confirm_text: None,
        });
        self.physical_input.clear_gameplay();
    }

    pub(super) fn dispatch_stable_popup_results(&mut self, results: &[PopupResult]) {
        for result in results.iter().filter(|result| result.key == RELEASE_POPUP) {
            let Some(slot) = self.combat_stable.releasing.take() else {
                continue;
            };
            if result.outcome == PopupOutcome::Accepted {
                if let Some(request) = self.combat_stable.session.abandon(slot) {
                    self.combat_stable.queued.push(request);
                }
            }
        }
    }

    fn scroll_stable(&mut self, delta: i32) {
        let maximum = self
            .combat_stable
            .session
            .stabled_pets()
            .len()
            .saturating_sub(art::VISIBLE_ROWS);
        let first = self.combat_stable.session.first_row as i64 + i64::from(delta);
        self.combat_stable.session.first_row = first.clamp(0, maximum as i64) as usize;
    }

    fn sync_stable_ui(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Stable has no viewport")?
            .get_visible_rect()
            .size;
        let view = StableFrameView::from_session(
            &self.combat_stable.session,
            [size.x / scale, size.y / scale],
            self.account.spells.spec() == Some(BEAST_MASTERY),
        );
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            art::pet_stable_screen,
        ));
        self.extract_art(&[art::PORTRAIT_MASK]);
        if self.combat_stable.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("CombatPetStableUI");
            ui.set_layer(10);
            self.base_mut().add_child(&ui);
            let shown = {
                let mut host = ui.bind_mut();
                host.set_ui_scale(scale)
                    .and_then(|()| host.show_combat_pet_stable(view.clone()))
            };
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.combat_stable.ui = Some(ui);
        }
        let mut ui = self.combat_stable.ui.clone().ok_or("Stable UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view.clone())?;
        if let Some(name) = self.combat_stable.rename_prefill.take() {
            ui.bind_mut().set_editbox_text(art::RENAME_INPUT, &name)?;
        }
        self.connect_stable_pointer_controls(&ui, &view)?;
        Ok(())
    }

    fn connect_stable_pointer_controls(
        &self,
        ui: &Gd<RegistryUi>,
        view: &StableFrameView,
    ) -> Result<(), String> {
        let mut names = vec![(art::FRAME_NAME.to_owned(), -1)];
        names.extend(
            (0..5 + usize::from(view.secondary))
                .map(|slot| (art::slot_name(slot as u16), slot as i64)),
        );
        names.extend(
            view.rows
                .iter()
                .map(|pet| (art::slot_name(pet.slot), i64::from(pet.slot))),
        );
        for (name, slot) in names {
            let Some(mut control) = ui.bind().frame_control(&name) else {
                continue;
            };
            if control.has_meta("stable_pointer_connected") {
                continue;
            }
            let callback = self
                .base()
                .callable("stable_gui_input")
                .bind(&[slot.to_variant()]);
            let error = control.connect("gui_input", &callback);
            if error != godot::global::Error::OK {
                return Err(format!("Connect stable pointer {name}: {error:?}"));
            }
            control.set_meta("stable_pointer_connected", &true.to_variant());
        }
        Ok(())
    }

    pub(super) fn stable_gui_event(&mut self, event: Gd<InputEvent>, slot: i64) {
        if let Ok(mouse) = event.clone().try_cast::<InputEventMouseMotion>() {
            if self.combat_stable.drag_from.is_some()
                && mouse
                    .get_global_position()
                    .distance_to(self.combat_stable.drag_start)
                    >= DRAG_DISTANCE
            {
                self.combat_stable.dragging = true;
            }
            return;
        }
        let Ok(mouse) = event.try_cast::<InputEventMouseButton>() else {
            return;
        };
        match mouse.get_button_index() {
            MouseButton::WHEEL_UP if mouse.is_pressed() && slot == -1 => self.scroll_stable(-1),
            MouseButton::WHEEL_DOWN if mouse.is_pressed() && slot == -1 => self.scroll_stable(1),
            MouseButton::LEFT => self.stable_drag_event(&mouse, slot),
            _ => {}
        }
    }

    fn stable_drag_event(&mut self, mouse: &Gd<InputEventMouseButton>, slot: i64) {
        if self.combat_stable.session.pending || self.combat_stable.session.rename_slot.is_some() {
            return;
        }
        if mouse.is_pressed() {
            let slot = u16::try_from(slot).ok();
            let exists = slot.is_some_and(|slot| {
                self.combat_stable
                    .session
                    .contents
                    .as_ref()
                    .is_some_and(|contents| contents.pets.iter().any(|pet| pet.slot == slot))
            });
            self.combat_stable.drag_from = slot.filter(|_| exists);
            self.combat_stable.drag_start = mouse.get_global_position();
            self.combat_stable.dragging = false;
        } else {
            let from = self.combat_stable.drag_from.take();
            let dragging = std::mem::take(&mut self.combat_stable.dragging);
            if let Some(from) = from.filter(|_| dragging) {
                if let Some(to) = self.stable_drop_slot(mouse.get_global_position()) {
                    if let Some(request) = self.combat_stable.session.swap(from, to) {
                        self.combat_stable.queued.push(request);
                    }
                }
            }
        }
    }

    fn stable_drop_slot(&self, point: Vector2) -> Option<u16> {
        let ui = self.combat_stable.ui.as_ref()?;
        let contents = self.combat_stable.session.contents.as_ref()?;
        for slot in (0..5).chain(contents.pets.iter().map(|pet| usize::from(pet.slot))) {
            let control = ui.bind().frame_control(&art::slot_name(slot as u16));
            if control.is_some_and(|control| control.get_global_rect().contains_point(point)) {
                return Some(slot as u16);
            }
        }
        let root = ui.bind().frame_control(art::FRAME_NAME)?;
        let local = (point - root.get_global_rect().position) / self.effective_ui_scale();
        (local.x < 330.0 && local.y >= 93.0 && local.y <= 636.0)
            .then(|| first_unused_stable_slot(contents))
    }

    fn sync_stable_portraits(&mut self) -> Result<(), String> {
        let ui = self.combat_stable.ui.clone().ok_or("Stable UI missing")?;
        let contents = self.combat_stable.session.contents.as_ref();
        let pets: Vec<_> = contents
            .into_iter()
            .flat_map(|contents| &contents.pets)
            .cloned()
            .collect();
        let visible: Vec<_> = pets
            .into_iter()
            .filter_map(|pet| {
                ui.bind()
                    .frame_control(&art::portrait_name(pet.slot))
                    .map(|host| (pet, host))
            })
            .collect();
        let hidden: Vec<_> = self
            .combat_stable
            .portraits
            .keys()
            .copied()
            .filter(|slot| !visible.iter().any(|(pet, _)| pet.slot == *slot))
            .collect();
        for slot in hidden {
            if let Some(mut portrait) = self.combat_stable.portraits.remove(&slot) {
                portrait.clear(&mut self.world);
            }
        }
        for (pet, host) in visible {
            let size = if pet.slot < 6 { 70.0 } else { 60.0 };
            let portrait = self
                .combat_stable
                .portraits
                .entry(pet.slot)
                .or_insert_with(|| {
                    Portrait::new(PortraitSlot {
                        frame: "StablePetPortrait",
                        rect: (0.0, 0.0, size, size),
                        mask_fdid: art::PORTRAIT_MASK,
                        mask_rect: (0.0, 0.0, size, size),
                        tex_coords: FULL_PORTRAIT,
                    })
                });
            portrait.sync(
                &mut self.world,
                Some(host),
                Some(UnitAppearance::Creature {
                    display_id: pet.display_id,
                    items: Default::default(),
                }),
            )?;
        }
        Ok(())
    }

    fn sync_stable_model(&mut self) -> Result<(), String> {
        let display = self
            .combat_stable
            .session
            .selected_pet()
            .map(|pet| pet.display_id);
        let current = self
            .combat_stable
            .pending_model
            .map(|(_, display)| display)
            .or(self.combat_stable.shown_display);
        if display != current {
            self.clear_stable_model();
            if let Some(display) = display {
                let id = self
                    .world
                    .request_detached_visual(&UnitAppearance::Creature {
                        display_id: display,
                        items: Default::default(),
                    });
                self.combat_stable.pending_model = Some((id, display));
            }
        }
        if display.is_none() {
            return Ok(());
        }
        if self.combat_stable.scene.is_none() {
            let host = self
                .combat_stable
                .ui
                .as_ref()
                .ok_or("Stable UI missing")?
                .bind()
                .frame_control(art::MODEL)
                .ok_or("Stable model scene missing")?;
            self.combat_stable.scene = Some(Scene::new(host));
        }
        if let Some((id, display)) = self.combat_stable.pending_model {
            if let Some(loaded) = self.world.take_detached_visual(id) {
                self.combat_stable.pending_model = None;
                self.combat_stable.shown_display = Some(display);
                let model = loaded?;
                bind_sheet_light(&model);
                self.combat_stable
                    .scene
                    .as_mut()
                    .expect("stable model scene")
                    .replace_model(model);
            }
        }
        self.combat_stable
            .scene
            .as_mut()
            .expect("stable model scene")
            .fit_viewport();
        Ok(())
    }

    pub(super) fn combat_stable_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("shown", self.combat_stable.session.npc.is_some());
        state.set("pending", self.combat_stable.session.pending);
        let Some(contents) = &self.combat_stable.session.contents else {
            return state;
        };
        state.set("npc", contents.npc as i64);
        state.set(
            "current",
            &contents
                .current
                .map(|slot| i64::from(slot).to_variant())
                .unwrap_or_default(),
        );
        let mut pets = VarArray::new();
        for pet in &contents.pets {
            let mut entry = VarDictionary::new();
            entry.set("slot", i64::from(pet.slot));
            entry.set("name", pet.name.as_str());
            entry.set("health", pet.health);
            entry.set(
                "specialization",
                &pet.specialization
                    .map(|id| i64::from(id).to_variant())
                    .unwrap_or_default(),
            );
            pets.push(&entry.to_variant());
        }
        state.set("pets", &pets);
        state
    }
}
