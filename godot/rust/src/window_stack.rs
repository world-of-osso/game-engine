//! Pointer ownership across native UI canvases: one hit-test in visual stacking order,
//! Retail raise-on-click for toplevel windows, and one arrival-ordered input stream.
//!
//! Every RegistryUi is its own CanvasLayer. Godot draws and picks canvases by
//! `layer`, then by sibling index, so that pair is the stacking key here too; a
//! release resolves against the same canvas the pointer sees on top.

use game_engine_ui_model::panel_lifecycle::REGISTRATIONS;
use godot::prelude::*;
use ui_toolkit::lifecycle::{CloseGroup, close_registered_group};

use crate::GameClient;
use crate::bag_cursor::BagInput;
use crate::frame_error::FrameError;
use crate::tooltips::HoveredFrame;
use crate::ui::RegistryUi;
use crate::ui::input_queue::in_arrival_order;

type CloseCallback = fn(&mut GameClient) -> Result<bool, FrameError>;

/// Exact domain adapters only; membership, group and priority live in ui-model.
const CLOSE_CALLBACKS: &[(&str, CloseCallback)] = &[
    ("bags", |client| Ok(client.bags.close_all_bags(None))),
    ("housing", |client| {
        let open = client.housing.ui.is_some();
        if open {
            client.close_housing();
        }
        Ok(open)
    }),
    ("toybox", |client| {
        let open = client.toybox.model.open;
        if open {
            client.close_toybox();
        }
        Ok(open)
    }),
    ("spellbook", |client| {
        let open = client.spellbook_open();
        if open {
            client.close_spellbook();
        }
        Ok(open)
    }),
    ("world_map", |client| {
        let open = client.world_map.is_open();
        if open {
            client.close_world_map();
        }
        Ok(open)
    }),
    ("mailbox", |client| Ok(client.close_mailbox_window()?)),
    ("auction", |client| Ok(client.close_auction_window()?)),
    ("merchant", |client| Ok(client.close_merchant_window()?)),
    ("trade", |client| Ok(client.close_trade_window()?)),
    ("quests", |client| client.close_quest_windows()),
    ("character", |client| Ok(client.close_character_window())),
    ("guild_ranks", |client| Ok(client.close_guild_ranks())),
    ("guild_charter", |client| client.close_guild_charter()),
    ("guild_tabard", |client| client.close_guild_tabard()),
    ("professions", |client| client.close_professions_window()),
    ("pet_journal", |client| Ok(client.close_pet_journal())),
    ("mount_journal", |client| {
        let open = client.mount_journal.view.visible;
        if open {
            client.close_mount_journal();
        }
        Ok(open)
    }),
    ("adventure_guide", |client| {
        Ok(client.close_adventure_guide())
    }),
    ("trading_post", |client| client.close_trading_post_window()),
    ("trainer", |client| client.close_trainer_window()),
    ("transmog", |client| client.close_transmog_window()),
    ("achievements", |client| Ok(client.close_achievements())),
    ("mythic_mentor", |client| Ok(client.close_mythic_mentor())),
    ("raid_info", |client| Ok(client.close_raid_info())),
    ("combat_stable", |client| Ok(client.close_combat_stable()?)),
    ("bank", |client| Ok(client.close_bank_window()?)),
    ("guild_bank", |client| Ok(client.close_guild_bank_window()?)),
    ("flight_map", |client| {
        let open = client.flight_map.session.map.is_some();
        if open {
            client.close_flight_map_interaction()?;
        }
        Ok(open)
    }),
    ("professions_customer", |client| {
        client.close_professions_window()
    }),
];

/// Godot canvas order: higher `layer` first, then the later sibling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StackKey {
    pub layer: i32,
    pub index: i32,
}

/// The hit of the topmost canvas that claims the point; `None` when none does.
pub(crate) fn topmost_hit<T>(
    canvases: impl IntoIterator<Item = (StackKey, Option<T>)>,
) -> Option<T> {
    canvases
        .into_iter()
        .filter_map(|(key, hit)| Some((key, hit?)))
        .max_by_key(|(key, _)| *key)
        .map(|(_, hit)| hit)
}

