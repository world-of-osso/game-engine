//! Retail Toy Box (`Blizzard_ToyBox.lua`): catalog filters, 18/page, favourites,
//! learning navigation and cooldowns. Retail pickup uses an item action; ToyAction
//! resolves that persisted ItemID against the authoritative catalog, never bag contents.
use shared::protocol::{ActionRef, SpellCooldownUpdate, ToySnapshot, UseToy};
use std::collections::{BTreeSet, HashMap};

pub const PAGE_SIZE: usize = 18;
pub const SEARCH_FIELD: &str = "ToyBoxSearchBox";
pub const USE_PREFIX: &str = "toy_use:";
pub const FAVOURITE_PREFIX: &str = "toy_favourite:";
pub const CLOSE: &str = "toybox_close";

#[derive(Clone, Debug, PartialEq)]
pub struct ToyFilters {
    pub collected: bool,
    pub uncollected: bool,
    pub usable_only: bool,
    pub sources: Option<BTreeSet<i32>>,
    pub expansions: Option<BTreeSet<i32>>,
    pub search: String,
}
impl Default for ToyFilters {
    fn default() -> Self {
        Self {
            collected: true,
            uncollected: true,
            usable_only: false,
            sources: None,
            expansions: None,
            search: String::new(),
        }
    }
}
impl ToyFilters {
    fn matches(&self, toy: &ToySnapshot) -> bool {
        let ownership = if toy.learned {
            self.collected
        } else {
            self.uncollected
        };
        let usable = !self.usable_only || can_use(toy);
        let source = self
            .sources
            .as_ref()
            .is_none_or(|set| set.contains(&toy.source_type));
        let expansion = self
            .expansions
            .as_ref()
            .is_none_or(|set| set.contains(&toy.expansion_id));
        let search = toy
            .name
            .to_lowercase()
            .contains(&self.search.to_lowercase());
        ownership && usable && source && expansion && search
    }
}
pub fn can_use(toy: &ToySnapshot) -> bool {
    toy.learned && toy.spell_id.is_some() && toy.unavailable_reason.is_none()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToyCooldowns {
    timers: HashMap<u32, (f32, f32)>,
}
impl ToyCooldowns {
    pub fn apply(&mut self, update: &SpellCooldownUpdate) {
        if update.is_gcd {
            return;
        }
        if update.remaining_ms == 0 {
            self.timers.remove(&update.spell_id);
        } else {
            self.timers.insert(
                update.spell_id,
                (
                    update.duration_ms as f32 / 1000.0,
                    update.remaining_ms as f32 / 1000.0,
                ),
            );
        }
    }
    pub fn tick(&mut self, delta: f32) {
        self.timers.retain(|_, (_, remaining)| {
            *remaining = (*remaining - delta).max(0.0);
            *remaining > 0.0
        });
    }
    pub fn fraction(&self, spell_id: u32) -> f32 {
        self.timers
            .get(&spell_id)
            .filter(|(duration, _)| *duration > 0.0)
            .map_or(0.0, |(duration, remaining)| {
                (remaining / duration).clamp(0.0, 1.0)
            })
    }
    pub fn remaining(&self, spell_id: u32) -> f32 {
        self.timers
            .get(&spell_id)
            .map_or(0.0, |(_, remaining)| *remaining)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToyBox {
    pub catalog: Vec<ToySnapshot>,
    pub filters: ToyFilters,
    pub page: usize,
    pub new_toys: BTreeSet<u32>,
    pub cooldowns: ToyCooldowns,
    pub open: bool,
    pub filters_open: bool,
    pub filter_submenu: Option<String>,
    pub context: Option<u32>,
    pub hovered: Option<u32>,
    pub error: Option<String>,
    received: bool,
}
impl ToyBox {
    pub fn receive(&mut self, catalog: Vec<ToySnapshot>) {
        let learned: Vec<_> = catalog
            .iter()
            .filter(|toy| {
                toy.learned
                    && self.received
                    && self.toy(toy.item_id).is_some_and(|old| !old.learned)
            })
            .map(|toy| toy.item_id)
            .collect();
        self.catalog = catalog;
        self.received = true;
        self.new_toys.extend(&learned);
        if let Some(id) = learned.last() {
            if let Some(index) = self.filtered().iter().position(|toy| toy.item_id == *id) {
                self.page = index / PAGE_SIZE;
            }
        }
        self.page = self.page.min(self.page_count() - 1);
    }
    pub fn toy(&self, item_id: u32) -> Option<&ToySnapshot> {
        self.catalog.iter().find(|toy| toy.item_id == item_id)
    }
    pub fn filtered(&self) -> Vec<&ToySnapshot> {
        let mut toys: Vec<_> = self
            .catalog
            .iter()
            .filter(|toy| self.filters.matches(toy))
            .collect();
        toys.sort_by_cached_key(|toy| (!toy.favourite, toy.name.to_lowercase(), toy.item_id));
        toys
    }
    pub fn page_count(&self) -> usize {
        self.filtered().len().div_ceil(PAGE_SIZE).max(1)
    }
    pub fn page_items(&self) -> Vec<&ToySnapshot> {
        self.filtered()
            .into_iter()
            .skip(self.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect()
    }
    pub fn turn_page(&mut self, delta: i32) {
        self.page = (self.page as i32 + delta).clamp(0, self.page_count() as i32 - 1) as usize;
        self.context = None;
    }
    pub fn acknowledge(&mut self, item_id: u32) {
        self.new_toys.remove(&item_id);
    }
    pub fn pickup(&self, item_id: u32) -> Option<ToyAction> {
        self.toy(item_id)
            .filter(|toy| toy.learned)
            .map(|_| ToyAction { item_id })
    }
    pub fn tooltip(&self, item_id: u32) -> Option<String> {
        let toy = self.toy(item_id)?;
        let owned = if toy.learned {
            "Collected"
        } else {
            "Not collected"
        };
        let availability = toy.unavailable_reason.as_deref().unwrap_or(if toy.learned {
            "Use: Activate this toy"
        } else {
            "Learn this toy from its item"
        });
        Some(format!(
            "{}\n{}\n{}\n{}",
            toy.name, owned, toy.source_text, availability
        ))
    }
    pub fn apply_filter_action(&mut self, action: &str) -> bool {
        match action {
            "toy_filter:collected" => self.filters.collected = !self.filters.collected,
            "toy_filter:uncollected" => self.filters.uncollected = !self.filters.uncollected,
            "toy_filter:usable" => self.filters.usable_only = !self.filters.usable_only,
            "toy_filter:reset" => self.filters = ToyFilters::default(),
            "toy_filter:sources" => self.filter_submenu = Some("sources".into()),
            "toy_filter:expansions" => self.filter_submenu = Some("expansions".into()),
            _ => return self.apply_type_filter(action),
        }
        self.page = 0;
        true
    }
    fn apply_type_filter(&mut self, action: &str) -> bool {
        let (selected, all, value) = if let Some(raw) = action.strip_prefix("toy_source:") {
            (
                &mut self.filters.sources,
                self.catalog.iter().map(|toy| toy.source_type).collect(),
                raw,
            )
        } else if let Some(raw) = action.strip_prefix("toy_expansion:") {
            (
                &mut self.filters.expansions,
                self.catalog.iter().map(|toy| toy.expansion_id).collect(),
                raw,
            )
        } else {
            return false;
        };
        match value {
            "all" => *selected = None,
            "none" => *selected = Some(BTreeSet::new()),
            value => {
                let Ok(id) = value.parse::<i32>() else {
                    return false;
                };
                let set = selected.get_or_insert(all);
                if !set.remove(&id) {
                    set.insert(id);
                }
            }
        }
        self.page = 0;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToyAction {
    pub item_id: u32,
}
impl ToyAction {
    pub fn from_slot(slot: ActionRef, model: &ToyBox) -> Option<Self> {
        let ActionRef::Item(item_id) = slot else {
            return None;
        };
        model.toy(item_id).map(|_| Self { item_id })
    }
    pub fn to_slot(self) -> ActionRef {
        ActionRef::Item(self.item_id)
    }
    pub fn use_request(self) -> UseToy {
        UseToy {
            item_id: self.item_id,
        }
    }
}
