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
    auction_row_item, item_game_tooltip, named_item, without_sell_price,
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

impl GameClient {
    /// The tooltip of the hovered frame, if it has one.
    pub(crate) fn frame_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        type Source = fn(&mut GameClient, &HoveredFrame) -> Option<HoveredTooltip>;
        const SOURCES: [Source; 17] = [
            GameClient::action_button_tooltip,
            GameClient::spellbook_tooltip,
            GameClient::chat_link_tooltip,
            GameClient::bag_slot_tooltip,
            GameClient::paperdoll_tooltip,
            GameClient::merchant_tooltip,
            GameClient::merchant_button_tooltip,
            GameClient::loot_tooltip,
            GameClient::mail_attachment_tooltip,
            GameClient::inbox_item_tooltip,
            GameClient::auction_tooltip,
            GameClient::quest_reward_tooltip,
            GameClient::bag_bar_tooltip,
            GameClient::micro_menu_tooltip,
            GameClient::player_aura_tooltip,
            GameClient::target_aura_tooltip,
            GameClient::hud_frame_tooltip,
        ];
        SOURCES.iter().find_map(|source| source(self, hit))
    }

    fn spell_game_tooltip(&self, spell_id: u32, available_at: Option<u32>) -> GameTooltip {
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
            cooldown_remaining: self
                .account
                .spells
                .button_cooldown(spell_id, false)
                .map(|timer| timer.remaining),
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

    fn action_button_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (_, slot) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            parse_action_button(frame.onclick.as_deref()?)
        })?;
        let ActionRef::Spell(spell_id) = self.account.spells.slot(slot)? else {
            return None;
        };
        Some(HoveredTooltip::text(
            self.spell_game_tooltip(spell_id, None),
        ))
    }

    fn spellbook_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        let ui = hit.ui.bind();
        let (owner, spell_id) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            name_of(frame)?
                .strip_prefix("SpellBookItem")?
                .strip_suffix("Button")?
                .parse::<u32>()
                .ok()
        })?;
        drop(ui);
        let tooltip = self.spell_game_tooltip(spell_id, self.spellbook_available_at(spell_id));
        Some(HoveredTooltip::text(self.owned_by(
            hit,
            owner,
            OwnerSide::Right,
            tooltip,
        )?))
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
        let tooltip = item_game_tooltip(&item, self.player_level());
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
        let tooltip = without_sell_price(item_game_tooltip(&item, self.player_level()));
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
        ..named_item(
            stack.item_id,
            &attachment.name,
            attachment.quality,
            stack.count,
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