/// Sibling index a raised toplevel window moves to: above every peer on its layer.
pub(crate) fn raised_index(raised: StackKey, peers: &[StackKey]) -> Option<i32> {
    let top = peers
        .iter()
        .filter(|peer| peer.layer == raised.layer)
        .map(|peer| peer.index)
        .max()?;
    (top > raised.index).then_some(top)
}

/// The frame a pointer hit: its canvas and its click action (`None` blocks without one).
pub(crate) struct UiHit {
    pub owner: i64,
    pub action: Option<String>,
}

enum WindowInput {
    Raise(Gd<RegistryUi>),
    Cursor(BagInput),
}

fn stack_key(ui: &Gd<RegistryUi>) -> StackKey {
    StackKey {
        layer: ui.get_layer(),
        index: ui.get_index(),
    }
}

impl GameClient {
    fn registry_uis(&mut self) -> Result<Vec<Gd<RegistryUi>>, String> {
        let mut uis = Vec::new();
        self.for_each_registry_ui(|ui| {
            uis.push(ui.clone());
            Ok(())
        })?;
        Ok(uis)
    }

    /// Topmost native UI frame under a physical viewport point; `None` is the world.
    pub(super) fn ui_hit_at(&mut self, at: Vector2) -> Result<Option<UiHit>, String> {
        Ok(self.ui_frame_at(at, &[])?.map(|hit| UiHit {
            owner: hit.ui.instance_id().to_i64(),
            action: hit.ui.bind().frame_click_action(hit.frame),
        }))
    }

    /// [`Self::ui_hit_at`]'s frame over every canvas but `skip`.
    pub(super) fn ui_frame_at(
        &mut self,
        at: Vector2,
        skip: &[Option<Gd<RegistryUi>>],
    ) -> Result<Option<HoveredFrame>, String> {
        self.registry_frame_at(at, skip, false)
    }

    pub(super) fn ui_tooltip_frame_at(
        &mut self,
        at: Vector2,
        skip: &[Option<Gd<RegistryUi>>],
    ) -> Result<Option<HoveredFrame>, String> {
        self.registry_frame_at(at, skip, true)
    }

    fn registry_frame_at(
        &mut self,
        at: Vector2,
        skip: &[Option<Gd<RegistryUi>>],
        tooltip: bool,
    ) -> Result<Option<HoveredFrame>, String> {
        let canvases = self
            .registry_uis()?
            .into_iter()
            .filter(|ui| !skip.iter().flatten().any(|other| other == ui))
            .map(|ui| {
                let frame = if tooltip {
                    ui.bind().tooltip_frame_at(at)
                } else {
                    ui.bind().pointer_frame_at(at)
                };
                let hit = frame.map(|frame| HoveredFrame {
                    ui: ui.clone(),
                    frame,
                });
                (stack_key(&ui), hit)
            });
        Ok(topmost_hit(canvases.collect::<Vec<_>>()))
    }

    /// Whether `ui` owns the topmost frame under a physical point.
    pub(super) fn ui_owns_point(
        &mut self,
        ui: &Gd<RegistryUi>,
        at: Vector2,
    ) -> Result<bool, String> {
        let owner = ui.instance_id().to_i64();
        Ok(self.ui_hit_at(at)?.is_some_and(|hit| hit.owner == owner))
    }

    /// Retail `Frame:Raise` on a toplevel click: the window moves above its peers.
    fn raise_window(&mut self, ui: &Gd<RegistryUi>) -> Result<(), String> {
        let peers: Vec<_> = self
            .registry_uis()?
            .iter()
            .filter(|peer| peer.bind().is_toplevel())
            .map(stack_key)
            .collect();
        self.move_above(ui, &peers)
    }

    /// Retail `Frame:Raise` on show: the frame moves to the top of its strata, above
    /// every canvas on its layer, HUD panels included.
    pub(super) fn raise_above_layer(&mut self, ui: &Gd<RegistryUi>) -> Result<(), String> {
        let peers: Vec<_> = self.registry_uis()?.iter().map(stack_key).collect();
        self.move_above(ui, &peers)
    }

