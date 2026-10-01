//! Item, quest and presence status in the original response text (src/ipc/format.rs,
//! src/ipc/plugin.rs `dispatch_inventory_request`, src/friends.rs
//! `format_presence_status`). The original listed bags from the last auction-house
//! inventory query; the native client lists the live bags. The guild vault and Warband
//! bank list the last contents the server sent, with no item names, as the original.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::item_catalog::item_catalog_entry;
use shared::{
    components::PresenceStatus,
    protocol::{ItemStack, QuestEntrySnapshot, QuestRepeatability},
};

/// One stored item as the original `InventoryItemEntry`.
struct StoredItem {
    storage: &'static str,
    slot: u32,
    item_guid: u64,
    item_id: u32,
    name: String,
    stack_count: u32,
}

impl crate::GameClient {
    /// An item, quest or presence request's response, or the request when the client
    /// does not serve it.
    pub(crate) fn item_request(&mut self, request: Request) -> Result<Response, Request> {
        let answer = match request {
            Request::QuestList => Ok(format_quest_list(&self.account.quests.log)),
            Request::QuestWatch => Ok(format_quest_watch(
                &self.account.quests.log,
                &self.account.quests.watched,
            )),
            Request::QuestShow { quest_id } => {
                Ok(format_quest_show(&self.account.quests.log, quest_id))
            }
            Request::BagsStatus => self.bags_status(),
            Request::InventoryList => Ok(format_inventory_list(&self.stored_items())),
            Request::InventorySearch { text } => {
                Ok(format_inventory_search(&self.stored_items(), &text))
            }
            Request::InventoryWhereis { item_id } => {
                Ok(format_inventory_whereis(&self.stored_items(), item_id))
            }
            Request::GuildVaultStatus => Ok(format_storage_list(
                "guild_vault",
                &self.guild_vault_items(),
            )),
            Request::WarbankStatus => Ok(format_storage_list("warbank", &self.warbank_items())),
            Request::ItemInfo { query } => self.item_info(query.item_id),
            Request::PresenceStatus => Ok(self.presence_status()),
            request => return Err(request),
        };
        Ok(match answer {
            Ok(text) => Response::Text(text),
            Err(error) => Response::Error(error),
        })
    }

