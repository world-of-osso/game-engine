//! Native Retail CharacterFrame (paperdoll) and micro menu. Equipment stays server-owned:
//! paperdoll clicks and drops go through the shared cursor item (`bag_cursor.rs`).
//! docs/specs/character-frame.md.

pub(crate) mod preview;

use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::bag_data::InventoryRequest;
use game_engine_ui_model::character_frame::{
    ACTION_CLOSE, ACTION_MODEL, CharacterFrameView, CharacterTab, MIN_LEVEL_FOR_ITEM_LEVEL,
    PAPERDOLL_BUTTONS, PORTRAIT, attribute_lines, average_equipped_item_level, class_background,
    enhancement_lines, level_line, paperdoll_slots, paperdoll_unequip_request,
    parse_equipment_slot_action, race_background, reputation_art_fdids, reputation_rows,
};
use game_engine_ui_model::cursor_item::{CursorItem, CursorTarget};
use game_engine_ui_model::damage_meter_data::class_color;
use game_engine_ui_model::item_catalog::item_catalog_entry_for;
use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::micro_menu::{
    ACTION_CHARACTER, ACTION_MAIN_MENU, ACTION_QUEST_LOG, CHARACTER_PORTRAIT, MICRO_BUTTONS,
    MicroMenuView, OpenWindows, micro_button_index, player_spells_micro_tab, unavailable_message,
};
use godot::prelude::*;
use shared::components::{CombatRatings, DerivedStats, Player, UnitLevel, UnitStats};
use shared::protocol::{EquipItem, ItemLocation};

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

const FRAME_UI: &str = "CharacterFrameUI";
const MICRO_UI: &str = "MicroMenuUI";
/// `OrbitCameraMixin:GetDeltaModifierForCameraMode` yaw: radians per UI unit dragged.
const ROTATE_PER_UNIT: f32 = 0.008;
/// Chrome, slot and backdrop art drawn before the frame first shows.
const FRAME_ART: [u32; 8] = [
    410_247, 410_248, 410_249, 651_080, 5_882_640, 1_400_895, 1_400_896, 374_154,
];

#[derive(Default)]
pub(crate) struct CharacterFrame {
    open: bool,
    /// The shown subframe.
    tab: CharacterTab,
    selected_reputation: Option<u32>,
    ui: Option<Gd<RegistryUi>>,
    micro_ui: Option<Gd<RegistryUi>>,
    /// Left button went down on the model scene and has not been released.
    rotating: bool,
    preview: preview::ModelPreview,
}

impl CharacterFrame {
    fn reset(&mut self) {
        // The preview view is a child of the paperdoll UI; release it before its parent.
        self.preview.reset();
        for ui in [self.ui.take(), self.micro_ui.take()].into_iter().flatten() {
            ui.free();
        }
        self.open = false;
        self.tab = CharacterTab::PaperDoll;
        self.selected_reputation = None;
        self.rotating = false;
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.ui, &mut self.micro_ui].into_iter().flatten() {
            visit(ui)?;
        }
        Ok(())
    }

    /// The character window canvas, once created.
    pub(crate) fn frame_ui(&self) -> Option<&Gd<RegistryUi>> {
        self.ui.as_ref()
    }

    /// The micro menu canvas, once shown.
    pub(crate) fn micro_ui(&self) -> Option<&Gd<RegistryUi>> {
        self.micro_ui.as_ref()
    }

    /// The PaperDollFrame shows: the frame is open on its Character tab.
    pub(crate) fn paperdoll_shown(&self) -> bool {
        self.open && self.tab == CharacterTab::PaperDoll
    }
}

