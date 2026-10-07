//! Native Retail ClassTrainerFrame, driven by TrainerChannel and replicated Gold.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::trainer::TrainerBook;
use game_engine_ui_model::trainer_frame::{TrainerView, trainer_screen};
use godot::prelude::*;
use shared::components::{Gold, Npc};
use shared::protocol::{TrainerBuyFailed, TrainerList, TrainerServiceState};

#[derive(Default)]
pub(crate) struct Trainer {
    pub(crate) book: TrainerBook,
    ui: Option<Gd<RegistryUi>>,
}
impl Trainer {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
}
impl GameClient {
    pub(super) fn receive_trainer_list(&mut self, list: TrainerList) {
        self.auction_interaction_closed_any();
        self.merchant.session.close();
        self.trainer.book.receive_list(list);
    }
    pub(super) fn receive_trainer_failure(&mut self, failed: TrainerBuyFailed) {
        self.trainer.book.receive_failure(failed);
    }
    pub(super) fn close_trainer(&mut self) -> bool {
        let visible = self.trainer.book.list.is_some();
        self.trainer.book.close();
        self.trainer.free_ui();
        visible
    }
    pub(super) fn trainer_closed_for(&mut self, npc: u64) {
        if self
            .trainer
            .book
            .list
            .as_ref()
            .is_some_and(|list| list.npc == npc)
        {
            self.close_trainer();
        }
    }
    pub(super) fn update_trainer(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_trainer();
            return Ok(());
        }
        if self.trainer.book.list.is_none() {
            return Ok(());
        }
        if let Some(gold) = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<Gold>())
        {
            self.trainer.book.money = gold.0;
        }
        self.trainer.book.primary_professions = self.professions.primary_count();
        self.poll_trainer_actions()?;
        if self.trainer.book.list.is_some() {
            self.sync_trainer()?;
        }
        Ok(())
    }
    fn poll_trainer_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.trainer.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "trainer:close" {
                if let Some(list) = &self.trainer.book.list {
                    self.account.send_close_interaction(list.npc)?;
                }
                self.close_trainer();
                break;
            }
            let request = self.apply_trainer_action(&action)?;
            if let Some(request) = request {
                self.account.send_train(request)?;
            }
        }
        Ok(())
    }
    fn apply_trainer_action(
        &mut self,
        action: &str,
    ) -> Result<Option<shared::protocol::TrainerBuySpell>, String> {
        let book = &mut self.trainer.book;
        match action {
            "trainer:train" => return Ok(book.train()),
            "trainer:confirm" => return Ok(book.confirm(true)),
            "trainer:cancel" => {
                book.confirm(false);
            }
            "trainer:filter:0" => book.toggle_filter(TrainerServiceState::Available),
            "trainer:filter:1" => book.toggle_filter(TrainerServiceState::Unavailable),
            "trainer:filter:2" => book.toggle_filter(TrainerServiceState::Known),
            _ => {
                let id = action
                    .strip_prefix("trainer:select:")
                    .and_then(|id| id.parse().ok())
                    .ok_or_else(|| format!("Unknown trainer action: {action}"))?;
                book.select(id);
            }
        }
        Ok(None)
    }
    fn trainer_view(&self) -> TrainerView {
        let mut view = TrainerView {
            book: self.trainer.book.clone(),
            ..Default::default()
        };
        view.title = view
            .book
            .list
            .as_ref()
            .and_then(|list| self.replica.unit(list.npc))
            .and_then(|unit| unit.get::<Npc>())
            .map(|npc| npc.name.clone())
            .unwrap_or_else(|| "Trainer".into());
        let Some(catalog) = self.spells.catalog() else {
            return view;
        };
        for service in view.book.list.iter().flat_map(|list| &list.services) {
            for id in std::iter::once(service.spell_id).chain(service.req_abilities.iter().copied())
            {
                if let Some(spell) = catalog.get(id) {
                    view.display.names.insert(id, spell.name.to_string());
                    view.display.icons.insert(id, spell.icon_fdid);
                }
            }
        }
        view.display.skills = self.professions.skill_names();
        view
    }
    fn sync_trainer(&mut self) -> Result<(), String> {
        let view = self.trainer_view();
        let scale = self.effective_ui_scale();
        if let Some(ui) = &mut self.trainer.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("TrainerUI");
        self.base_mut().add_child(&ui);
        let result = {
            let mut bound = ui.bind_mut();
            bound
                .set_ui_scale(scale)
                .and_then(|()| bound.show_quest_window(view, trainer_screen))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.trainer.ui = Some(ui);
        Ok(())
    }
}
