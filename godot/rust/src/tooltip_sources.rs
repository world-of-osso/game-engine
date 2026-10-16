//! Which tooltip each hovered frame shows, with the owner and anchor of its Retail
//! `OnEnter`: action buttons and chat spell links at the default anchor
//! (ActionButton.lua:1070-1080 with `UberTooltips` 1), spellbook items `ANCHOR_RIGHT`
//! (Blizzard_SpellBookItem.lua:494), bag slots by screen half (ContainerFrame.lua:1448-1458),
//! merchant, loot, mail, auction and quest reward items `ANCHOR_RIGHT` (MerchantFrame.lua:710-711,
//! LootFrame.lua:342-357, MailFrame.lua:354-378 and 903-911, AuctionHouseUtil.lua:83,
//! QuestInfo.lua:1240-1258), the bag bar `ANCHOR_LEFT` (MainMenuBarBagButtons.lua:102-126 and 243-256), player auras
//! `ANCHOR_BOTTOMLEFT` (BuffFrame.lua:888-899), target auras by their centre
//! (TargetFrame.xml:35-40) and the minimap buttons as Minimap.lua sets them.

use game_engine_core::spell_catalog::{CasterPower, SpellTextContext};
use game_engine_ui_model::bag_data::{InventorySlot, InventoryState};
use game_engine_ui_model::buff_frame_component::buff_button_at;
use game_engine_ui_model::character_frame::{paperdoll_button, parse_equipment_slot_action};
use game_engine_ui_model::chat_frame_component::chat_spell_link_at;
use game_engine_ui_model::game_tooltip::hud::{
    backpack_tooltip, calendar_tooltip, clock_tooltip, empty_bag_slot_tooltip,
    empty_paperdoll_slot_tooltip, minimap_mouseover_tooltip, tracking_tooltip, twelve_hour_time,
    unread_mail_tooltip, zoom_tooltip,
};
use game_engine_ui_model::game_tooltip::item::{
    auction_row_item, item_game_tooltip, named_item, named_item_for, without_sell_price,
};
use game_engine_ui_model::game_tooltip::merchant::{
    buyback_item, guild_repair_tooltip, merchant_cell_item, repair_all_tooltip,
    repair_item_tooltip, sell_all_junk_tooltip,
};
use game_engine_ui_model::game_tooltip::spell::{
    SpellTooltipInput, aura_tooltip, spell_tooltip, unknown_spell_tooltip,
};
use game_engine_ui_model::game_tooltip::{GameTooltip, OwnerSide};
use game_engine_ui_model::inworld_unit_frames_component::{TargetAuraView, target_frame_auras};
use game_engine_ui_model::item_catalog::item_catalog_entry;
use game_engine_ui_model::mail_frame_component::ACTION_OPEN_PREFIX;
use game_engine_ui_model::main_action_bar_component::parse_action_button;
use game_engine_ui_model::micro_menu::{MICRO_BUTTONS, micro_button_index, micro_button_tooltip};
use game_engine_ui_model::minimap::{MINIMAP_ZOOM_IN, MINIMAP_ZOOM_OUT};
use game_engine_ui_model::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, TooltipPresentation,
};
use shared::protocol::{ActionRef, ItemLocation, LootContent, MailAttachment, MailHeader};
use ui_toolkit::frame::Frame;

use crate::GameClient;
use crate::faction_reaction::Reaction;
use crate::replicated::UnitFields;
use crate::tooltips::{HoveredFrame, HoveredTooltip, named_ancestor};

/// `ENCLOSED_MONEY`, `COD_AMOUNT`, `MAIL_MULTIPLE_ITEMS`.
const ENCLOSED_MONEY: &str = "Money Enclosed:";
const COD_AMOUNT: &str = "Amount Due:";
const MAIL_MULTIPLE_ITEMS: &str = "Multiple Items";

fn name_of(frame: &Frame) -> Option<&str> {
    frame.name.as_deref()
}

/// The 1-based index after `prefix` in a frame name that is exactly `{prefix}{n}`.
fn indexed(frame: &Frame, prefix: &str) -> Option<usize> {
    name_of(frame)?.strip_prefix(prefix)?.parse().ok()
}

enum HousingTooltipTarget {
    EditorControl(game_engine_core::input_bindings_data::InputAction),
    Decor(u32),
    Chest,
    Category(u32),
    Subcategory(u32),
    AllSubcategories,
}

fn parse_housing_tooltip_target(frame: &Frame) -> Option<HousingTooltipTarget> {
    let name = name_of(frame)?;
    if let Some((_, action, _)) = game_engine_ui_model::housing::EDITOR_CONTROL_TOOLTIPS
        .iter()
        .find(|(control, _, _)| *control == name)
    {
        return Some(HousingTooltipTarget::EditorControl(*action));
    }
    if name == "HouseEditorStorageExpandButton" {
        return Some(HousingTooltipTarget::Chest);
    }
    if matches!(
        name,
        "HousingCatalogSubcategoryAll" | "HouseEditorStorageSubcategoryAll"
    ) {
        return Some(HousingTooltipTarget::AllSubcategories);
    }
    for (prefix, kind) in [
        (
            "HousingCatalogEntry",
            HousingTooltipTarget::Decor as fn(u32) -> HousingTooltipTarget,
        ),
        ("HouseEditorStored", HousingTooltipTarget::Decor),
        ("HousingCatalogCategory", HousingTooltipTarget::Category),
        ("HouseEditorStorageCategory", HousingTooltipTarget::Category),
        (
            "HousingCatalogSubcategory",
            HousingTooltipTarget::Subcategory,
        ),
        (
            "HouseEditorStorageSubcategory",
            HousingTooltipTarget::Subcategory,
        ),
    ] {
        if let Some(value) = name.strip_prefix(prefix) {
            return value.parse().ok().map(kind);
        }
    }
    None
}