    /// Occupied bag slots in bag order with the item's required level, and the
    /// player's money; unavailable outside the world.
    fn bags_status(&self) -> Result<String, String> {
        if self.world.local_player_id().is_none() {
            return Ok("bags: unavailable\n-".into());
        }
        let gold = self.merchant.session.money;
        let items = self.bag_items();
        if items.is_empty() {
            return Ok(format!("bags: 0\ngold: {gold}\n-"));
        }
        let lines = items
            .iter()
            .enumerate()
            .map(|(slot, item)| {
                let required_level = item_catalog_entry(item.item_id)
                    .map(|entry| entry.required_level)
                    .ok_or_else(|| format!("item {} is not in the item catalog", item.item_id))?;
                Ok(format!(
                    "{slot} {} {} x{} q{} lvl{required_level}",
                    item.item_guid, item.name, item.count, item.quality as u8
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(format!(
            "bags: {}\ngold: {gold}\n{}",
            items.len(),
            lines.join("\n")
        ))
    }

    fn bag_items(&self) -> Vec<&game_engine_ui_model::bag_data::InventorySlot> {
        self.merchant
            .session
            .inventory
            .slots
            .iter()
            .flatten()
            .filter(|slot| !slot.is_empty())
            .collect()
    }

    /// `build_inventory_entries`: bags, then the guild vault and the Warband bank.
    fn stored_items(&self) -> Vec<StoredItem> {
        let mut items: Vec<StoredItem> = self
            .bag_items()
            .into_iter()
            .enumerate()
            .map(|(slot, item)| StoredItem {
                storage: "bags",
                slot: slot as u32,
                item_guid: item.item_guid,
                item_id: item.item_id,
                name: item.name.clone(),
                stack_count: item.count,
            })
            .collect();
        items.extend(self.guild_vault_items());
        items.extend(self.warbank_items());
        items
    }

    fn guild_vault_items(&self) -> Vec<StoredItem> {
        self.banks
            .guild_vault_seen
            .as_ref()
            .map_or_else(Vec::new, |contents| {
                storage_items(
                    "guild_vault",
                    contents.tabs.iter().map(|tab| tab.slots.as_slice()),
                )
            })
    }

    fn warbank_items(&self) -> Vec<StoredItem> {
        self.banks
            .warbank_seen
            .as_ref()
            .map_or_else(Vec::new, |contents| {
                storage_items(
                    "warbank",
                    contents.tabs.iter().map(|tab| tab.slots.as_slice()),
                )
            })
    }

    /// The item's DB2 entry (`ItemSparse`); the appearance is known when the item is in
    /// the bags or equipped. The original read a generated item table whose multi-line
    /// entries its line parser never matched.
    fn item_info(&self, item_id: u32) -> Result<String, String> {
        let item =
            item_catalog_entry(item_id).ok_or_else(|| format!("item {item_id} not found"))?;
        let inventory = &self.merchant.session.inventory;
        let appearance_known = self.bag_items().iter().any(|slot| slot.item_id == item_id)
            || inventory
                .equipment
                .values()
                .any(|slot| slot.item_id == item_id);
        Ok(format!(
            "item_id: {item_id}\nname: {}\nquality: {}\nitem_level: {}\nrequired_level: {}\ninventory_type: {}\nsell_price: {}\nstackable: {}\nbonding: {}\nexpansion_id: {}\nappearance_known: {appearance_known}",
            item.name,
            item.quality,
            item.item_level,
            item.required_level,
            item.inventory_type,
            item.sell_price,
            item.stackable,
            item.bonding,
            item.expansion_id,
        ))
    }

    /// The local player's replicated presence.
    fn presence_status(&self) -> String {
        let presence = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<PresenceStatus>().copied());
        let presence = match presence {
            Some(PresenceStatus::Online) => "online",
            Some(PresenceStatus::Afk) => "afk",
            Some(PresenceStatus::Dnd) => "dnd",
            Some(PresenceStatus::Offline) => "offline",
            None => "-",
        };
        format!("presence: {presence}")
    }
}

/// The original `storage_entries`: tab-major slot numbers, no item names.
fn storage_items<'a>(
    storage: &'static str,
    tabs: impl Iterator<Item = &'a [Option<ItemStack>]>,
) -> Vec<StoredItem> {
    tabs.enumerate()
        .flat_map(|(tab, slots)| {
            slots.iter().enumerate().filter_map(move |(slot, item)| {
                let item = item.as_ref()?;
                Some(StoredItem {
                    storage,
                    slot: (tab * slots.len() + slot) as u32,
                    item_guid: item.item_guid,
                    item_id: item.item_id,
                    name: String::new(),
                    stack_count: item.count,
                })
            })
        })
        .collect()
}

fn format_storage_list(title: &str, items: &[StoredItem]) -> String {
    if items.is_empty() {
        return format!("{title}: 0\n-");
    }
    let lines = items
        .iter()
        .map(|e| {
            format!(
                "{} {} {} {} x{}",
                e.slot, e.item_guid, e.item_id, e.name, e.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{title}: {}\n{lines}", items.len())
}

fn format_inventory_list(items: &[StoredItem]) -> String {
    if items.is_empty() {
        return "inventory: 0\n-".into();
    }
    let lines = items
        .iter()
        .map(|e| {
            format!(
                "{}:{} {} {} {} x{}",
                e.storage, e.slot, e.item_guid, e.item_id, e.name, e.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("inventory: {}\n{lines}", items.len())
}

/// `inventory_search_snapshot` and `format_inventory_search`: a case-insensitive name
/// match, grouped by storage.
fn format_inventory_search(items: &[StoredItem], text: &str) -> String {
    let needle = text.trim().to_ascii_lowercase();
    let found: Vec<&StoredItem> = items
        .iter()
        .filter(|e| needle.is_empty() || e.name.to_ascii_lowercase().contains(&needle))
        .collect();
    if found.is_empty() {
        return format!("inventory search text={text}: 0\n-");
    }
    let mut lines = Vec::new();
    for storage in ["bags", "guild_vault", "warbank"] {
        let grouped: Vec<_> = found.iter().filter(|e| e.storage == storage).collect();
        if grouped.is_empty() {
            continue;
        }
        lines.push(format!("[{storage}]"));
        lines.extend(grouped.iter().map(|e| {
            format!(
                "{} {} {} {} x{}",
                e.slot, e.item_guid, e.item_id, e.name, e.stack_count
            )
        }));
    }
    format!(
        "inventory search text={text}: {}\n{}",
        found.len(),
        lines.join("\n")
    )
}

fn format_inventory_whereis(items: &[StoredItem], item_id: u32) -> String {
    let matches: Vec<_> = items.iter().filter(|e| e.item_id == item_id).collect();
    if matches.is_empty() {
        return format!("inventory whereis item_id={item_id}: 0\n-");
    }
    let lines = matches
        .iter()
        .map(|e| {
            format!(
                "{}:{} {} {} x{}",
                e.storage, e.slot, e.item_guid, e.name, e.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "inventory whereis item_id={item_id}: {}\n{lines}",
        matches.len()
    )
}

fn repeatability_label(value: &QuestRepeatability) -> &'static str {
    match value {
        QuestRepeatability::Normal => "normal",
        QuestRepeatability::Daily => "daily",
        QuestRepeatability::Weekly => "weekly",
    }
}

fn format_quest_list(log: &[QuestEntrySnapshot]) -> String {
    if log.is_empty() {
        return "quests: 0\n-".into();
    }
    let lines = log
        .iter()
        .map(|e| {
            format!(
                "{} {} zone={} repeat={} completed={} objectives={}",
                e.quest_id,
                e.title,
                e.zone,
                repeatability_label(&e.repeatability),
                e.completed,
                e.objectives.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("quests: {}\n{lines}", log.len())
}

fn format_quest_watch(log: &[QuestEntrySnapshot], watched: &[u32]) -> String {
    let watched: Vec<_> = watched
        .iter()
        .filter_map(|id| log.iter().find(|e| e.quest_id == *id))
        .collect();
    if watched.is_empty() {
        return "quest_watch: 0\n-".into();
    }
    let lines = watched
        .iter()
        .map(|e| {
            let objectives = e
                .objectives
                .iter()
                .map(|o| format!("{} {}/{}", o.text, o.current, o.required))
                .collect::<Vec<_>>()
                .join("; ");
            format!("{} {} [{objectives}]", e.quest_id, e.title)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("quest_watch: {}\n{lines}", watched.len())
}

fn format_quest_show(log: &[QuestEntrySnapshot], quest_id: u32) -> String {
    let Some(entry) = log.iter().find(|e| e.quest_id == quest_id) else {
        return format!("quest {quest_id}: not found");
    };
    let objectives = if entry.objectives.is_empty() {
        "-".into()
    } else {
        entry
            .objectives
            .iter()
            .map(|o| {
                format!(
                    "{} {}/{} completed={}",
                    o.text, o.current, o.required, o.completed
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "quest_id: {}\ntitle: {}\nzone: {}\nrepeatability: {}\ncompleted: {}\nobjectives:\n{}",
        entry.quest_id,
        entry.title,
        entry.zone,
        repeatability_label(&entry.repeatability),
        entry.completed,
        objectives,
    )
}
