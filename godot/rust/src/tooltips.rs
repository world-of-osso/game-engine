//! The native `GameTooltip` (docs/specs/unit-tooltip.md, "Native Godot coverage"): every
//! frame the topmost mouse-enabled frame under the pointer across the mounted registries,
//! else the unit under the cursor, picks the tooltip; `game_tooltip` builds and places it,
//! and one `GameTooltipUI` draws it with the Shift comparison tooltips. Rebuilding every
//! frame refreshes what changes while hovered (cooldowns, aura time left, collection marks).

use std::collections::{HashMap, HashSet};

use game_engine_core::input_bindings_data::InputState;
use game_engine_session::SessionScreen;
use game_engine_ui_model::bag_data::InventorySlot;
use game_engine_ui_model::game_tooltip::item::comparisons;
use game_engine_ui_model::game_tooltip::unit::FactionNames;
use game_engine_ui_model::game_tooltip::{
    GameTooltip, GameTooltipView, OwnerSide, TooltipScreen, place, place_comparisons,
};
use godot::prelude::*;
use shared::components::UnitLevel;
use shared::protocol::{AppearanceCollectionUpdate, CreatureTooltip};
use shared::transmog::AppearanceCollection;
use ui_toolkit::frame::Frame;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

const TOOLTIP_UI: &str = "GameTooltipUI";
const TOOLTIP_LAYER: i32 = 8;
const FACTION_CSV: &str = "db2/12.1.0.69933/Faction.csv";

/// The tooltip host, the server's creature answers and the account's appearances.
#[derive(Default)]
pub(crate) struct Tooltips {
    ui: Option<Gd<RegistryUi>>,
    /// `CreatureTooltip` answers by creature entry, until leaving the world.
    creatures: HashMap<u32, CreatureTooltip>,
    /// Entries already asked with `CreatureTooltipQuery`.
    asked: HashSet<u32>,
    appearances: AppearanceCollection,
    factions: Option<Result<FactionNames, String>>,
    /// What the host shows, for automation.
    shown: GameTooltipView,
}

impl Tooltips {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match &mut self.ui {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }

    pub(crate) fn reset(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.creatures.clear();
        self.asked.clear();
        self.shown = GameTooltipView::default();
    }

    pub(crate) fn receive_creature(&mut self, tooltip: CreatureTooltip) {
        self.creatures.insert(tooltip.entry, tooltip);
    }

    /// `AppearanceCollectionUpdate`: the account's learned appearances, replacing the old set.
    pub(crate) fn receive_appearances(&mut self, update: AppearanceCollectionUpdate) {
        let mut collection = AppearanceCollection::default();
        collection.learn_many(update.appearances);
        self.appearances = collection;
    }

    pub(crate) fn creature(&self, entry: u32) -> Option<&CreatureTooltip> {
        self.creatures.get(&entry)
    }

    pub(crate) fn appearances(&self) -> &AppearanceCollection {
        &self.appearances
    }

    /// Entries never asked yet; each is asked once.
    pub(crate) fn take_unasked(&mut self, entry: u32) -> bool {
        self.asked.insert(entry)
    }

    pub(crate) fn faction_names(&mut self, data_root: &std::path::Path) -> Option<&FactionNames> {
        self.factions
            .get_or_insert_with(|| FactionNames::load(&data_root.join(FACTION_CSV)))
            .as_ref()
            .inspect_err(|error| {
                crate::frame_error::report_once(&format!("Faction names: {error}"))
            })
            .ok()
    }
}

/// What is under the pointer: a frame of a mounted registry.
pub(crate) struct HoveredFrame {
    pub ui: Gd<RegistryUi>,
    pub frame: u64,
}

/// A tooltip, the item it shows (which the Shift comparison compares) and the health of
/// the unit it shows.
pub(crate) struct HoveredTooltip {
    pub tooltip: GameTooltip,
    pub item: Option<InventorySlot>,
    pub health: Option<f32>,
}

impl HoveredTooltip {
    pub fn text(tooltip: GameTooltip) -> Self {
        Self {
            tooltip,
            item: None,
            health: None,
        }
    }
}

impl GameClient {
    pub(super) fn update_tooltips(&mut self) -> Result<(), FrameError> {
        let view = if self.account.session.screen == SessionScreen::InWorld {
            self.game_tooltip_view()?
        } else {
            self.tooltips.reset();
            return Ok(());
        };
        self.tooltips.shown = view.clone();
        Ok(self.sync_game_tooltip(view)?)
    }

    fn tooltip_screen(&self) -> Option<TooltipScreen> {
        let scale = self.effective_ui_scale();
        let size = self.base().get_viewport()?.get_visible_rect().size / scale;
        let [x, y] = self.physical_input.pointer();
        Some(TooltipScreen {
            size: [size.x, size.y],
            cursor: [x / scale, y / scale],
        })
    }

    fn game_tooltip_view(&mut self) -> Result<GameTooltipView, FrameError> {
        let Some(screen) = self.tooltip_screen() else {
            return Ok(GameTooltipView::default());
        };
        let hovered = match self.hovered_ui_frame() {
            Some(hit) => self.frame_tooltip(&hit),
            None => match self.minimap_button_tooltip() {
                Some(tooltip) => Some(tooltip),
                None => self.world_tooltip()?,
            },
        };
        let Some(HoveredTooltip {
            tooltip,
            item,
            health,
        }) = hovered
        else {
            return Ok(GameTooltipView::default());
        };
        let anchor = tooltip.anchor;
        let mut main = place(tooltip, screen);
        let compared = item
            .filter(|_| self.physical_input.shift_held())
            .map(|item| comparisons(&item, &self.merchant.session.inventory, self.player_level()))
            .unwrap_or_default();
        let shopping = place_comparisons(&mut main, anchor, compared, screen);
        Ok(GameTooltipView {
            main,
            shopping,
            health,
        })
    }