impl GameClient {
    /// The tooltip of the hovered frame, if it has one.
    pub(crate) fn frame_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        type Source = fn(&mut GameClient, &HoveredFrame) -> Option<HoveredTooltip>;
        const SOURCES: &[Source] = &[
            GameClient::trading_post_button_tooltip,
            GameClient::trading_post_tooltip,
            GameClient::journal_defeated_tooltip,
            GameClient::journal_creature_tooltip,
            GameClient::journal_monthly_tooltip,
            GameClient::journal_journey_tooltip,
            GameClient::journal_set_tooltip,
            GameClient::journal_loot_tooltip,
            GameClient::transmog_set_tooltip,
            GameClient::transmog_source_tooltip,
            GameClient::transmog_illusion_tooltip,
            GameClient::housing_tooltip,
            GameClient::toybox_tooltip,
            GameClient::action_button_tooltip,
            GameClient::mentor_teleport_tooltip,
            GameClient::talent_gate_tooltip,
            GameClient::profession_reagent_quality_tooltip,
            GameClient::profession_specialization_tooltip,
            GameClient::spellbook_tooltip,
            GameClient::trainer_tooltip,
            GameClient::chat_link_tooltip,
            GameClient::bag_slot_tooltip,
            GameClient::paperdoll_tooltip,
            GameClient::merchant_tooltip,
            GameClient::merchant_button_tooltip,
            GameClient::group_loot_tooltip,
            GameClient::loot_tooltip,
            GameClient::mail_attachment_tooltip,
            GameClient::inbox_item_tooltip,
            GameClient::auction_tooltip,
            GameClient::quest_reward_tooltip,
            GameClient::bag_bar_tooltip,
            GameClient::micro_menu_tooltip,
            GameClient::player_aura_tooltip,
            GameClient::target_aura_tooltip,
            GameClient::xp_bar_tooltip,
            GameClient::hud_frame_tooltip,
        ];
        SOURCES.iter().find_map(|source| source(self, hit))
    }

    /// PerksProgram button ShowTooltip methods own ANCHOR_LEFT, including disabled controls.
    fn trading_post_button_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        use ui_toolkit::{frame::WidgetData, widgets::button::ButtonState};
        let ui = hit.ui.bind();
        let (owner, tooltip) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let WidgetData::Button(button) = frame.widget_data.as_ref()? else {
                return None;
            };
            let enabled = button.state != ButtonState::Disabled;
            self.trading_post
                .view
                .button_tooltip(name_of(frame)?, enabled)
        })?;
        drop(ui);
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Left,
            tooltip,
        )?))
    }

    /// PerksProgramElements.lua306–314: hovered product GetItemByID, no SellPrice.
    fn trading_post_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, product_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            indexed(frame, "PerksProgramProduct")
        })?;
        drop(ui);
        let snapshot = self.trading_post.view.snapshot.as_ref()?;
        let product = snapshot
            .items
            .iter()
            .find(|row| row.id as usize == product_id)?;
        let item = named_item_for(
            shared::item_data::ItemDefinitionSource::Retail,
            product.item_id,
            &product.name,
            product.quality,
            1,
            0,
        );
        let tooltip = without_sell_price(item_game_tooltip(&item, self.player_level()));
        let owner_rect = self.owner_rect(&hit.ui, owner)?;
        Some(HoveredTooltip::text(tooltip.owned_offset(
            owner_rect,
            OwnerSide::Right,
            [-16.0, 0.0],
        )))
    }

    /// Catalog/stored decor and the source open-storage handle use `ANCHOR_RIGHT`.
    fn housing_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, target) =
            named_ancestor(ui.registry()?, hit.frame, parse_housing_tooltip_target)?;
        let editor_control = matches!(target, HousingTooltipTarget::EditorControl(_));
        if editor_control {
            let frame = ui.registry()?.get(owner)?;
            let Some(ui_toolkit::frame::WidgetData::Button(button)) = &frame.widget_data else {
                return None;
            };
            if button.state == ui_toolkit::widgets::button::ButtonState::Disabled {
                return None;
            }
        }
        drop(ui);
        let (tooltip, category) = self.housing_tooltip_content(target)?;
        let rect = self.owner_rect(&hit.ui, owner)?;
        let tooltip = if editor_control {
            tooltip.owned(rect, OwnerSide::Top)
        } else if category {
            tooltip.owned_offset(rect, OwnerSide::Right, [-12.0, 12.0])
        } else {
            tooltip.owned(rect, OwnerSide::Right)
        };
        Some(HoveredTooltip::text(tooltip))
    }

    fn housing_tooltip_content(&self, target: HousingTooltipTarget) -> Option<(GameTooltip, bool)> {
        let category = matches!(
            target,
            HousingTooltipTarget::Category(_)
                | HousingTooltipTarget::Subcategory(_)
                | HousingTooltipTarget::AllSubcategories
        );
        let tooltip = match target {
            HousingTooltipTarget::EditorControl(action) => self
                .housing
                .model
                .as_ref()?
                .editor_control_tooltip(action, self.client_options.bindings.binding(action))?,
            HousingTooltipTarget::Chest => game_engine_ui_model::housing::storage_handle_tooltip(),
            HousingTooltipTarget::Decor(id) => {
                let model = self.housing.model.as_ref()?;
                let decor = model.catalog.decor.iter().find(|decor| decor.id == id)?;
                game_engine_ui_model::housing::decor_entry_tooltip(
                    decor,
                    &model.catalog,
                    &self.housing.authority,
                )
            }
            category => self.housing_category_tooltip(category)?,
        };
        Some((tooltip, category))
    }

    fn housing_category_tooltip(&self, target: HousingTooltipTarget) -> Option<GameTooltip> {
        let catalog = &self.housing.model.as_ref()?.catalog;
        let name = match target {
            HousingTooltipTarget::Category(id) => {
                &catalog.categories.iter().find(|row| row.id == id)?.name
            }
            HousingTooltipTarget::Subcategory(id) => {
                &catalog.subcategories.iter().find(|row| row.id == id)?.name
            }
            HousingTooltipTarget::AllSubcategories => {
                &catalog
                    .categories
                    .iter()
                    .find(|row| row.id == game_engine_ui_model::housing::ALL_CATEGORY)?
                    .name
            }
            _ => return None,
        };
        Some(GameTooltip::new(
            TooltipPresentation {
                title: name.clone(),
                title_color: game_engine_ui_model::tooltip_presentation::TOOLTIP_WHITE,
                ..TooltipPresentation::hidden()
            },
            None,
        ))
    }

    fn journal_defeated_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, tooltip) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            game_engine_ui_model::adventure_defeated::tooltip(
                &self.adventure_guide.view,
                frame.name.as_deref()?,
            )
        })?;
        drop(ui);
        let rect = self.owner_rect(&hit.ui, owner)?;
        Some(HoveredTooltip::text(tooltip.owned(rect, OwnerSide::Right)))
    }

    fn journal_creature_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, tooltip) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            game_engine_ui_model::adventure_creatures::tooltip(
                &self.adventure_guide.view,
                frame.name.as_deref()?,
            )
        })?;
        drop(ui);
        let rect = self.owner_rect(&hit.ui, owner)?;
        Some(HoveredTooltip::text(tooltip.owned(rect, OwnerSide::Right)))
    }

    fn journal_monthly_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let snapshot = self.adventure_guide.view.activities.as_deref()?;
        let ui = hit.ui.bind();
        let (_, tooltip) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            game_engine_ui_model::adventure_activities::activity_tooltip(
                snapshot,
                frame.name.as_deref()?,
            )
        })?;
        Some(HoveredTooltip::text(tooltip))
    }

    /// SetRewards uses ANCHOR_RIGHT; LockedState's requirement uses ANCHOR_CURSOR_RIGHT.
    fn journal_journey_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, (mut tooltip, at_cursor)) =
            named_ancestor(ui.registry()?, hit.frame, |frame| {
                self.journey_tooltip_for_frame(frame)
            })?;
        drop(ui);
        if at_cursor {
            tooltip.anchor = game_engine_ui_model::game_tooltip::TooltipAnchor::CursorRight;
        } else {
            let rect = self.owner_rect(&hit.ui, owner)?;
            tooltip = tooltip.owned(rect, OwnerSide::Right);
        }
        Some(HoveredTooltip::text(tooltip))
    }

    fn journey_tooltip_for_frame(&self, frame: &Frame) -> Option<(GameTooltip, bool)> {
        let view = &self.adventure_guide.view;
        let name = frame.name.as_deref()?;
        game_engine_ui_model::adventure_journeys::reward_tooltip(
            &view.journeys,
            view.renown.as_deref(),
            name,
        )
        .map(|tooltip| (tooltip, false))
    }

    fn journal_loot_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, item) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let id = frame
                .name
                .as_deref()?
                .strip_prefix("JournalLoot")?
                .parse::<u32>()
                .ok()?;
            self.adventure_guide
                .view
                .loot()
                .into_iter()
                .find(|item| item.id == id)
        })?;
        let slot = named_item_for(
            shared::item_data::ItemDefinitionSource::Retail,
            item.id,
            &item.name,
            item.quality as u8,
            1,
            0,
        );
        drop(ui);
        self.item_owned(hit, owner, OwnerSide::Right, slot)
    }

    fn journal_set_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, (set, item)) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            self.adventure_guide.view.set_member(frame.name.as_deref()?)
        })?;
        let slot = named_item_for(
            shared::item_data::ItemDefinitionSource::Retail,
            item.id,
            &item.name,
            item.quality as u8,
            1,
            0,
        );
        drop(ui);
        let descriptions = self.journal_bonus_descriptions(set.id);
        let lines = game_engine_ui_model::adventure_guide_tooltip::set_lines(
            &self.adventure_guide.view,
            set,
            &self.merchant.session.inventory,
            &descriptions,
        );
        let mut hovered = self.item_owned(hit, owner, OwnerSide::Right, slot)?;
        hovered.tooltip.content.lines.extend(lines);
        Some(hovered)
    }

    /// TransmogSetModelMixin:OnEnter -> RefreshTooltip, `ANCHOR_RIGHT`.
    fn transmog_set_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, set_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            name_of(frame)?
                .strip_prefix("TransmogSet")?
                .parse::<u32>()
                .ok()
        })?;
        drop(ui);
        let set = self
            .transmog
            .book
            .state
            .as_ref()?
            .sets
            .iter()
            .find(|set| set.set_id == set_id)?;
        let tooltip = game_engine_ui_model::game_tooltip::transmog::transmog_set_tooltip(set);
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    /// ItemModelBaseMixin:OnEnter and TransmogAppearanceSlotMixin item tooltips.
    fn transmog_source_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        enum Target {
            Source(u32),
            Slot(u8),
        }
        let ui = hit.ui.bind();
        let (owner, target) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            if let Some(id) = name.strip_prefix("TransmogSource") {
                id.parse().ok().map(Target::Source)
            } else {
                name.strip_prefix("TransmogSlot")?
                    .parse()
                    .ok()
                    .map(Target::Slot)
            }
        })?;
        drop(ui);
        let book = &self.transmog.book;
        let state = book.state.as_ref()?;
        let source = match target {
            Target::Source(id) => state
                .slots
                .iter()
                .flat_map(|slot| &slot.sources)
                .find(|source| source.source_id == id),
            Target::Slot(inv) => {
                let slot = state
                    .slots
                    .iter()
                    .find(|slot| slot.slot.inv_slot_id() == inv)?;
                match book.effective_choice(slot) {
                    shared::protocol::TransmogChoice::Source(id) => {
                        slot.sources.iter().find(|source| source.source_id == id)
                    }
                    _ => None,
                }
            }
        };
        let (definition, item, name) = if let Some(source) = source {
            (
                shared::item_data::ItemDefinitionSource::Retail,
                source.item_id,
                source.name.clone(),
            )
        } else if let Target::Slot(inv) = target {
            let slot = state
                .slots
                .iter()
                .find(|slot| slot.slot.inv_slot_id() == inv)?;
            let catalog = game_engine_ui_model::item_catalog::item_catalog_entry_for(
                slot.definition_source,
                slot.item_id,
            )?;
            (slot.definition_source, slot.item_id, catalog.name.clone())
        } else {
            return None;
        };
        let quality =
            game_engine_ui_model::item_catalog::item_catalog_entry_for(definition, item)?.quality;
        let item = named_item_for(definition, item, &name, quality, 1, 0);
        self.item_owned(hit, owner, OwnerSide::Right, item)
    }

    /// An illusion card or a weapon's illusion slot, `ANCHOR_RIGHT`.
    fn transmog_illusion_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        enum Hovered {
            Card(u32),
            Slot(u8),
        }
        let ui = hit.ui.bind();
        let (owner, hovered) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            match name.strip_prefix("TransmogIllusionEntry") {
                Some(id) => id.parse().ok().map(Hovered::Card),
                None => name
                    .strip_prefix("TransmogIllusion")?
                    .parse()
                    .ok()
                    .map(Hovered::Slot),
            }
        })?;
        drop(ui);
        let book = &self.transmog.book;
        let name = match hovered {
            Hovered::Card(id) => Some(book.illusion(id)?.name.clone()),
            Hovered::Slot(inv) => {
                let slot = shared::protocol::EquipmentSlot::ALL
                    .into_iter()
                    .find(|slot| slot.inv_slot_id() == inv)?;
                book.effective_illusion(slot)
                    .and_then(|id| book.illusion(id))
                    .map(|illusion| illusion.name.clone())
            }
        };
        let tooltip = game_engine_ui_model::game_tooltip::transmog::transmog_illusion_tooltip(
            name.as_deref(),
        );
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    fn journal_bonus_descriptions(&self, set: u32) -> std::collections::BTreeMap<u32, String> {
        let catalog = self
            .spells
            .catalog()
            .expect("Loaded Journal requires spell catalog");
        let context = SpellTextContext {
            known_spells: self.account.spells.known().to_vec(),
            auras: Vec::new(),
            spec_id: (self.adventure_guide.view.spec != 0)
                .then_some(self.adventure_guide.view.spec),
            caster_power: self.local_caster_power(),
        };
        self.adventure_guide
            .view
            .catalog
            .set_bonuses
            .get(&set)
            .into_iter()
            .flatten()
            .filter_map(
                |bonus| match catalog.render_description(bonus.spell, &context) {
                    Some(text) => Some((bonus.spell, text)),
                    None => {
                        crate::frame_error::report_once(&format!(
                            "Journal set{set} bonus{}: description unavailable",
                            bonus.spell
                        ));
                        None
                    }
                },
            )
            .collect()
    }

    fn spell_game_tooltip(&self, spell_id: u32, available_at: Option<u32>) -> GameTooltip {
        let remaining = self
            .account
            .spells
            .button_cooldown(spell_id, false)
            .map(|timer| timer.remaining);
        self.spell_game_tooltip_with_cooldown(spell_id, available_at, remaining)
    }

    fn spell_game_tooltip_with_cooldown(
        &self,
        spell_id: u32,
        available_at: Option<u32>,
        cooldown_remaining: Option<f32>,
    ) -> GameTooltip {
        let Some(catalog) = self.spells.catalog() else {
            return unknown_spell_tooltip(spell_id);
        };
        let Some(spell) = catalog.get(spell_id) else {
            return unknown_spell_tooltip(spell_id);
        };
        let context = SpellTextContext {
            known_spells: self.account.spells.known().to_vec(),
            auras: Vec::new(),
            spec_id: self.account.spells.spec(),
            caster_power: self.local_caster_power(),
        };
        let input = SpellTooltipInput {
            description: catalog
                .render_description(spell_id, &context)
                .unwrap_or_default(),
            available_at,
            cooldown_remaining,
        };
        spell_tooltip(spell, &input)
    }

    /// The local player's replicated `DerivedStats` powers.
    fn local_caster_power(&self) -> Option<CasterPower> {
        let unit = self.replica.unit(self.world.local_player_id()?)?;
        let derived = unit.get::<shared::components::DerivedStats>()?;
        Some(CasterPower {
            spell_power: derived.spell_power,
            attack_power: derived.attack_power,
        })
    }

    fn mentor_teleport_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        use game_engine_ui_model::mythic_mentor::{
            TELEPORT_ACTION_PREFIX, mentor_teleport_tooltip,
        };
        let ui = hit.ui.bind();
        let (owner, spell_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            frame
                .onclick
                .as_deref()?
                .strip_prefix(TELEPORT_ACTION_PREFIX)?
                .parse::<u32>()
                .ok()
        })?;
        drop(ui);
        let remaining = self
            .account
            .spells
            .category_cooldown(game_engine_ui_model::mythic_mentor::TELEPORT_CATEGORY)
            .map(|timer| timer.remaining);
        let tooltip = mentor_teleport_tooltip(
            self.spell_game_tooltip_with_cooldown(spell_id, None, remaining),
            self.account.spells.known().contains(&spell_id),
        );
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    fn action_button_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (_, (bar, index)) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            parse_action_button(frame.onclick.as_deref()?)
        })?;
        match self.action_slot(self.bar_slot(bar, index))? {
            ActionRef::Spell(spell_id) => Some(HoveredTooltip::text(
                self.spell_game_tooltip(spell_id, None),
            )),
            ActionRef::Item(item_id) => Some(HoveredTooltip::text(self.toy_game_tooltip(item_id)?)),
            ActionRef::Macro(_) => None,
        }
    }

    fn toy_game_tooltip(&self, item_id: u32) -> Option<GameTooltip> {
        let toy = self.toybox.model.toy(item_id)?;
        let text = self.toybox.model.tooltip(item_id)?;
        let text = text
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .replace("|n", "\n");
        let mut lines = game_engine_ui_model::tooltip_presentation::description_lines(
            &text,
            TOOLTIP_DESCRIPTION_COLOR,
        );
        if let Some(spell_id) = toy.spell_id {
            lines.extend(self.spell_game_tooltip(spell_id, None).content.lines);
        }
        Some(GameTooltip::new(
            TooltipPresentation {
                title: toy.name.clone(),
                lines,
                ..Default::default()
            },
            Some(game_engine_ui_model::game_tooltip::TooltipRecord::Item(
                item_id,
            )),
        ))
    }

    fn toybox_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, item_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            frame
                .onclick
                .as_deref()?
                .strip_prefix(game_engine_ui_model::toybox::USE_PREFIX)?
                .parse::<u32>()
                .ok()
        })?;
        drop(ui);
        let tooltip = self.toy_game_tooltip(item_id)?;
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    fn profession_reagent_quality_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, item_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            name_of(frame)?
                .strip_prefix("ProfessionQuality")?
                .strip_suffix("Choice")?
                .parse::<u32>()
                .ok()
        })?;
        drop(ui);
        let item = game_engine_ui_model::item_catalog::item_catalog_entry(item_id)?;
        let slot = named_item(item_id, &item.name, item.quality, 1);
        // QualityDialog.lua81-84: GameTooltip owner ANCHOR_TOPLEFT, actual reagent item.
        self.item_owned(hit, owner, OwnerSide::TopLeft, slot)
    }

    fn profession_specialization_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, node) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            let id = name
                .strip_prefix("ProfessionSpecNode")
                .or_else(|| name.strip_prefix("ProfessionSpecPerk"))?;
            id.parse::<u32>().ok()
        })?;
        drop(ui);
        let tooltip = self.professions.specializations.tooltip(node)?;
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    fn talent_gate_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        if let Some((owner, export)) =
            named_ancestor(ui.registry()?, hit.frame, |frame| match name_of(frame)? {
                "TalentExportLoadout" => Some(true),
                "TalentStarterBuild" => Some(false),
                _ => None,
            })
        {
            let view = self.spells.projected_talents()?;
            if export && view.editor.can_export(view) {
                return None;
            }
            drop(ui);
            let tooltip = GameTooltip::new(
                TooltipPresentation {
                    title: if export {
                        "You must spend all available talent points to share this loadout"
                    } else {
                        "A preset talent build that's suitable for most content"
                    }
                    .into(),
                    title_color: if export {
                        game_engine_ui_model::tooltip_presentation::TOOLTIP_WHITE
                    } else {
                        game_engine_ui_model::tooltip_presentation::TOOLTIP_DESCRIPTION_COLOR
                    },
                    ..TooltipPresentation::hidden()
                },
                None,
            );
            return Some(HoveredTooltip::text(self.owned_by(
                hit,
                owner,
                OwnerSide::Right,
                tooltip,
            )?));
        }
        let (owner, condition) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            name_of(frame)?
                .strip_prefix("TalentGate")?
                .strip_suffix("Button")?
                .parse::<u32>()
                .ok()
        })?;
        drop(ui);
        let tooltip = self.spells.projected_talents()?.gate_tooltip(condition)?;
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Left,
            tooltip,
        )?))
    }
    fn spellbook_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, (spell_id, talent)) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            if let Some(spell) = game_engine_ui_model::talents::talent_button_spell(name) {
                return Some((spell, true));
            }
            name.strip_prefix("SpellBookItem")?
                .strip_suffix("Button")?
                .parse::<u32>()
                .ok()
                .map(|spell| (spell, false))
        })?;
        let talent_entry = if talent {
            let name = ui.registry()?.get(owner)?.name.as_deref()?;
            Some((
                game_engine_ui_model::talents::talent_button_node(name)?,
                game_engine_ui_model::talents::talent_button_entry(name)?,
            ))
        } else {
            None
        };
        drop(ui);
        let available_at = if talent {
            None
        } else {
            self.spellbook_available_at(spell_id)
        };
        let presented_spell = if talent || available_at.is_some() {
            spell_id
        } else {
            self.effective_spell(spell_id)
        };
        let tooltip = self.spell_game_tooltip(presented_spell, available_at);
        let tooltip = if let Some((node, entry)) = talent_entry {
            self.spells
                .projected_talents()?
                .entry_tooltip(node, entry, tooltip)?
        } else {
            tooltip
        };
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    /// TrainerUI.xml OnEnter: the hovered service, ANCHOR_RIGHT +35, through GameTooltipUI.
    fn trainer_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, spell) = self
            .trainer
            .book
            .hovered_service(ui.registry()?, hit.frame)?;
        drop(ui);
        let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
            self.spell_game_tooltip(spell, None),
            self.professions.recipe(spell),
            self.player_level(),
        );
        let tooltip = match tooltip {
            Ok(tooltip) => tooltip,
            Err(error) => {
                eprintln!("Trainer service {spell}: {error}");
                return None;
            }
        };
        let tooltip = self.trainer_view().service_tooltip(
            spell,
            tooltip,
            self.owner_rect(&hit.ui, owner)?,
        )?;
        Some(HoveredTooltip::text(tooltip))
    }

    fn chat_link_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let spell_id = chat_spell_link_at(hit.ui.bind().registry()?, hit.frame)?;
        Some(HoveredTooltip::text(
            self.spell_game_tooltip(spell_id, None),
        ))
    }

    /// An item tooltip owned by `owner`, compared on Shift.
    fn item_owned(
        &self,
        hit: &HoveredFrame,
        owner: u64,
        side: OwnerSide,
        item: InventorySlot,
    ) -> Option<HoveredTooltip> {
        let mut tooltip = item_game_tooltip(&item, self.player_level());
        tooltip.content = self.mount_journal.view.item_tooltip(
            item.definition_source,
            item.item_id,
            tooltip.content,
        );
        Some(HoveredTooltip {
            tooltip: self.owned_by(hit, owner, side, tooltip)?,
            item: Some(item),
            health: None,
        })
    }

    /// `ContainerFrame{bag}Slot{slot}` of the standalone bags and the NPC-window backpack.
    fn bag_slot_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, (bag, slot)) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let (bag, slot) = name_of(frame)?
                .strip_prefix("ContainerFrame")?
                .split_once("Slot")?;
            Some((bag.parse::<usize>().ok()?, slot.parse::<usize>().ok()?))
        })?;
        drop(ui);
        let item = self.merchant.session.inventory.slot(bag, slot)?;
        if item.is_empty() {
            return None;
        }
        self.item_owned(hit, owner, OwnerSide::BagSlot, item.clone())
    }

    /// `PaperDollItemSlotButton_OnEnter` (PaperDollFrame.lua): `SetInventoryItem`, `ANCHOR_RIGHT`;
    /// an empty slot shows its slot name. None while the cursor holds an item.
    fn paperdoll_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, slot) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            parse_equipment_slot_action(frame.onclick.as_deref()?)
        })?;
        drop(ui);
        if !self.bags.cursor.item.is_empty() {
            return None;
        }
        let location = ItemLocation::Equipment(slot);
        if let Some(item) = self.merchant.session.inventory.item_at(location) {
            return self.item_owned(hit, owner, OwnerSide::Right, item.clone());
        }
        let tooltip = empty_paperdoll_slot_tooltip(paperdoll_button(slot)?.label);
        let tooltip = self.owned_by(hit, owner, OwnerSide::Right, tooltip)?;
        Some(HoveredTooltip::text(tooltip))
    }

    /// `MerchantItem{n}`: `SetMerchantItem` on the merchant tab, `SetBuybackItem` on buyback;
    /// the full item tooltip, compared on Shift (`GameTooltip_ShowCompareItem`, MF.lua:714).
    fn merchant_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let merchant = &self.merchant.session.merchant;
        if !merchant.is_open() {
            return None;
        }
        let ui = hit.ui.bind();
        let (owner, index) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            indexed(frame, "MerchantItem")?.checked_sub(1)
        })?;
        drop(ui);
        let item = merchant_cell_item(merchant, index)?;
        self.item_owned(hit, owner, OwnerSide::Right, item)
    }

    /// The service buttons' `OnEnter` (MF.xml:207-211, 240-252, 300-303, 332-364) and the last-sale
    /// slot's `MerchantBuyBackButton_OnEnter` (`SetBuybackItem`, MF.lua:1078-1082), all
    /// `ANCHOR_RIGHT`.
    fn merchant_button_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let session = &self.merchant.session;
        if !session.is_open() {
            return None;
        }
        let ui = hit.ui.bind();
        let (owner, button) =
            named_ancestor(ui.registry()?, hit.frame, |frame| match name_of(frame)? {
                name @ ("MerchantSellAllJunkButton"
                | "MerchantRepairAllButton"
                | "MerchantRepairItemButton"
                | "MerchantGuildBankRepairButton"
                | "MerchantBuyBackItem") => Some(name.to_owned()),
                _ => None,
            })?;
        drop(ui);
        let cost = u64::from(session.repair_cost);
        let tooltip = match button.as_str() {
            "MerchantSellAllJunkButton" => sell_all_junk_tooltip(),
            "MerchantRepairItemButton" => repair_item_tooltip(),
            "MerchantRepairAllButton" => repair_all_tooltip(cost, session.money)?,
            "MerchantGuildBankRepairButton" => {
                let guild_money = session.merchant.guild_repair_money?;
                guild_repair_tooltip(cost, guild_money, session.money)?
            }
            _ => {
                let last = buyback_item(session.merchant.last_buyback()?);
                return self.item_owned(hit, owner, OwnerSide::Right, last);
            }
        };
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
    }

    fn group_loot_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, (history, index, choice)) =
            named_ancestor(ui.registry()?, hit.frame, |frame| {
                let name = name_of(frame)?;
                if let Some(index) = name
                    .strip_prefix("LootHistoryElement")
                    .and_then(|index| index.parse::<usize>().ok())
                {
                    return Some((true, index.checked_sub(1)?, None));
                }
                for (suffix, choice) in [
                    ("NeedButton", Some(shared::loot::LootRollChoice::Need)),
                    ("GreedButton", Some(shared::loot::LootRollChoice::Greed)),
                    (
                        "TransmogButton",
                        Some(shared::loot::LootRollChoice::Transmog),
                    ),
                    ("PassButton", Some(shared::loot::LootRollChoice::Pass)),
                    ("IconFrame", None),
                ] {
                    if let Some(index) = name
                        .strip_prefix("GroupLootFrame")
                        .and_then(|name| name.strip_suffix(suffix))
                        .and_then(|index| index.parse::<usize>().ok())
                    {
                        return Some((false, index.checked_sub(1)?, choice));
                    }
                }
                None
            })?;
        drop(ui);
        let viewer = self.account.session.selected_character_id?;
        let roll = if history {
            self.loot.group.history().iter().rev().nth(index)?
        } else {
            self.loot.group.visible(viewer).get(index).copied()?
        };
        if let Some(choice) = choice {
            let player = roll
                .players
                .iter()
                .find(|player| player.character_id == viewer)?;
            let mut content = TooltipPresentation {
                title: game_engine_ui_model::group_loot::choice_name(choice).into(),
                ..Default::default()
            };
            if choice == shared::loot::LootRollChoice::Need
                && let Some(reason) = &player.need_reason
            {
                content.lines.push(TooltipLineState::colored(
                    reason.clone(),
                    game_engine_ui_model::item_tooltip::RED_FONT_COLOR,
                ));
            }
            let tooltip = GameTooltip::new(content, None);
            return Some(HoveredTooltip::text(self.owned_by(
                hit,
                owner,
                OwnerSide::Right,
                tooltip,
            )?));
        }
        let LootContent::Item {
            item_id,
            name,
            quality,
            count,
        } = &roll.item
        else {
            return None;
        };
        let item = named_item(*item_id, name, *quality, *count);
        let lines: Vec<_> = if history {
            roll.players
                .iter()
                .map(|player| match player.response {
                    Some(response) => format!(
                        "{}: {} ({})",
                        player.name,
                        game_engine_ui_model::group_loot::choice_name(response.choice),
                        response.roll_value
                    ),
                    None => format!("{}: Waiting", player.name),
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut tooltip = self.item_owned(hit, owner, OwnerSide::Right, item)?;
        tooltip.tooltip.content.lines.extend(
            lines
                .into_iter()
                .map(|line| TooltipLineState::colored(line, TOOLTIP_DESCRIPTION_COLOR)),
        );
        Some(tooltip)
    }

    /// `LootFrameElement{n}`: `SetLootItem` for item slots; money slots have no tooltip.
    fn loot_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, index) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            indexed(frame, "LootFrameElement")?.checked_sub(1)
        })?;
        drop(ui);
        let LootContent::Item {
            item_id,
            name,
            quality,
            count,
        } = &self.loot.state.slots.get(index)?.content
        else {
            return None;
        };
        let item = named_item(*item_id, name, *quality, *count);
        self.item_owned(hit, owner, OwnerSide::Right, item)
    }

    /// `QuestInfoItem{n}` of the quest frame and log: `QuestInfoRewardItemMixin:OnEnter`
    /// (QuestInfo.lua:1240-1258), `SetQuestItem`/`SetQuestLogItem` at `ANCHOR_RIGHT`.
    fn quest_reward_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, n) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            indexed(frame, "QuestInfoRewardsFrameQuestInfoItem")
        })?;
        drop(ui);
        let reward = self.quest_reward_item(&hit.ui, n)?;
        let quality = item_catalog_entry(reward.item_id).map_or(1, |entry| entry.quality);
        let item = named_item(reward.item_id, &reward.name, quality, reward.count);
        self.item_owned(hit, owner, OwnerSide::Right, item)
    }

    /// `OpenMailAttachmentButton{n}`: `SetInboxItem` and the C.O.D. money line.
    fn mail_attachment_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, slot) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            indexed(frame, "OpenMailAttachmentButton")?.checked_sub(1)
        })?;
        drop(ui);
        let session = &self.mailbox.session;
        let contents = session.contents.as_ref()?;
        let mail = contents
            .mails
            .iter()
            .find(|mail| Some(mail.mail_id) == session.selected)?;
        let attachment = mail
            .attachments
            .iter()
            .find(|attachment| usize::from(attachment.slot) == slot)?;
        let mut hovered =
            self.item_owned(hit, owner, OwnerSide::Right, attachment_item(attachment))?;
        if mail.cod > 0 {
            let lines = &mut hovered.tooltip.content.lines;
            lines.push(TooltipLineState::money(String::new(), mail.cod));
        }
        Some(hovered)
    }

    /// `MailItem{n}Button`, the inbox package (`InboxFrameItem_OnEnter`).
    fn inbox_item_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, mail_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            name.strip_prefix("MailItem")?.strip_suffix("Button")?;
            let action = frame.onclick.as_deref()?;
            action.strip_prefix(ACTION_OPEN_PREFIX)?.parse::<u64>().ok()
        })?;
        drop(ui);
        let contents = self.mailbox.session.contents.as_ref()?;
        let mail = contents.mails.iter().find(|mail| mail.mail_id == mail_id)?;
        let tooltip = inbox_tooltip(mail, self.player_level())?;
        let tooltip = self.owned_by(hit, owner, OwnerSide::Right, tooltip)?;
        Some(HoveredTooltip::text(tooltip))
    }

    /// Auction house rows: `GameTooltip:SetItemKey` / `SetHyperlink` without the vendor price.
    fn auction_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        if self.auction.session.ui.npc.is_none() {
            return None;
        }
        let ui = hit.ui.bind();
        let (owner, item) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            auction_row_item(frame.onclick.as_deref()?, &self.auction.session.net)
        })?;
        drop(ui);
        let mut tooltip = without_sell_price(item_game_tooltip(&item, self.player_level()));
        tooltip.content = self.mount_journal.view.item_tooltip(
            item.definition_source,
            item.item_id,
            tooltip.content,
        );
        Some(HoveredTooltip {
            tooltip: self.owned_by(hit, owner, OwnerSide::Right, tooltip)?,
            item: Some(item),
            health: None,
        })
    }

    /// `MainMenuBarBackpackButton` and `CharacterBag{i}Slot`.
    fn bag_bar_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, bag) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            if name == "MainMenuBarBackpackButton" {
                return Some(0);
            }
            let index: usize = name
                .strip_prefix("CharacterBag")?
                .strip_suffix("Slot")?
                .parse()
                .ok()?;
            Some(index + 1)
        })?;
        drop(ui);
        let tooltip = bag_bar_button_tooltip(&self.merchant.session.inventory, bag);
        let tooltip = self.owned_by(hit, owner, OwnerSide::Left, tooltip)?;
        Some(HoveredTooltip::text(tooltip))
    }

    /// `MainMenuBarMicroButtonMixin:EvaluateTooltipVisibility`: `ANCHOR_RIGHT`.
    fn micro_menu_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, index) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            micro_button_index(name_of(frame)?)
        })?;
        drop(ui);
        let key = MICRO_BUTTONS[index]
            .binding
            .and_then(|action| self.client_options.bindings.binding(action))
            .map(|binding| binding.display());
        let tooltip = micro_button_tooltip(&self.micro_menu_view(), index, key.as_deref())?;
        let tooltip = self.owned_by(hit, owner, OwnerSide::Right, tooltip)?;
        Some(HoveredTooltip::text(tooltip))
    }

    /// `BuffButton{n}` / `DebuffButton{n}` of the local player's BuffFrame.
    fn player_aura_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let (is_debuff, index) = buff_button_at(hit.ui.bind().registry()?, hit.frame)?;
        let local = self.world.local_player_id()?;
        let auras = self.unit_auras(local);
        let aura = auras
            .iter()
            .filter(|aura| aura.is_debuff == is_debuff)
            .nth(index)?;
        let tooltip = aura_tooltip(aura);
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            hit.frame,
            OwnerSide::BottomLeft,
            tooltip,
        )?))
    }

    /// `TargetBuffIcon{n}` / `TargetDebuffIcon{n}` in the TargetFrame's sort.
    fn target_aura_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, (debuff, index)) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            let name = name_of(frame)?;
            let (debuff, rest) = match name.strip_prefix("TargetBuffIcon") {
                Some(rest) => (false, rest),
                None => (true, name.strip_prefix("TargetDebuffIcon")?),
            };
            Some((debuff, rest.parse::<usize>().ok()?))
        })?;
        drop(ui);
        let target = self.targeting_target()?;
        let view = self.target_aura_view(target);
        let auras = self.unit_auras(target);
        let (buffs, debuffs) = target_frame_auras(&auras, view);
        let aura = if debuff { debuffs } else { buffs }.get(index).copied()?;
        let tooltip = aura_tooltip(aura);
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::ByCenter,
            tooltip,
        )?))
    }

    fn target_aura_view(&mut self, target: u64) -> TargetAuraView {
        let player_is_target = self.world.local_player_id() == Some(target);
        let reaction = self.reaction_to(target);
        let npc = self
            .replica
            .unit(target)
            .is_some_and(|unit| unit.has::<shared::components::Npc>());
        TargetAuraView {
            player_is_target,
            friendly: player_is_target || reaction == Reaction::Friendly,
            hostile_npc: npc && reaction == Reaction::Hostile,
        }
    }

    /// Unit frames and the minimap buttons.
    fn hud_frame_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        if let Some(unit) = self.unit_frame_unit(hit) {
            return self.unit_hovered_tooltip(unit);
        }
        let ui = hit.ui.bind();
        let (owner, name) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            Some(name_of(frame)?.to_owned())
        })?;
        drop(ui);
        match name.as_str() {
            MINIMAP_ZOOM_IN => Some(HoveredTooltip::text(zoom_tooltip(true))),
            MINIMAP_ZOOM_OUT => Some(HoveredTooltip::text(zoom_tooltip(false))),
            game_engine_ui_model::minimap::MINIMAP_ZONE_TEXT => {
                let tooltip = self.zone_text_tooltip()?;
                Some(HoveredTooltip::text(self.owned_by(
                    hit,
                    owner,
                    OwnerSide::Left,
                    tooltip,
                )?))
            }
            _ => None,
        }
    }
}

