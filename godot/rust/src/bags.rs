//! Standalone authored bag HUD and containers; inventory stays server-owned.

use game_engine_session::SessionScreen;
use game_engine_ui_model::bag_frame_component::{
    ACTION_BAG_TOGGLE_PREFIX, BagFrameState, bag_frame_screen,
};
use game_engine_ui_model::bags_bar_component::{BagBarState, bags_bar_screen};
use game_engine_ui_model::container_layout_data::container_positions;
use game_engine_ui_model::window_manager::{WindowId, WindowManager};
use godot::global::Key;
use godot::prelude::*;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::replicated::UnitFields;
use crate::ui::RegistryUi;

const BAGS_UI: &str = "BagsUI";

#[derive(Default)]
pub(crate) struct Bags {
    windows: WindowManager,
    pub(crate) ui: Option<Gd<RegistryUi>>,
    npc_backpack_open: bool,
}

#[derive(Clone, PartialEq)]
pub(crate) struct BagsView {
    containers: BagFrameState,
    money: u64,
}

pub(crate) fn bags_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<BagsView>()
        .expect("BagsView must be in SharedContext");
    let mut shared = SharedContext::new();
    shared.insert(view.containers.clone());
    shared.insert(BagBarState { money: view.money });
    let mut elements = bags_bar_screen(&shared);
    elements.extend(bag_frame_screen(&shared));
    elements
}

impl Bags {
    fn reset(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.windows.close_all();
        self.npc_backpack_open = false;
    }
}

impl GameClient {
    pub(super) fn update_bags(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.bags.reset();
            return Ok(());
        }
        self.sync_npc_backpack();
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.poll_bag_actions()?;
        }
        let view = self.bags_view();
        let scale = self.effective_ui_scale();
        if self.bags.ui.is_none() {
            self.mount_bags(view.clone(), scale)?;
        }
        let mut ui = self.bags.ui.clone().ok_or("Bags UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)?;
        self.place_bags(&mut ui)?;
        Ok(())
    }

    fn sync_npc_backpack(&mut self) {
        let open = self.merchant.session.is_open() || self.mailbox.session.is_open();
        if open != self.bags.npc_backpack_open {
            self.bags.windows.set_open(WindowId::Bag(0), open);
            self.bags.npc_backpack_open = open;
        }
    }

    fn bags_view(&self) -> BagsView {
        let mut containers = self.merchant.session.bag_state();
        for bag in &mut containers.bags {
            // The existing NPC owners retain their backpack, never a second visible copy.
            let npc_backpack = bag.bag_index == 0 && self.bags.npc_backpack_open;
            bag.visible = self.bags.windows.is_open(WindowId::Bag(bag.bag_index)) && !npc_backpack;
        }
        let money = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold())
            .unwrap_or(0);
        BagsView { containers, money }
    }

    fn mount_bags(&mut self, view: BagsView, scale: f32) -> Result<(), String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(BAGS_UI);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale).and_then(|()| host.show_bags(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.bags.ui = Some(ui);
        Ok(())
    }

    fn poll_bag_actions(&mut self) -> Result<(), String> {
        let Some(mut ui) = self.bags.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.toggle_bag_action(&action)?;
        }
        while let Some((action, _, _)) = ui.bind_mut().pop_alt_click() {
            self.toggle_bag_action(&action)?;
        }
        Ok(())
    }

    fn toggle_bag_action(&mut self, action: &str) -> Result<(), String> {
        let Some(index) = action.strip_prefix(ACTION_BAG_TOGGLE_PREFIX) else {
            return Err(format!("Standalone bag action not converted: {action}"));
        };
        let index: usize = index
            .parse()
            .map_err(|error| format!("Invalid bag toggle {action}: {error}"))?;
        if self
            .merchant
            .session
            .inventory
            .bags
            .iter()
            .any(|bag| bag.index == index)
        {
            self.bags.windows.toggle(WindowId::Bag(index));
        }
        Ok(())
    }

    pub(super) fn bags_key(&mut self, key: Key) -> bool {
        key == Key::ESCAPE && self.bags.windows.close_all()
    }

    fn place_bags(&self, ui: &mut Gd<RegistryUi>) -> Result<(), String> {
        let screen = {
            let host = ui.bind();
            let registry = host.registry().ok_or("Bags registry missing")?;
            [registry.screen_width, registry.screen_height]
        };
        let mut bags: Vec<_> = self
            .merchant
            .session
            .inventory
            .bags
            .iter()
            .filter(|bag| self.bags.windows.is_open(WindowId::Bag(bag.index)))
            .map(|bag| {
                let (w, h) = BagFrameState::bag_dimensions(bag.size);
                (bag.index, [w, h])
            })
            .collect();
        bags.sort_by_key(|(index, _)| *index);
        for (index, position) in container_positions(&bags, screen) {
            ui.bind_mut()
                .set_window_position(&format!("ContainerFrame{index}"), position)?;
        }
        Ok(())
    }
}