    pub(crate) fn player_level(&self) -> Option<u16> {
        let id = self.world.local_player_id()?;
        let level = self.replica.unit(id)?.get::<UnitLevel>()?;
        Some(u16::from(level.0))
    }

    /// The topmost visible mouse-enabled frame under the pointer over every mounted
    /// registry but the tooltip's and the cursor item's: highest canvas layer, then strata,
    /// frame level and raise order.
    fn hovered_ui_frame(&mut self) -> Option<HoveredFrame> {
        let at = Vector2::from_array(self.physical_input.pointer());
        let skip = [self.tooltips.ui.clone(), self.bags.cursor.ui.clone()];
        let mut best: Option<((i32, (u8, i32, i32)), HoveredFrame)> = None;
        let _ = self.for_each_registry_ui(|ui| {
            if !ui.is_visible() || skip.iter().flatten().any(|other| other == ui) {
                return Ok(());
            }
            let Some((frame, order)) = ui.bind().pointer_frame_at(at) else {
                return Ok(());
            };
            let key = (ui.get_layer(), order);
            if best.as_ref().is_none_or(|(best, _)| key > *best) {
                best = Some((
                    key,
                    HoveredFrame {
                        ui: ui.clone(),
                        frame,
                    },
                ));
            }
            Ok(())
        });
        best.map(|(_, hit)| hit)
    }

    /// The owner rect `[x, y, w, h]` in UI units of frame `id` of `ui`.
    pub(crate) fn owner_rect(&self, ui: &Gd<RegistryUi>, id: u64) -> Option<[f32; 4]> {
        let rect = ui.bind().frame_id_rect(id)?;
        let scale = self.effective_ui_scale();
        Some([
            rect.position.x / scale,
            rect.position.y / scale,
            rect.size.x / scale,
            rect.size.y / scale,
        ])
    }

    /// `tooltip` owned by frame `id` of `hit`'s registry at `side`.
    pub(crate) fn owned_by(
        &self,
        hit: &HoveredFrame,
        id: u64,
        side: OwnerSide,
        tooltip: GameTooltip,
    ) -> Option<GameTooltip> {
        Some(tooltip.owned(self.owner_rect(&hit.ui, id)?, side))
    }

    fn sync_game_tooltip(&mut self, view: GameTooltipView) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.tooltips.ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(view);
        }
        if !view.main.visible {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(TOOLTIP_UI);
        ui.set_layer(TOOLTIP_LAYER);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_game_tooltip(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.tooltips.ui = Some(ui);
        Ok(())
    }

    /// The shown tooltip for automation: title, `left|right` lines, rect and the
    /// comparison titles.
    pub(crate) fn tooltip_snapshot(&self) -> VarDictionary {
        let view = &self.tooltips.shown;
        let mut state = VarDictionary::new();
        state.set("visible", view.main.visible);
        state.set("title", view.main.title.as_str());
        state.set("lines", &tooltip_lines(&view.main));
        state.set("rect", &rect_array(&view.main));
        state.set("health", view.health.map_or(-1.0, f64::from));
        let mut shopping = VarArray::new();
        for compare in view
            .shopping
            .iter()
            .filter(|compare| compare.tooltip.visible)
        {
            let mut entry = VarDictionary::new();
            entry.set("header", compare.header.as_str());
            entry.set("title", compare.tooltip.title.as_str());
            entry.set("lines", &tooltip_lines(&compare.tooltip));
            entry.set("rect", &rect_array(&compare.tooltip));
            shopping.push(&entry.to_variant());
        }
        state.set("shopping", &shopping);
        state
    }

    /// Title then `left|right` lines of the shown tooltip; empty while hidden.
    pub(crate) fn tooltip_text_lines(&self) -> Vec<String> {
        let main = &self.tooltips.shown.main;
        if !main.visible {
            return Vec::new();
        }
        std::iter::once(main.title.clone())
            .chain(
                main.lines
                    .iter()
                    .map(|line| format!("{}|{}", line.left_text, line.right_text)),
            )
            .collect()
    }
}

fn tooltip_lines(
    tooltip: &game_engine_ui_model::tooltip_presentation::TooltipPresentation,
) -> PackedStringArray {
    tooltip
        .lines
        .iter()
        .map(|line| GString::from(format!("{}|{}", line.left_text, line.right_text).as_str()))
        .collect()
}

fn rect_array(
    tooltip: &game_engine_ui_model::tooltip_presentation::TooltipPresentation,
) -> PackedFloat32Array {
    let size = [
        game_engine_ui_model::tooltip_presentation::TOOLTIP_W,
        tooltip.height(),
    ];
    PackedFloat32Array::from(&[tooltip.x, tooltip.y, size[0], size[1]][..])
}

/// The nearest frame from `id` up whose name `parse` accepts, and what it parsed.
pub(crate) fn named_ancestor<T>(
    registry: &ui_toolkit::registry::FrameRegistry,
    mut id: u64,
    parse: impl Fn(&Frame) -> Option<T>,
) -> Option<(u64, T)> {
    loop {
        let frame = registry.get(id)?;
        if let Some(value) = parse(frame) {
            return Some((id, value));
        }
        id = frame.parent_id?;
    }
}
