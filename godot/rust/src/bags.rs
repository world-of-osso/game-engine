//! Standalone authored bag HUD and containers; inventory stays server-owned.

use game_engine_session::SessionScreen;
use game_engine_ui_model::bag_frame_component::{
    ACTION_BAG_TOGGLE_PREFIX, BagFrameState, bag_frame_screen,
};
use game_engine_ui_model::bags_bar_component::{BagBarState, bags_bar_screen};
use game_engine_ui_model::container_layout_data::container_positions;
use game_engine_ui_model::window_manager::{WindowId, WindowManager};
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
    pub(crate) tooltip_ui: Option<Gd<RegistryUi>>,
    pub(crate) cursor: crate::bag_cursor::BagCursor,
    /// NPC windows open last frame, for their OpenAllBags/CloseAllBags edges.
    npc_windows: Vec<WindowId>,
    /// Retail `FRAME_THAT_OPENED_BAGS` (ContainerFrame.lua:1903-1921, 1987-1999).
    bags_opener: Option<WindowId>,
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
        if let Some(ui) = self.tooltip_ui.take() {
            ui.free();
        }
        self.windows.close_all();
        self.cursor.reset();
        self.npc_windows.clear();
        self.bags_opener = None;
    }

    fn any_bag_open(&self) -> bool {
        self.windows
            .open_windows()
            .iter()
            .any(|id| matches!(id, WindowId::Bag(_)))
    }

    /// Retail `OpenAllBags(frame)`: a no-op while any bag is open; the first opener
    /// is remembered so only it closes them again.
    pub(crate) fn open_all_bags(
        &mut self,
        opener: WindowId,
        bags: impl IntoIterator<Item = usize>,
    ) {
        if self.any_bag_open() {
            return;
        }
        self.bags_opener.get_or_insert(opener);
        for bag in bags {
            self.windows.open(WindowId::Bag(bag));
        }
    }

    /// Retail `CloseAllBags(frame)`: a frame that did not open the bags closes none;
    /// `None` (Escape's `CloseAllWindows`) closes them unconditionally.
    pub(crate) fn close_all_bags(&mut self, closer: Option<WindowId>) -> bool {
        if closer.is_some() && closer != self.bags_opener {
            return false;
        }
        self.bags_opener = None;
        let open: Vec<_> = self
            .windows
            .open_windows()
            .iter()
            .copied()
            .filter(|id| matches!(id, WindowId::Bag(_)))
            .collect();
        for id in &open {
            self.windows.close(*id);
        }
        !open.is_empty()
    }
}