impl GameClient {
    /// The minimap header buttons drawn as plain art (`MiniMapMailFrame`,
    /// `MinimapClusterTrackingBackground`,
    /// `TimeManagerClockTicker`, `GameTimeFrame`), hit by their rect.
    pub(crate) fn minimap_button_tooltip(&self) -> Option<HoveredTooltip> {
        let ui = self.minimap.ui.as_ref()?;
        let [px, py] = self.physical_input.pointer();
        let scale = self.effective_ui_scale();
        let host = ui.bind();
        let hit = |name: &str| {
            let ([x, y, w, h], _) = host.frame_rect(name)?;
            (px >= x && px <= x + w && py >= y && py <= y + h)
                .then(|| [x / scale, y / scale, w / scale, h / scale])
        };
        let (hour, minute, _) = crate::minimap::local_time();
        let time = twelve_hour_time(hour, minute);
        if let Some(tooltip) = minimap_mouseover_tooltip(&self.minimap_blip_names(&host, &hit)) {
            return Some(HoveredTooltip::text(tooltip));
        }
        let senders = &self.mailbox.session.pending_senders;
        let mail = (!senders.is_empty())
            .then(|| hit(game_engine_ui_model::minimap::MINIMAP_MAIL_FRAME))
            .flatten();
        let (tooltip, rect, side) = if let Some(rect) = mail {
            (unread_mail_tooltip(senders), rect, OwnerSide::BottomLeft)
        } else if let Some(rect) = hit("MinimapClusterTrackingBackground") {
            (tracking_tooltip(), rect, OwnerSide::Left)
        } else if let Some(rect) = hit(game_engine_ui_model::minimap::MINIMAP_CLOCK_TEXT) {
            // No realm clock is replicated: the realm time reads the local time.
            (clock_tooltip(&time, &time), rect, OwnerSide::Left)
        } else {
            (
                calendar_tooltip(),
                hit("GameTimeFrame")?,
                OwnerSide::BottomLeft,
            )
        };
        Some(HoveredTooltip::text(tooltip.owned(rect, side)))
    }
}