    fn move_above(&mut self, ui: &Gd<RegistryUi>, peers: &[StackKey]) -> Result<(), String> {
        if let Some(index) = raised_index(stack_key(ui), peers) {
            self.base_mut().move_child(ui, index);
            // Workaround: Godot 4.7 restacks the moved canvas's drawing but keeps its
            // GUI pick order until `set_layer` marks the viewport's root order dirty
            // (probe: hover stays on the old top canvas after move_child alone).
            // Retire when moving a CanvasLayer re-sorts GUI roots itself.
            let mut ui = ui.clone();
            let layer = ui.get_layer();
            ui.set_layer(layer);
        }
        Ok(())
    }

    /// Retail Escape step `CloseAllWindows` (UIParentPanelManager.lua:1091-1106): one
    /// press closes every bag and every panel. Returns whether anything was open.
    pub(super) fn close_all_windows(&mut self) -> Result<bool, FrameError> {
        let mut closed = false;
        for group in [CloseGroup::Bags, CloseGroup::Panels, CloseGroup::Special] {
            closed |=
                close_registered_group(REGISTRATIONS, group, self, Self::close_registered_window)?;
        }
        Ok(closed)
    }

    fn close_registered_window(&mut self, hook: &str) -> Result<bool, FrameError> {
        let (_, close) = CLOSE_CALLBACKS
            .iter()
            .find(|(key, _)| *key == hook)
            .ok_or_else(|| {
                FrameError::Client(format!("Unknown authored window close hook: {hook}"))
            })?;
        close(self)
    }

    /// Raises and cursor-item input from every window canvas, in arrival order.
    pub(super) fn poll_window_inputs(&mut self) -> Result<(), FrameError> {
        let mut drained = Vec::new();
        for mut ui in self.registry_uis()? {
            let (toplevel, cursor) = {
                let ui = ui.bind();
                (ui.is_toplevel(), ui.accepts_cursor_inputs())
            };
            if cursor {
                let inputs = ui.bind_mut().drain_bag_inputs()?;
                drained.extend(
                    inputs
                        .into_iter()
                        .map(|(arrival, input)| (arrival, WindowInput::Cursor(input))),
                );
            }
            let raise = if toplevel {
                ui.bind_mut().take_raise_request()?
            } else {
                None
            };
            if let Some(arrival) = raise {
                drained.push((arrival, WindowInput::Raise(ui)));
            }
        }
        for input in in_arrival_order(drained) {
            match input {
                WindowInput::Raise(ui) => self.raise_window(&ui)?,
                WindowInput::Cursor(input) => self.dispatch_bag_cursor_input(input)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn key(layer: i32, index: i32) -> StackKey {
        StackKey { layer, index }
    }

    #[test]
    fn release_resolves_to_the_later_sibling_canvas_over_a_covered_slot() {
        // BagsUI (index 40) holds bag 1's slot under MerchantUI (index 52).
        let hit = topmost_hit([
            (key(1, 40), Some("ContainerFrame1Slot0")),
            (key(1, 52), Some("MerchantFrame")),
            (key(1, 10), None),
        ]);
        assert_eq!(hit, Some("MerchantFrame"));
    }

    #[test]
    fn higher_layer_wins_over_later_sibling_and_misses_never_block() {
        let hit = topmost_hit([
            (key(1, 90), Some("ContainerFrame1Slot0")),
            (key(5, 3), Some("WorldMapFrame")),
            (key(100, 95), None),
        ]);
        assert_eq!(hit, Some("WorldMapFrame"));
        assert_eq!(
            topmost_hit::<&str>([(key(1, 1), None), (key(2, 2), None)]),
            None
        );
    }

    #[test]
    fn raising_a_covered_window_puts_it_above_its_layer_peers_only() {
        let bags = key(1, 40);
        let peers = [bags, key(1, 52), key(1, 47), key(8, 99)];
        let index = raised_index(bags, &peers).expect("bags are covered");
        assert_eq!(index, 52);
        // After Godot's move_child the merchant shifts down one sibling.
        let hit = topmost_hit([
            (key(1, index), Some("ContainerFrame1Slot0")),
            (key(1, 51), Some("MerchantFrame")),
        ]);
        assert_eq!(hit, Some("ContainerFrame1Slot0"));
    }

    #[test]
    fn the_topmost_window_is_not_moved_again() {
        let merchant = key(1, 52);
        assert_eq!(raised_index(merchant, &[key(1, 40), merchant]), None);
        assert_eq!(raised_index(merchant, &[merchant]), None);
    }
}