impl GameClient {
    pub(super) fn update_character_frame(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            if let Some((id, _)) = self.character_frame.preview.pending.take() {
                self.world.cancel_detached_visual(id);
            }
            self.character_frame.reset();
            return Ok(());
        }
        self.sync_micro_menu()?;
        if self.account.session.gameplay_input_allowed() {
            self.poll_micro_menu_inputs()?;
        }
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            let input = self.physical_input.gameplay_state(self.keyboard_free());
            if self
                .client_options
                .bindings
                .is_just_pressed(InputAction::ToggleCharacter, &input)
            {
                self.toggle_character_frame();
            }
        }
        self.sync_character_frame_ui()?;
        self.rotate_character_model();
        Ok(self.sync_character_model()?)
    }

    /// `ToggleCharacter("PaperDollFrame")` (Mainline/CharacterFrame.lua:24-48): opens the
    /// frame on the paper doll, switches a frame shown on another tab to it, and hides
    /// a frame showing it.
    pub(crate) fn toggle_character_frame(&mut self) {
        if self.character_frame.open && self.character_frame.tab != CharacterTab::PaperDoll {
            self.show_character_tab(CharacterTab::PaperDoll);
            return;
        }
        self.character_frame.tab = CharacterTab::PaperDoll;
        self.character_frame.selected_reputation = None;
        self.character_frame.open = !self.character_frame.open;
        self.character_frame.rotating = false;
    }

    /// `CharacterFrame:ShowSubFrame` (CharacterFrame.lua:75-88) for a tab click: hiding
    /// the PaperDollFrame drops its model scene.
    fn show_character_tab(&mut self, tab: CharacterTab) {
        if self.character_frame.tab == tab {
            return;
        }
        if let Some((id, _)) = self.character_frame.preview.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        self.character_frame.preview.reset();
        self.character_frame.rotating = false;
        self.character_frame.tab = tab;
        self.character_frame.selected_reputation = None;
    }

    /// `CloseAllWindows` step: hides the frame. Returns whether it was open.
    pub(super) fn close_character_window(&mut self) -> bool {
        let open = self.character_frame.open;
        self.character_frame.open = false;
        self.character_frame.selected_reputation = None;
        self.character_frame.rotating = false;
        open
    }

    pub(super) fn character_frame_input_owner(&self, owner: i64) -> bool {
        self.character_frame
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
    }

    /// A press on the frame: paperdoll slots pick up or drop the cursor item
    /// (`PaperDollItemSlotButton_OnClick` → `PickupInventoryItem`), the model scene
    /// auto-equips it (`CharacterModelSceneMixin:OnMouseUp` → `TryAutoEquipCursorItem`)
    /// or starts rotating, the close button closes.
    pub(super) fn character_frame_click(
        &mut self,
        action: &str,
        click: Click,
    ) -> Result<(), FrameError> {
        if let Some(slot) = parse_equipment_slot_action(action) {
            if click.shift {
                return Ok(());
            }
            if click.right {
                let request =
                    paperdoll_unequip_request(&self.merchant.session.inventory, slot, click);
                return match request {
                    Ok(Some(request)) => Ok(self.account.send_inventory_request(&request)?),
                    Ok(None) => Ok(()),
                    Err(error) => Ok(self.add_world_error(error)?),
                };
            }
            let location = ItemLocation::Equipment(slot);
            if let Some(effect) = self.merchant.session.repair_click(location) {
                return Ok(self.apply_merchant_effect(effect)?);
            }
            let target = CursorTarget::Location(location);
            return self.send_cursor_click(target);
        }
        if let Some(tab) = CharacterTab::from_action(action) {
            self.show_character_tab(tab);
            return Ok(());
        }
        if !click.right
            && (action.starts_with("reputation_faction:")
                || action == game_engine_ui_model::character_frame::ACTION_REPUTATION_DETAIL_CLOSE)
        {
            let rows = self
                .reputation
                .as_ref()
                .map(|snapshot| reputation_rows(&snapshot.entries))
                .unwrap_or_default();
            self.character_frame.selected_reputation =
                game_engine_ui_model::character_frame::reputation_selection(
                    action,
                    &rows,
                    self.character_frame.selected_reputation,
                );
            if let Some(ui) = self.character_frame.ui.as_mut() {
                ui.bind_mut().reset_reputation_description_scroll();
            }
            return Ok(());
        }
        match action {
            ACTION_CLOSE => {
                self.character_frame.open = false;
                self.character_frame.selected_reputation = None;
            }
            ACTION_MODEL if !click.right && !click.shift => self.press_character_model()?,
            _ => {}
        }
        Ok(())
    }

    fn press_character_model(&mut self) -> Result<(), FrameError> {
        let CursorItem::Inventory {
            from, split: false, ..
        } = self.bags.cursor.item
        else {
            self.character_frame.rotating = self.bags.cursor.item.is_empty();
            return Ok(());
        };
        self.bags.cursor.item = CursorItem::Empty;
        let request = InventoryRequest::Equip(EquipItem { from });
        Ok(self.account.send_inventory_request(&request)?)
    }

    fn poll_micro_menu_inputs(&mut self) -> Result<(), FrameError> {
        if let Some(mut micro) = self.character_frame.micro_ui.clone() {
            loop {
                let action = micro.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                self.micro_button_click(&action)?;
            }
        }
        Ok(())
    }

    /// A micro-menu button toggles its native window; a button whose window is not
    /// converted yet shows its Retail unavailable line in the error frame.
    pub(super) fn micro_button_click(&mut self, action: &str) -> Result<(), FrameError> {
        if let Some(tab) = player_spells_micro_tab(action) {
            if self.spellbook_open() {
                self.close_spellbook();
            } else {
                self.toggle_player_spells(tab)?;
            }
            return Ok(());
        }
        match action {
            ACTION_CHARACTER => self.toggle_character_frame(),
            ACTION_QUEST_LOG => self.toggle_quest_log(),
            ACTION_MAIN_MENU => self.toggle_game_menu_from_micro_button()?,
            "micro:GuildMicroButton" => self.toggle_guild_ranks()?,
            "micro:CollectionsMicroButton" => self.toggle_pet_journal(),
            game_engine_ui_model::achievements::OPEN_ACTION => self.toggle_achievements()?,
            _ => match unavailable_message(action) {
                Some(message) => self.add_world_error(&message)?,
                None => return Err(format!("Unknown micro menu action {action}").into()),
            },
        }
        Ok(())
    }

    /// `MainMenuMicroButtonMixin:OnClick`: hides a shown GameMenuFrame, else
    /// `CloseAllWindows` and shows it.
    fn toggle_game_menu_from_micro_button(&mut self) -> Result<(), FrameError> {
        if self.game_menu_ui.is_some() {
            self.close_game_menu();
            return Ok(());
        }
        self.close_all_windows()?;
        Ok(self.open_game_menu()?)
    }

    pub(crate) fn micro_menu_view(&self) -> MicroMenuView {
        let (hovered, pressed) = self
            .character_frame
            .micro_ui
            .as_ref()
            .map(|ui| {
                let ui = ui.bind();
                let hovered = ui.hovered_button().map(|(name, _)| name);
                (hovered, ui.pushed_button())
            })
            .unwrap_or_default();
        MicroMenuView {
            open: OpenWindows {
                character: self.character_frame.open,
                player_spells: self.spellbook_open(),
                quest_log: self.quests.log_open(),
                game_menu: self.game_menu_ui.is_some(),
            },
            hovered: hovered.as_deref().and_then(micro_button_index),
            pressed: pressed.as_deref().and_then(micro_button_index),
        }
    }

    fn sync_micro_menu(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let view = self.micro_menu_view();
        if let Some(ui) = self.character_frame.micro_ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(view);
        }
        self.drawable_fdid(4_708_813);
        self.drawable_fdid(CHARACTER_PORTRAIT.mask_fdid);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(MICRO_UI);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_micro_menu(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.character_frame.micro_ui = Some(ui);
        Ok(())
    }

    fn sync_character_frame_ui(&mut self) -> Result<(), String> {
        let view = self.character_frame_view()?;
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.character_frame.ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(view);
        }
        if !view.visible {
            return Ok(());
        }
        for fdid in FRAME_ART {
            self.drawable_fdid(fdid);
        }
        self.drawable_fdid(PORTRAIT.mask_fdid);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(FRAME_UI);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_character_frame(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.character_frame.ui = Some(ui);
        Ok(())
    }

    fn character_frame_view(&mut self) -> Result<CharacterFrameView, String> {
        if !self.character_frame.open {
            return Ok(CharacterFrameView::default());
        }
        let unit = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id));
        let player = unit.and_then(|unit| unit.get::<Player>()).cloned();
        let level = unit
            .and_then(|unit| unit.get::<UnitLevel>())
            .map_or(1, |level| level.0);
        let sheet =
            unit.and_then(|unit| Some((unit.get::<UnitStats>()?, unit.get::<CombatRatings>()?)));
        let spec_primary = self.spells.catalog().and_then(|data| {
            let spec = data.tabs.specs.get(&self.account.spells.spec()?)?;
            Some(spec.primary_stat())
        });
        let attributes = attribute_lines(sheet, spec_primary);
        let enhancements = enhancement_lines(unit.and_then(|unit| unit.get::<DerivedStats>()));
        let (race_id, class_id, title) = player.as_ref().map_or((0, 0, String::new()), |player| {
            (player.race, player.class, player.name.clone())
        });
        let inventory = &self.merchant.session.inventory;
        let mut slots = paperdoll_slots(inventory, self.bags.cursor.item.source());
        for slot in &mut slots {
            slot.icon_fdid = self.drawable_fdid(slot.icon_fdid);
        }
        self.draw_character_backdrops(race_id, class_id);
        let tab = self.character_frame.tab;
        let mut reputation = match (tab, &self.reputation) {
            (CharacterTab::Reputation, Some(snapshot)) => reputation_rows(&snapshot.entries),
            _ => Vec::new(),
        };
        if tab == CharacterTab::Reputation {
            game_engine_ui_model::character_frame::enrich_reputation_rows(
                &mut reputation,
                race_id,
                class_id,
            )?;
            for fdid in reputation_art_fdids() {
                self.drawable_fdid(fdid);
            }
        }
        Ok(CharacterFrameView {
            visible: true,
            tab,
            reputation,
            selected_reputation: self.character_frame.selected_reputation,
            title,
            level: self.character_level_line(level, class_id),
            slots,
            item_level: (level >= MIN_LEVEL_FOR_ITEM_LEVEL)
                .then(|| self.equipped_item_level().to_string()),
            attributes,
            enhancements,
            race_id,
            class_id,
        })
    }

    /// `PaperDollFrame_SetLevel`: the spec and class names from the spell catalog.
    fn character_level_line(
        &self,
        level: u8,
        class_id: u8,
    ) -> game_engine_ui_model::character_frame::LevelLine {
        let tabs = self.spells.catalog().map(|data| &data.tabs);
        let class = tabs
            .and_then(|tabs| tabs.class_names.get(&u32::from(class_id)))
            .map_or("", String::as_str);
        let spec = tabs.and_then(|tabs| tabs.spec_name(self.account.spells.spec()));
        level_line(level, spec, class, class_color(class_id))
    }

    /// `PaperDollFrame_SetItemLevel`: `floor` of the equipped average.
    fn equipped_item_level(&self) -> u32 {
        let average = average_equipped_item_level(&self.merchant.session.inventory, |item| {
            item_catalog_entry_for(item.definition_source, item.item_id)
                .map(|entry| (entry.item_level, entry.inventory_type))
        });
        average.floor() as u32
    }

    fn draw_character_backdrops(&mut self, race_id: u8, class_id: u8) {
        if let Some(first) = race_background(race_id) {
            for quarter in first..first + 4 {
                self.drawable_fdid(quarter);
            }
        }
        if let Some(art) = class_background(class_id) {
            self.drawable_fdid(art.fdid);
        }
        for button in &PAPERDOLL_BUTTONS {
            self.drawable_fdid(button.empty_texture);
        }
    }

    /// Reputation uses the same Retail ScrollBar input as Options and QuestFrame.
    pub(super) fn character_reputation_pointer(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> bool {
        let reputation_open =
            self.character_frame.open && self.character_frame.tab == CharacterTab::Reputation;
        if self.game_menu_ui.is_some() || !reputation_open {
            return false;
        }
        let Some(ui) = self.character_frame.ui.as_mut() else {
            return false;
        };
        let taken = ui.bind_mut().scroll_list_input(event);
        match taken {
            Ok(false) => false,
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                true
            }
            Err(error) => {
                godot_error!("Reputation scroll: {error}");
                true
            }
        }
    }

    /// Left-drag on the model scene turns it (`OrbitCameraMixin:OnUpdate`).
    /// The frame's pointer release ends a model drag: a press the frame consumed never
    /// holds the gameplay Left button.
    pub(super) fn release_character_frame_pointer(&mut self) {
        self.character_frame.rotating = false;
    }

    fn rotate_character_model(&mut self) {
        if !self.character_frame.rotating {
            return;
        }
        let delta = self.physical_input.motion()[0] / self.effective_ui_scale();
        self.character_frame.preview.yaw += delta * ROTATE_PER_UNIT;
    }
}

impl GameClient {
    /// Open state, the frame's slot icons and the model preview, for automation.
    pub(super) fn character_frame_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("open", self.character_frame.open);
        state.set("tab", format!("{:?}", self.character_frame.tab).as_str());
        let view = self.micro_menu_view();
        let mut micro = VarDictionary::new();
        for (index, button) in MICRO_BUTTONS.iter().enumerate() {
            micro.set(button.name, format!("{:?}", view.state(index)).as_str());
        }
        state.set("micro", &micro);
        self.character_frame.preview.snapshot(&mut state);
        state.set(
            "world_slots",
            &preview::appearance_slots(self.world.local_player_appearance()),
        );
        state
    }
}