/// The backpack's free slots over every bag, an equipped bag, or `EQUIP_CONTAINER`.
fn bag_bar_button_tooltip(inventory: &InventoryState, bag: usize) -> GameTooltip {
    if bag == 0 {
        return backpack_tooltip(inventory.total_free_slots());
    }
    match inventory.bags.iter().find(|info| info.index == bag) {
        Some(info) => bag_item_tooltip(&info.name, info.size),
        None => empty_bag_slot_tooltip(),
    }
}

/// A mail attachment as the bags would hold it.
fn attachment_item(attachment: &MailAttachment) -> InventorySlot {
    let stack = &attachment.item;
    InventorySlot {
        durability: stack.durability,
        soulbound: stack.soulbound,
        item_guid: stack.item_guid,
        ..named_item_for(
            stack.definition_source,
            stack.item_id,
            &attachment.name,
            attachment.quality,
            stack.count,
            stack.item_guid,
        )
    }
}

/// The single attachment's tooltip or `MAIL_MULTIPLE_ITEMS (n)`, then the enclosed money
/// or the C.O.D. amount; nothing for a mail with neither.
fn inbox_tooltip(mail: &MailHeader, player_level: Option<u16>) -> Option<GameTooltip> {
    let mut tooltip = match mail.attachments.as_slice() {
        [] => GameTooltip::new(TooltipPresentation::hidden(), None),
        [single] => item_game_tooltip(&attachment_item(single), player_level),
        many => GameTooltip::new(
            TooltipPresentation {
                title: format!("{MAIL_MULTIPLE_ITEMS} ({})", many.len()),
                title_color: TOOLTIP_DESCRIPTION_COLOR,
                ..TooltipPresentation::hidden()
            },
            None,
        ),
    };
    match (mail.money, mail.cod) {
        (0, 0) if mail.attachments.is_empty() => return None,
        (0, 0) => {}
        (0, cod) => add_money_section(&mut tooltip.content, COD_AMOUNT, cod),
        (money, _) => add_money_section(&mut tooltip.content, ENCLOSED_MONEY, money),
    }
    Some(tooltip)
}