impl GameClient {
    pub(super) fn update_bags(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.bags.reset();
            return Ok(());
        }
        self.sync_npc_bags();
        self.clear_stale_bag_cursor();
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.poll_window_inputs()?;
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
        self.sync_bag_cursor()?;
        Ok(self.sync_bag_tooltip()?)
    }

    /// NPC windows that open every bag on show and close them on hide (Retail
    /// `OpenAllBags`/`CloseAllBags` in MerchantFrame.lua:147/165, MailFrame.lua:63/73,
    /// Blizzard_AuctionHouseFrame.lua:402/462, BankFrame.lua:74/81). TradeFrame and
    /// GuildBankFrame do not open bags. A new NPC window adds its entry here.
    fn npc_bag_windows(&self) -> [(WindowId, bool); 3] {
        [
            (WindowId::Merchant, self.merchant.session.is_open()),
            (WindowId::Mail, self.mailbox.session.is_open()),
            (
                WindowId::AuctionHouse,
                self.auction.session.ui.npc.is_some(),
            ),
        ]
    }

    /// NPC windows that draw the backpack inside their own canvas.
    fn npc_backpack_embedded(&self) -> bool {
        self.merchant.session.is_open() || self.mailbox.session.is_open()
    }

    fn sync_npc_bags(&mut self) {
        let open: Vec<WindowId> = self
            .npc_bag_windows()
            .into_iter()
            .filter_map(|(id, open)| open.then_some(id))
            .collect();
        for closed in self.bags.npc_windows.clone() {
            if !open.contains(&closed) {
                self.bags.close_all_bags(Some(closed));
            }
        }
        let bags: Vec<usize> = self
            .merchant
            .session
            .inventory
            .bags
            .iter()
            .map(|bag| bag.index)
            .collect();
        for opened in &open {
            if !self.bags.npc_windows.contains(opened) {
                self.bags.open_all_bags(*opened, bags.iter().copied());
            }
        }
        self.bags.npc_windows = open;
    }

    fn bags_view(&self) -> BagsView {
        let mut containers = self.merchant.session.bag_state();
        for bag in &mut containers.bags {
            // NPC windows drawing the backpack keep it; never a second visible copy.
            let npc_backpack = bag.bag_index == 0 && self.npc_backpack_embedded();
            bag.visible = self.bags.windows.is_open(WindowId::Bag(bag.bag_index)) && !npc_backpack;
            let items = self.merchant.session.inventory.slots.get(bag.bag_index);
            for (index, slot) in bag.slots.iter_mut().enumerate() {
                let guid = items.and_then(|s| s.get(index)).map_or(0, |s| s.item_guid);
                // Mail attachments stay locked in every bag (`SetItemButtonDesaturated`).
                slot.locked = self.bags.cursor.item.source()
                    == Some(shared::protocol::ItemLocation::Bag {
                        bag: bag.bag_index as u8,
                        slot: index as u8,
                    })
                    || (guid != 0 && self.mailbox.session.is_attached(guid));
            }
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

    pub(super) fn dispatch_bag_action(
        &mut self,
        action: &str,
        click: game_engine_ui_model::merchant::Click,
    ) -> Result<(), FrameError> {
        if action.starts_with(game_engine_ui_model::bag_frame_component::ACTION_BAG_SLOT_PREFIX) {
            if let Some(handled) = self.mail_bag_slot_click(action, click) {
                return Ok(handled?);
            }
            self.bag_cursor_click(action, click)
        } else {
            Ok(self.toggle_bag_action(action)?)
        }
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

    fn place_bags(&self, ui: &mut Gd<RegistryUi>) -> Result<(), String> {
        let screen = {
            let host = ui.bind();
            let registry = host.registry().ok_or("Bags registry missing")?;
            [registry.screen_width, registry.screen_height]
        };
        // An NPC canvas drawing the backpack keeps its slot in the stack.
        let embedded = self.npc_backpack_embedded();
        let mut bags: Vec<_> = self
            .merchant
            .session
            .inventory
            .bags
            .iter()
            .filter(|bag| {
                self.bags.windows.is_open(WindowId::Bag(bag.index)) || (embedded && bag.index == 0)
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn open_bags(bags: &Bags) -> Vec<WindowId> {
        bags.windows.open_windows().to_vec()
    }

    #[test]
    fn npc_opener_opens_every_bag_and_only_it_closes_them() {
        let mut bags = Bags::default();
        bags.open_all_bags(WindowId::Merchant, [0, 1, 2]);
        assert_eq!(
            open_bags(&bags),
            [WindowId::Bag(0), WindowId::Bag(1), WindowId::Bag(2)]
        );
        // A second NPC window opening meanwhile does not take over the opener.
        bags.open_all_bags(WindowId::AuctionHouse, [0, 1, 2]);
        assert!(!bags.close_all_bags(Some(WindowId::AuctionHouse)));
        assert_eq!(open_bags(&bags).len(), 3);
        assert!(bags.close_all_bags(Some(WindowId::Merchant)));
        assert!(open_bags(&bags).is_empty());
    }

    #[test]
    fn bags_the_player_opened_survive_an_npc_visit() {
        let mut bags = Bags::default();
        bags.windows.open(WindowId::Bag(1));
        bags.open_all_bags(WindowId::Mail, [0, 1]);
        assert_eq!(open_bags(&bags), [WindowId::Bag(1)]);
        assert!(!bags.close_all_bags(Some(WindowId::Mail)));
        assert_eq!(open_bags(&bags), [WindowId::Bag(1)]);
    }

    #[test]
    fn escape_closes_every_bag_whoever_opened_them() {
        let mut bags = Bags::default();
        bags.open_all_bags(WindowId::Merchant, [0, 1]);
        assert!(bags.close_all_bags(None));
        assert!(open_bags(&bags).is_empty());
        // The opener is forgotten: the next NPC window opens them afresh.
        bags.open_all_bags(WindowId::AuctionHouse, [0]);
        assert!(bags.close_all_bags(Some(WindowId::AuctionHouse)));
        assert!(!bags.close_all_bags(None));
    }
}