impl GameClient {
    /// Names of the units whose minimap blips (`MinimapBlip{unit}`) `hit` covers.
    fn minimap_blip_names(
        &self,
        host: &crate::ui::RegistryUi,
        hit: &dyn Fn(&str) -> Option<[f32; 4]>,
    ) -> Vec<String> {
        let Some(registry) = host.registry() else {
            return Vec::new();
        };
        registry
            .frames_iter()
            .filter_map(|frame| {
                let name = frame.name.as_deref()?;
                let unit: u64 = name.strip_prefix("MinimapBlip")?.parse().ok()?;
                hit(name)?;
                Some(self.replica.unit(unit)?.name()?.to_owned())
            })
            .collect()
    }
}

/// `SetInventoryItem` for an equipped bag from what the server names: the bag and
/// `CONTAINER_SLOTS` "%d Slot %s".
fn bag_item_tooltip(name: &str, size: usize) -> GameTooltip {
    GameTooltip::new(
        TooltipPresentation {
            title: name.to_owned(),
            title_color: game_engine_ui_model::tooltip_presentation::TOOLTIP_WHITE,
            lines: vec![TooltipLineState::colored(
                format!("{size} Slot Bag"),
                game_engine_ui_model::tooltip_presentation::TOOLTIP_WHITE,
            )],
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

/// A blank line after other content, the wrapped gold label and `GameTooltip_AddMoneyLine`.
fn add_money_section(tooltip: &mut TooltipPresentation, label: &str, copper: u64) {
    if !tooltip.title.is_empty() {
        tooltip.lines.push(TooltipLineState::new(String::new()));
    }
    if tooltip.title.is_empty() {
        tooltip.title = label.to_owned();
        tooltip.title_color = TOOLTIP_DESCRIPTION_COLOR;
    } else {
        tooltip
            .lines
            .push(TooltipLineState::colored(label, TOOLTIP_DESCRIPTION_COLOR));
    }
    tooltip
        .lines
        .push(TooltipLineState::money(String::new(), copper));
}
