//! Retail Mainline Adventure Guide; both skins share the same 800×496 layout.
use crate::bank_art::{WHITE, label};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, portrait_border};
use game_engine_core::adventure_guide::{
    Catalog, Instance, Item, ItemSet, JourneyPresentation, Section, difficulty_matches,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "EncounterJournalSearchBox";
pub const LIST: &str = "EncounterJournalContent";
pub const SCROLL_LISTS: [&str; 6] = [
    LIST,
    "EncounterJournalBosses",
    "EncounterJournalLore",
    "EncounterJournalSearchList",
    "EncounterJournalFilterList",
    "EncounterJournalDifficultyList",
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    Journeys,
    #[default]
    Dungeons,
    Raids,
    ItemSets,
    Tutorials,
    Monthly,
    Suggested,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Overview,
    Abilities,
    Loot,
    Model,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LootFilter {
    Class,
    Spec,
    Slot,
}
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub visible: bool,
    pub loading: bool,
    pub viewport: [f32; 2],
    pub catalog: Arc<Catalog>,
    pub tab: Tab,
    pub page: Page,
    pub tier: u32,
    pub tier_open: bool,
    pub difficulty_open: bool,
    pub nav_overflow_open: bool,
    pub class_menu_open: bool,
    pub instance: Option<u32>,
    pub encounter: Option<u32>,
    pub creature: Option<u32>,
    pub difficulty: u32,
    pub role: u32,
    pub class: u32,
    pub spec: u32,
    pub player_spec: Option<u32>,
    pub slot: u32,
    pub filter_menu: Option<LootFilter>,
    pub search: String,
    pub collapsed: BTreeSet<u32>,
    pub lockouts: Vec<(u32, String)>,
    pub saved_instances: Vec<shared::protocol::InstanceLockInfo>,
    pub challenge: Option<shared::protocol::ChallengeRun>,
    pub keys: Vec<shared::protocol::KeystoneInfo>,
    pub owned_keys_only: bool,
    pub activities: Option<Arc<shared::protocol::AdventureGuideSnapshot>>,
    pub trading_post: Option<Arc<shared::protocol::TradingPostSnapshot>>,
    pub activity_filters: crate::adventure_activities::ActivityFilters,
    pub renown: Option<Arc<shared::protocol::RenownSnapshot>>,
    pub journey_presentation: Arc<BTreeMap<u32, JourneyPresentation>>,
    pub journeys: crate::adventure_journeys::Journeys,
}
impl Default for View {
    fn default() -> Self {
        Self {
            visible: false,
            loading: false,
            viewport: [1280., 720.],
            catalog: Arc::new(Catalog::default()),
            tab: Tab::Dungeons,
            page: Page::Overview,
            tier: 73,
            tier_open: false,
            difficulty_open: false,
            nav_overflow_open: false,
            class_menu_open: false,
            instance: None,
            encounter: None,
            creature: None,
            difficulty: 2,
            role: 0,
            class: 0,
            spec: 0,
            player_spec: None,
            slot: 0,
            filter_menu: None,
            search: String::new(),
            collapsed: BTreeSet::new(),
            lockouts: Vec::new(),
            saved_instances: Vec::new(),
            challenge: None,
            keys: Vec::new(),
            owned_keys_only: false,
            activities: None,
            trading_post: None,
            activity_filters: Default::default(),
            renown: None,
            journey_presentation: Arc::default(),
            journeys: Default::default(),
        }
    }
}
impl View {
    pub fn challenge_text(&self) -> Option<String> {
        let instance = self.catalog.instance(self.instance?)?;
        let run = self
            .challenge
            .as_ref()
            .filter(|run| run.map_id == instance.map)?;
        let block = crate::challenge::tracker_block(run)?;
        Some(
            std::iter::once(block.name)
                .chain(block.bosses.into_iter().map(|line| line.text))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
    pub fn set_member(&self, frame_name: &str) -> Option<(&ItemSet, &Item)> {
        let (set, item) = frame_name.strip_prefix("JournalSet")?.split_once("Item")?;
        let set: u32 = set.parse().ok()?;
        let item: u32 = item.parse().ok()?;
        let set = self
            .catalog
            .sets
            .iter()
            .find(|row| row.id == set && row.items.contains(&item))?;
        Some((set, self.catalog.items.get(&item)?))
    }
    pub fn item_sets(&self) -> Vec<&ItemSet> {
        let search = self.search.to_lowercase();
        let mut sets: Vec<_> = self
            .catalog
            .sets
            .iter()
            .filter(|set| {
                let items: Vec<_> = set
                    .items
                    .iter()
                    .filter_map(|id| self.catalog.items.get(id))
                    .collect();
                set.name.to_lowercase().contains(&search)
                    && !items.is_empty()
                    && items.iter().all(|item| {
                        let class_matches = self.class == 0
                            || item.class_mask == -1
                            || item.class_mask & (1 << (self.class - 1)) != 0;
                        class_matches && self.matches_specialization(item.id)
                    })
            })
            .collect();
        sets.sort_by(|a, b| {
            self.set_level(b)
                .cmp(&self.set_level(a))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| b.id.cmp(&a.id))
        });
        sets
    }
    pub(crate) fn set_level(&self, set: &ItemSet) -> Option<u32> {
        set.items
            .iter()
            .filter_map(|id| crate::item_catalog::item_catalog_entry(*id))
            .map(|item| {
                u32::from(crate::item_stats::item_level_for(
                    shared::item_data::ItemDefinitionSource::Retail,
                    item,
                ))
            })
            .max()
    }
    fn choose_set_specialization(&self) -> u32 {
        if self.class == 0 {
            return 0;
        }
        let specs: Vec<_> = self
            .catalog
            .specs
            .iter()
            .filter(|spec| spec.class == self.class && spec.name != "Initial")
            .collect();
        specs
            .iter()
            .find(|spec| Some(spec.id) == self.player_spec)
            .or_else(|| specs.first())
            .map_or(0, |spec| spec.id)
    }
    pub fn owned_keys(
        &self,
        map: u32,
    ) -> Vec<(
        &shared::protocol::KeystoneInfo,
        &game_engine_core::adventure_guide::ChallengeDungeon,
    )> {
        self.keys
            .iter()
            .filter_map(|key| {
                let dungeon = self.catalog.challenge_dungeons.get(&key.challenge_id)?;
                (dungeon.map == map).then_some((key, dungeon))
            })
            .collect()
    }
    pub fn owned_key_text(&self) -> Option<String> {
        let instance = self.catalog.instance(self.instance?)?;
        let rows: Vec<_> = self
            .owned_keys(instance.map)
            .iter()
            .map(|(key, dungeon)| {
                format!(
                    "Owned Keystone: {} +{} · Time Limit: {}",
                    dungeon.name,
                    key.level,
                    crate::challenge::clock(dungeon.time_limit_ms)
                )
            })
            .collect();
        (!rows.is_empty()).then(|| rows.join("\n"))
    }
    pub fn instances(&self) -> Vec<&Instance> {
        let search = self.search.to_lowercase();
        self.catalog
            .instances
            .iter()
            .filter(|i| {
                i.dungeon == (self.tab == Tab::Dungeons)
                    && if self.owned_keys_only && self.tab == Tab::Dungeons {
                        !self.owned_keys(i.map).is_empty()
                    } else {
                        i.tiers.contains(&self.tier)
                    }
                    && i.name.to_lowercase().contains(&search)
            })
            .collect()
    }
    pub(crate) fn encounters(
        &self,
    ) -> impl Iterator<Item = &game_engine_core::adventure_guide::Encounter> {
        self.catalog.encounters.iter().filter(|encounter| {
            Some(encounter.instance) == self.instance
                && difficulty_matches(
                    &self.catalog.encounter_difficulties,
                    encounter.id,
                    self.difficulty,
                )
        })
    }
    pub(crate) fn instance_has_overview(&self) -> bool {
        self.encounters().any(|encounter| {
            self.catalog
                .sections
                .get(&encounter.root)
                .is_some_and(|section| section.kind == 3)
        })
    }
    fn shows_overview(&self) -> bool {
        self.page == Page::Overview && self.instance_has_overview()
    }
    pub(crate) fn page_enabled(&self, page: Page) -> bool {
        match page {
            Page::Overview => {
                self.instance.is_some()
                    && self.encounter.is_none_or(|id| {
                        self.catalog
                            .encounter(id)
                            .is_some_and(|encounter| encounter.root != 0)
                    })
            }
            Page::Loot => self
                .instance_loot_entries()
                .any(|loot| self.catalog.items.contains_key(&loot.item)),
            Page::Abilities => self.encounter.is_some() && self.instance_has_overview(),
            Page::Model => self.encounter.is_some(),
        }
    }
    /// EncounterJournal_ValidateSelectedTab: retain a valid tab, otherwise select
    /// the first enabled source tab in Overview/Loot/Abilities/Model order.
    fn validate_selected_page(&mut self) {
        if self.page_enabled(self.page) {
            return;
        }
        if let Some(page) = [Page::Overview, Page::Loot, Page::Abilities, Page::Model]
            .into_iter()
            .find(|page| self.page_enabled(*page))
        {
            self.page = page;
        }
    }
    pub(crate) fn available_difficulties(&self, instance: u32, encounter: Option<u32>) -> Vec<u32> {
        let Some(instance) = self.catalog.instance(instance) else {
            return Vec::new();
        };
        let Some(difficulties) = self.catalog.map_difficulties.get(&instance.map) else {
            return Vec::new();
        };
        let mut seen = BTreeSet::new();
        difficulties
            .iter()
            .flat_map(|id| [*id, crate::adventure_guide_menus::base_difficulty(*id)])
            .filter(|id| seen.insert(*id))
            .filter(|difficulty| {
                encounter.is_none_or(|id| {
                    difficulty_matches(&self.catalog.encounter_difficulties, id, *difficulty)
                })
            })
            .collect()
    }
    pub fn select_dungeon_encounter(
        &mut self,
        dungeon_encounter: u32,
        difficulty: Option<u32>,
    ) -> Result<(), String> {
        let encounter = self
            .catalog
            .encounters
            .iter()
            .find(|encounter| encounter.dungeon_encounter == dungeon_encounter)
            .ok_or_else(|| format!("No journal entry for encounter {dungeon_encounter}"))?;
        let id = encounter.id;
        if let Some(difficulty) = difficulty
            && !self
                .available_difficulties(encounter.instance, Some(id))
                .contains(&difficulty)
        {
            return Err(format!(
                "Journal difficulty {difficulty} unavailable for encounter {dungeon_encounter}"
            ));
        }
        self.select_encounter(id)?;
        if let Some(difficulty) = difficulty {
            self.difficulty = difficulty;
        }
        self.validate_selected_page();
        Ok(())
    }
    pub fn select_encounter(&mut self, id: u32) -> Result<(), String> {
        let e = self
            .catalog
            .encounter(id)
            .ok_or_else(|| format!("Unknown journal encounter {id}"))?;
        self.instance = Some(e.instance);
        self.encounter = Some(id);
        self.creature = self.model_creatures().first().map(|creature| creature.id);
        self.tab = if self.catalog.instance(e.instance).is_some_and(|i| i.dungeon) {
            Tab::Dungeons
        } else {
            Tab::Raids
        };
        let instance = self
            .catalog
            .instance(e.instance)
            .ok_or("Encounter instance missing")?;
        let difficulties = self.available_difficulties(e.instance, Some(id));
        if !difficulties.contains(&self.difficulty) {
            self.difficulty = *difficulties.first().ok_or("Instance has no difficulties")?;
        }
        if !instance.tiers.contains(&self.tier) {
            self.tier = *instance.tiers.first().ok_or("Instance tier data missing")?;
        }
        // Initialize overview role groups independently of the retained info tab.
        let previous_page = self.page;
        self.page = Page::Overview;
        self.visible = true;
        self.search.clear();
        self.tier_open = false;
        self.difficulty_open = false;
        self.filter_menu = None;
        self.collapsed.clear();
        let role = if self.role == 0 { 2 } else { self.role };
        let collapse_roles = self.shows_overview();
        let collapsed: Vec<_> = self
            .sections()?
            .into_iter()
            .filter(|(section, depth)| collapse_roles && *depth > 0 && section.roles & role == 0)
            .map(|(section, _)| section.id)
            .collect();
        self.collapsed.extend(collapsed);
        self.page = previous_page;
        self.validate_selected_page();
        Ok(())
    }
    pub fn model_creatures(&self) -> Vec<&game_engine_core::adventure_guide::Creature> {
        self.catalog
            .creatures
            .iter()
            .filter(|creature| Some(creature.encounter) == self.encounter)
            // Retail fills the first nine authored slots before checking displayInfo.
            .take(crate::adventure_creatures::MAX_CREATURES)
            .filter(|creature| creature.display != 0)
            .collect()
    }
    pub fn model_creature(&self) -> Option<&game_engine_core::adventure_guide::Creature> {
        self.model_creatures()
            .into_iter()
            .find(|creature| Some(creature.id) == self.creature)
    }
    pub fn sections(&self) -> Result<Vec<(&Section, usize)>, String> {
        let Some(encounter) = self.encounter.and_then(|id| self.catalog.encounter(id)) else {
            return Ok(Vec::new());
        };
        let mut output = Vec::new();
        let mut visited = BTreeSet::new();
        self.walk_sections(encounter.root, 0, false, &mut visited, &mut output)?;
        if self.shows_overview() {
            let role = if self.role == 0 { 2 } else { self.role };
            output.sort_by_key(|(section, depth)| {
                if *depth == 0 {
                    return 0;
                }
                if section.roles & role != 0 {
                    return 1;
                }
                match section.roles & 7 {
                    2 => 2,
                    4 => 3,
                    1 => 4,
                    _ => 5,
                }
            });
        }
        Ok(output)
    }
    fn walk_sections<'a>(
        &'a self,
        mut id: u32,
        depth: usize,
        in_overview: bool,
        visited: &mut BTreeSet<u32>,
        output: &mut Vec<(&'a Section, usize)>,
    ) -> Result<(), String> {
        while id != 0 {
            if !visited.insert(id) {
                return Err(format!("Journal section cycle at {id}"));
            }
            let section = self
                .catalog
                .sections
                .get(&id)
                .ok_or_else(|| format!("Missing journal section {id}"))?;
            let overview = in_overview || section.kind == 3;
            let page_matches = match self.page {
                Page::Overview if self.shows_overview() => overview,
                Page::Overview | Page::Abilities => !overview,
                _ => false,
            };
            if difficulty_matches(&self.catalog.section_difficulties, id, self.difficulty) {
                if page_matches {
                    output.push((section, depth));
                }
                if !page_matches || !self.collapsed.contains(&id) {
                    self.walk_sections(
                        section.child,
                        depth + usize::from(page_matches),
                        overview,
                        visited,
                        output,
                    )?;
                }
            }
            id = section.next;
        }
        Ok(())
    }
    fn instance_loot_entries(
        &self,
    ) -> impl Iterator<Item = &game_engine_core::adventure_guide::Loot> {
        let encounters: BTreeSet<_> = self.encounters().map(|encounter| encounter.id).collect();
        self.catalog.loot.iter().filter(move |loot| {
            encounters.contains(&loot.encounter)
                && difficulty_matches(&self.catalog.loot_difficulties, loot.id, self.difficulty)
        })
    }
    pub fn loot(&self) -> Vec<&Item> {
        let search = self.search.to_lowercase();
        let mut seen = BTreeSet::new();
        let items = self
            .instance_loot_entries()
            .filter(|loot| self.encounter.is_none_or(|id| loot.encounter == id))
            .filter_map(|loot| self.catalog.items.get(&loot.item));
        items
            .filter(|item| self.matches_loot_filters(item, &search))
            .filter(|item| seen.insert(item.id))
            .collect()
    }
    fn matches_loot_filters(&self, item: &Item, search: &str) -> bool {
        slot_matches(item.slot, self.slot)
            && (self.class == 0
                || item.class_mask == -1
                || item.class_mask & (1 << (self.class - 1)) != 0)
            && self.matches_specialization(item.id)
            && item.name.to_lowercase().contains(search)
    }
    pub(crate) fn loot_boss_names(&self) -> BTreeMap<u32, Vec<&str>> {
        let encounters: BTreeMap<_, _> = self
            .encounters()
            .map(|encounter| (encounter.id, encounter))
            .collect();
        let mut names: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for loot in self.instance_loot_entries() {
            if seen.insert((loot.item, loot.encounter)) {
                names
                    .entry(loot.item)
                    .or_default()
                    .push(&encounters[&loot.encounter].name);
            }
        }
        names
    }
    fn matches_specialization(&self, item: u32) -> bool {
        if self.class == 0 && self.spec == 0 {
            return true;
        }
        // ObjectMgr::LoadItemTemplates: no authored specialization means unrestricted.
        let Some(specs) = self.catalog.item_specs.get(&item) else {
            return true;
        };
        if self.spec != 0 {
            return specs.contains(&self.spec);
        }
        self.catalog
            .specs
            .iter()
            .any(|spec| spec.class == self.class && specs.contains(&spec.id))
    }
    pub fn apply(&mut self, action: &str) -> Result<(), String> {
        let mut parts = action.split(':');
        if parts.next() != Some("journal") {
            return Err(format!("Unknown journal action {action}"));
        }
        let key = parts.next().ok_or("Missing journal action")?;
        let value = parts.next();
        let number = || {
            value
                .ok_or("Missing journal ID")?
                .parse::<u32>()
                .map_err(|_| "Invalid journal ID")
        };
        match key {
            "close" => self.visible = false,
            "back" => {
                if self.tab == Tab::Journeys {
                    self.journeys.selected = None;
                }
                self.instance = None;
                self.encounter = None;
                self.search.clear();
            }
            "tab" => {
                self.tab = match value {
                    Some("journeys") => Tab::Journeys,
                    Some("dungeons") => Tab::Dungeons,
                    Some("raids") => Tab::Raids,
                    Some("sets") => Tab::ItemSets,
                    Some("tutorials") => Tab::Tutorials,
                    Some("monthly") => Tab::Monthly,
                    Some("suggested") => Tab::Suggested,
                    _ => return Err("Unknown journal tab".into()),
                };
                self.visible = true;
                if self.tab == Tab::ItemSets && self.spec == 0 {
                    if self.class == 0 {
                        self.class = self
                            .catalog
                            .specs
                            .iter()
                            .find(|spec| Some(spec.id) == self.player_spec)
                            .map_or(0, |spec| spec.class);
                    }
                    self.spec = self.choose_set_specialization();
                }
                self.instance = None;
                self.encounter = None;
                self.filter_menu = None;
                self.search.clear();
                self.activity_filters.expanded.clear();
            }
            "activity-tag" => {
                let tag = number()?;
                if self.activity_filters.tag != tag {
                    self.activity_filters.tag = tag;
                    self.activity_filters.expanded.clear();
                }
            }
            "activity" => {
                self.activity_filters
                    .toggle_group(self.activities.as_deref(), number()?);
            }
            "owned-keys" => {
                self.owned_keys_only = !self.owned_keys_only;
                self.tier_open = false;
            }
            "nav-overflow" => self.nav_overflow_open = !self.nav_overflow_open,
            "class-menu" => self.class_menu_open = !self.class_menu_open,
            "search-clear" => self.search.clear(),
            "tier-menu" => self.tier_open = !self.tier_open,
            "tier" => {
                self.tier_open = false;
                self.tier = number()?;
                self.instance = None;
                self.encounter = None;
            }
            "instance" => {
                let id = number()?;
                if self.catalog.instance(id).is_none() {
                    return Err(format!("Unknown instance {id}"));
                }
                self.instance = Some(id);
                self.encounter = None;
                let difficulties = self.available_difficulties(id, None);
                if !difficulties.contains(&self.difficulty) {
                    self.difficulty =
                        *difficulties.first().ok_or("Instance has no difficulties")?;
                }
                self.validate_selected_page();
                self.search.clear();
            }
            "encounter" => {
                self.select_encounter(number()?)?;
                self.search.clear();
            }
            "loot" => {
                let item = number()?;
                let encounter = self
                    .instance_loot_entries()
                    .find(|loot| {
                        loot.item == item && self.encounter.is_none_or(|id| loot.encounter == id)
                    })
                    .map(|loot| loot.encounter)
                    .ok_or_else(|| format!("Item {item} is not loot for this selection"))?;
                // Retail Loot_OnClick preserves state when this boss is already open.
                if self.encounter != Some(encounter) {
                    self.select_encounter(encounter)?;
                }
            }
            "creature" => {
                let id = number()?;
                if !self
                    .model_creatures()
                    .iter()
                    .any(|creature| creature.id == id)
                {
                    return Err(format!(
                        "Creature {id} is not a model choice for this encounter"
                    ));
                }
                self.creature = Some(id);
            }
            "page" => {
                let page = match value {
                    Some("overview") => Page::Overview,
                    Some("abilities") => Page::Abilities,
                    Some("loot") => Page::Loot,
                    Some("model") => Page::Model,
                    _ => return Err("Unknown journal page".into()),
                };
                if !self.page_enabled(page) {
                    return Err(format!(
                        "Journal page {page:?} is unavailable for this selection"
                    ));
                }
                self.page = page;
                if self.page == Page::Model {
                    self.creature = self.model_creatures().first().map(|creature| creature.id);
                }
            }
            "difficulty-menu" => self.difficulty_open = !self.difficulty_open,
            "difficulty" => {
                let difficulty = number()?;
                let instance = self.instance.ok_or("No journal instance selected")?;
                if !self
                    .available_difficulties(instance, self.encounter)
                    .contains(&difficulty)
                {
                    return Err(format!(
                        "Journal difficulty {difficulty} is unavailable for this selection"
                    ));
                }
                self.difficulty = difficulty;
                self.difficulty_open = false;
                self.validate_selected_page();
            }
            "role" => self.role = number()?,
            "filter" => {
                let menu = match value {
                    Some("class") => LootFilter::Class,
                    Some("spec") => LootFilter::Spec,
                    Some("slot") => LootFilter::Slot,
                    _ => return Err("Unknown loot filter".into()),
                };
                self.filter_menu = if self.filter_menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
            }
            "class" => {
                self.class = number()?;
                self.spec = if self.tab == Tab::ItemSets {
                    self.choose_set_specialization()
                } else {
                    0
                };
                self.filter_menu = None;
                self.class_menu_open = false;
            }
            "spec" => {
                self.spec = number()?;
                if let Some(spec) = self.catalog.specs.iter().find(|s| s.id == self.spec) {
                    self.class = spec.class;
                }
                self.filter_menu = None;
                self.class_menu_open = false;
            }
            "slot" => {
                self.slot = number()?;
                self.filter_menu = None;
            }
            "section" => {
                let id = number()?;
                if !self.collapsed.remove(&id) {
                    self.collapsed.insert(id);
                }
            }
            key if key.starts_with("journey") => crate::adventure_journeys::apply(
                &mut self.journeys,
                self.renown.as_deref(),
                key,
                value,
            )?,
            _ => return Err(format!("Unknown journal action {action}")),
        }
        Ok(())
    }
}
pub(crate) fn text(name: &str, value: &str, rect: (f32, f32, f32, f32)) -> Element {
    label(name.into(), value, rect, (12., WHITE, "LEFT"))
}

pub fn adventure_guide_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let view = ctx.get::<View>().expect("Adventure Guide view");
    if !view.visible {
        return vec![];
    }
    let mut body = portrait_border(
        "EncounterJournal",
        (800., 496.),
        crate::adventure_guide_art::content_title(view.tab),
        "journal:close",
    );
    body.extend(crate::adventure_guide_art::portrait(view));
    if matches!(view.tab, Tab::Dungeons | Tab::Raids | Tab::Journeys) {
        body.extend(crate::adventure_guide_art::navigation(view));
    }
    if matches!(view.tab, Tab::Dungeons | Tab::Raids | Tab::ItemSets) {
        body.extend(crate::adventure_guide_art::search_box(&view.search));
    }
    body.extend(crate::adventure_guide_art::bottom_tabs(view));
    body.extend(if view.loading {
        text(
            "EncounterJournalLoading",
            "Loading Adventure Guide…",
            (24., 100., 740., 40.),
        )
    } else {
        match view.tab {
            Tab::Journeys => crate::adventure_journeys::journeys(
                ctx,
                &view.journeys,
                view.renown.as_deref(),
                &view.journey_presentation,
            ),
            Tab::Tutorials => crate::adventure_tutorials::tutorials(&view.catalog),
            Tab::ItemSets => crate::adventure_sets::item_sets(ctx, view),
            Tab::Monthly => crate::adventure_activities::monthly(
                ctx,
                view.activities.as_deref(),
                &view.activity_filters,
            ),
            Tab::Suggested => {
                crate::adventure_suggestions::suggested(ctx, view.activities.as_deref())
            }
            _ => {
                if view.instance.is_some() {
                    encounter_page(ctx, view)
                } else {
                    instance_list(ctx, view)
                }
            }
        }
    });
    if !view.loading
        && matches!(view.tab, Tab::Dungeons | Tab::Raids)
        && view.search.chars().count() >= 3
        && view.page != Page::Loot
    {
        body.extend(search_results(ctx, view));
    }
    let left = (view.viewport[0] - 800.) / 2.;
    let top = (view.viewport[1] - 496.) / 2.;
    rsx! {r#frame {name:"EncounterJournalRoot",stretch:true,r#frame {name:"EncounterJournal",width:800.0,height:496.0,left,top,pos_type:"absolute",mouse_enabled:true,background_color:"0.08,0.07,0.06,1.0",strata:crate::ui::strata::FrameStrata::Dialog,{body}}}}
}
fn instance_list(ctx: &SharedContext, view: &View) -> Element {
    let background = crate::adventure_guide_art::grid_background(view);
    let mut controls = rsx! {r#frame {name:"EncounterJournalInstanceSelect",left:4.0,top:62.0,width:789.0,height:429.0,pos_type:"absolute",{background}}};
    // Retail tier dropdown. Native entries are separate named buttons when expanded.
    let title = view
        .catalog
        .tiers
        .iter()
        .find(|t| t.id == view.tier)
        .map(|t| t.name.as_str())
        .unwrap_or("");
    controls.extend(crate::adventure_guide_art::dropdown(
        "EncounterJournalTierDropdown",
        title,
        "journal:tier-menu",
        (609., 72., 160., 25.),
        view.tier_open,
    ));
    controls.extend(label(
        "EncounterJournalInstanceSelectTitle".into(),
        if view.tab == Tab::Dungeons {
            "Dungeons"
        } else {
            "Raids"
        },
        (24., 77., 240., 20.),
        (16., NORMAL_FONT_COLOR, "LEFT"),
    ));
    let rows = view.instances();
    let content = rows
        .iter()
        .enumerate()
        .flat_map(|(index, instance)| instance_card(index, instance, view))
        .collect();
    controls.extend(scroll(
        ctx,
        content,
        (18., 112., 748., 367.),
        (5. + rows.len().div_ceil(4) as f32 * 111.).max(367.),
    ));
    if view.tier_open {
        controls.extend(crate::adventure_guide_menus::tier_menu(view));
    }
    controls
}
fn instance_card(index: usize, instance: &Instance, view: &View) -> Element {
    let left = (index % 4) as f32 * 189.;
    let top = 5. + (index / 4) as f32 * 111.;
    let mut art = crate::bank_art::cropped(
        format!("JournalInstance{}Art", instance.id),
        instance.button,
        "0,0.68359375,0,0.7421875",
        (0., 0., 174., 96.),
    );
    art.extend(crate::adventure_guide_art::art(
        format!("JournalInstance{}ButtonArt", instance.id),
        crate::adventure_guide_art::Art::DungeonUp,
        (0., 0., 174., 96.),
    ));
    art.extend(crate::adventure_guide_art::highlight(
        format!("JournalInstance{}ButtonHighlight", instance.id),
        crate::adventure_guide_art::Art::DungeonHighlight,
        (0., 0., 174., 96.),
    ));
    art.extend(label(
        format!("JournalInstance{}Title", instance.id),
        &instance.name,
        (12., 15., 150., 60.),
        (18., crate::adventure_guide_art::GOLD, "CENTER"),
    ));
    if let Some((key, _)) = view.owned_keys(instance.map).first() {
        art.extend(text(
            &format!("JournalInstance{}Key", instance.id),
            &format!("+{}", key.level),
            (130., 80., 50., 22.),
        ));
    }
    let name = DynName(format!("JournalInstance{}", instance.id));
    let action = format!("journal:instance:{}", instance.id);
    rsx! {button {name:{name},left,top,width:174.0,height:96.0,pos_type:"absolute",button_default_skin:false,onclick:{action.as_str()},{art}}}
}
fn encounter_page(ctx: &SharedContext, view: &View) -> Element {
    use crate::adventure_guide_art::{Art, HEADER, INFO, INK, MODEL, art};
    let Some(instance) = view.instance.and_then(|id| view.catalog.instance(id)) else {
        return vec![];
    };
    let mut out = crate::bank_art::cropped(
        "EncounterJournalInfo".into(),
        521750,
        "0,0.766601562,0,0.830078125",
        INFO,
    );
    out.extend(art(
        "EncounterJournalLeftHeader".into(),
        Art::LeftHeader,
        (7., 75., 386., 39.),
    ));
    out.extend(crate::adventure_guide_art::instance_button(instance));
    out.extend(label(
        "EncounterJournalLeftTitle".into(),
        &instance.name,
        (72., 84., 290., 12.),
        (14., HEADER, "LEFT"),
    ));
    let bosses: Vec<_> = view.encounters().collect();
    let rows = bosses
        .iter()
        .enumerate()
        .flat_map(|(index, e)| crate::adventure_guide_art::boss_row(view, e, index))
        .collect();
    out.extend(scroll_in(
        ctx,
        rows,
        (32., 106., 338., 382.),
        (10. + bosses.len() as f32 * 70.).max(382.),
        "EncounterJournalBosses",
    ));
    out.extend(crate::adventure_guide_art::side_tabs(view));
    if view.encounter.is_none() && view.page == Page::Loot {
        out.extend(art(
            "EncounterJournalRightHeader".into(),
            Art::RightHeader,
            (406., 75., 386., 39.),
        ));
        out.extend(loot_page(ctx, view));
        out.extend(difficulty_button(view));
        if view.difficulty_open {
            out.extend(difficulty_menu(ctx, view, instance));
        }
        return out;
    }
    let Some(encounter) = view.encounter.and_then(|id| view.catalog.encounter(id)) else {
        let mut lore = crate::adventure_guide_art::instance_lore(instance);
        lore.extend(instance_lore_scroll(ctx, instance));
        out.extend(rsx! {r#frame {name:"EncounterJournalInstanceFrame",left:402.0,top:64.0,width:390.0,height:425.0,pos_type:"absolute",{lore}}});
        out.extend(difficulty_button(view));
        let keys = [view.owned_key_text(), view.challenge_text()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        if !keys.is_empty() {
            out.extend(label(
                "EncounterJournalKeystone".into(),
                &keys,
                (437., 305., 315., 75.),
                (12., INK, "LEFT"),
            ));
        }
        if let Some((_, status)) = view.lockouts.iter().find(|(map, _)| *map == instance.map) {
            out.extend(label(
                "EncounterJournalLockout".into(),
                status,
                (437., 369., 315., 20.),
                (10., INK, "LEFT"),
            ));
        }
        if view.difficulty_open {
            out.extend(difficulty_menu(ctx, view, instance));
        }
        return out;
    };
    if view.page == Page::Model {
        let title = view
            .model_creature()
            .map_or("", |creature| creature.name.as_str());
        let model_art = crate::adventure_guide_art::model_art(instance, title);
        let (left, top, width, height) = MODEL;
        out.extend(rsx! {r#frame {name:"EncounterJournalModelScene",left,top,width,height,pos_type:"absolute",mouse_enabled:true,{model_art}}});
        out.extend(crate::adventure_creatures::buttons(view));
        return out;
    }
    out.extend(art(
        "EncounterJournalRightHeader".into(),
        Art::RightHeader,
        (406., 75., 386., 39.),
    ));
    if view.page != Page::Loot {
        out.extend(label(
            "EncounterJournalEncounterTitle".into(),
            &encounter.name,
            (442., 88., 220., 12.),
            (14., HEADER, "LEFT"),
        ));
    }
    out.extend(difficulty_button(view));
    match view.page {
        Page::Loot => out.extend(loot_page(ctx, view)),
        _ => out.extend(description_page(ctx, view, encounter)),
    }
    if view.difficulty_open {
        out.extend(difficulty_menu(ctx, view, instance));
    }
    out
}

fn instance_lore_scroll(ctx: &SharedContext, instance: &Instance) -> Element {
    let list = "EncounterJournalLore";
    let content_height = paragraph_height(&instance.description, 45.).max(95.);
    // XML bar is29px high, less than the shared38px arrow inset: it has no drag track.
    let geometry = pixel_geometry(95., content_height, 38.);
    let offset = geometry.clamp(ctx.scroll_first_row(list));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let text = paper_text(
        "EncounterJournalInstanceLore",
        &instance.description,
        (0., child_top, 315., content_height),
    );
    let mut out = rsx! {r#frame {name:"EncounterJournalLore",left:35.0,top:325.0,width:315.0,height:95.0,pos_type:"absolute",mouse_enabled:true,scroll_list:config,{text}}};
    let bar = MinimalScrollBar {
        list,
        left: 359.,
        top: 358.,
        height: 29.,
        geometry,
        offset,
        unscrollable: Unscrollable::HideBar,
    };
    out.extend(bar.element());
    out
}

fn description_page(
    ctx: &SharedContext,
    view: &View,
    encounter: &game_engine_core::adventure_guide::Encounter,
) -> Element {
    let height = paragraph_height(&encounter.description, 46.);
    let mut content = paper_text(
        "JournalEncounterLore",
        &encounter.description,
        (2., 8., 315., height),
    );
    let mut y = 8. + height + 6.;
    if view.shows_overview() {
        content.extend(crate::adventure_guide_sections::overview_header(y));
        y += 30. + 6.;
    }
    append_sections(&mut content, &mut y, view);
    scroll(ctx, content, (437., 105., 350., 383.), y.max(383.))
}
fn append_sections(content: &mut Element, y: &mut f32, view: &View) {
    let sections = match view.sections() {
        Ok(sections) => sections,
        Err(error) => {
            content.extend(paper_text(
                "JournalSectionError",
                &error,
                (2., *y, 315., 60.),
            ));
            *y += 60.;
            return;
        }
    };
    let overview_page = view.shows_overview();
    for (section, depth) in sections {
        if overview_page && depth == 0 {
            crate::adventure_guide_sections::append_overview_body(
                content,
                y,
                &format!("JournalSection{}Body", section.id),
                &section.body,
            );
            continue;
        }
        let indent = if overview_page {
            0.
        } else {
            depth as f32 * 15.
        };
        let collapsed = view.collapsed.contains(&section.id);
        let title = section.title.as_str();
        content.extend(section_row(
            section,
            title,
            (indent, *y, 320. - indent, 24.),
            collapsed,
        ));
        *y += 26.;
        if !collapsed && !section.body.is_empty() {
            if overview_page {
                crate::adventure_guide_sections::append_overview_body(
                    content,
                    y,
                    &format!("JournalSection{}Body", section.id),
                    &section.body,
                );
                continue;
            }
            let width = 310. - indent;
            let height = paragraph_height(&section.body, width / 7.);
            content.extend(paper_text(
                &format!("JournalSection{}Body", section.id),
                &section.body,
                (indent + 10., *y, width, height),
            ));
            *y += height + 8.;
        }
    }
}
pub(crate) fn paragraph_height(value: &str, columns: f32) -> f32 {
    value
        .lines()
        .map(|line| (line.chars().count() as f32 / columns).ceil().max(1.))
        .sum::<f32>()
        * 16.
        + 12.
}
pub(crate) fn paper_text(name: &str, value: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    let color = if name == "EncounterJournalInstanceLore" {
        crate::adventure_guide_art::INK
    } else {
        "0.25,0.1484375,0.02,1"
    };
    // GameFontBlack inherits unshadowed SystemFont_Med1, unlike gold header fonts.
    rsx! {fontstring {name:{DynName(name.into())},text:value,left,top,width,height,pos_type:"absolute",font_size:12.0,font_color:color,justify_h:"LEFT"}}
}
fn section_row(
    section: &Section,
    title: &str,
    rect: (f32, f32, f32, f32),
    collapsed: bool,
) -> Element {
    crate::adventure_guide_sections::section_header(section, title, rect, collapsed)
}

fn difficulty_button(view: &View) -> Element {
    let name = crate::adventure_guide_menus::difficulty_caption(view, view.difficulty);
    crate::adventure_guide_art::dropdown(
        "JournalDifficultyDropdown",
        &name,
        "journal:difficulty-menu",
        (673., 77., 100., 25.),
        view.difficulty_open,
    )
}
fn difficulty_menu(_ctx: &SharedContext, view: &View, instance: &Instance) -> Element {
    crate::adventure_guide_menus::difficulty_menu(view, instance.id)
}
fn loot_page(ctx: &SharedContext, view: &View) -> Element {
    let mut out = loot_filter_buttons(view);
    let (content, content_height) = crate::adventure_loot::rows(view);
    let list = scroll(
        ctx,
        content,
        (-20., -1., 345., 382.),
        content_height.max(382.),
    );
    out.extend(rsx! {r#frame {name:"EncounterJournalLootContainer",left:442.0,top:106.0,width:345.0,height:382.0,pos_type:"absolute",{list}}});
    if let Some(menu) = view.filter_menu {
        out.extend(crate::adventure_guide_menus::loot_menu(view, menu));
    }
    out
}
// Retail SlotFilterToSlotName has MainHand, OffHand and Other, not separate robe/ranged entries.
const OTHER_LOOT_SLOT: u32 = 99;
fn slot_filters(inventory_type: u32) -> impl Iterator<Item = u32> {
    let filters = match inventory_type {
        1..=3 | 5..=12 | 16 => [inventory_type, 0],
        20 => [5, 0],
        13 => [21, 22],
        15 | 17 | 21 | 25 | 26 => [21, 0],
        14 | 22 | 23 => [22, 0],
        _ => [OTHER_LOOT_SLOT, 0],
    };
    filters.into_iter().filter(|id| *id != 0)
}
fn slot_matches(inventory_type: u32, filter: u32) -> bool {
    filter == 0 || slot_filters(inventory_type).any(|id| id == filter)
}
fn slot_name(id: u32) -> &'static str {
    match id {
        1 => "Head",
        2 => "Neck",
        3 => "Shoulder",
        4 => "Shirt",
        5 | 20 => "Chest",
        6 => "Waist",
        7 => "Legs",
        8 => "Feet",
        9 => "Wrist",
        10 => "Hands",
        11 => "Finger",
        12 => "Trinket",
        13 => "One-Hand",
        14 => "Shield",
        15 | 26 => "Ranged",
        16 => "Back",
        17 => "Two-Hand",
        19 => "Tabard",
        21 => "Main Hand",
        22 => "Off Hand",
        23 => "Held in Off Hand",
        25 => "Thrown",
        28 => "Relic",
        _ => "Other",
    }
}
fn loot_filter_buttons(view: &View) -> Element {
    let class = view
        .catalog
        .classes
        .get(&view.class)
        .map(String::as_str)
        .unwrap_or(crate::auction_mail::text("ALL_CLASSES"));
    let spec = view.catalog.specs.iter().find(|s| s.id == view.spec);
    let title = spec.map_or_else(
        || class.to_string(),
        |spec| format!("{class} ({})", spec.name),
    );
    let slot = if view.slot == 0 {
        crate::auction_mail::text("ALL_INVENTORY_SLOTS")
    } else {
        slot_name(view.slot)
    };
    let mut out = crate::adventure_guide_art::dropdown(
        "JournalClassFilter",
        &title,
        "journal:filter:spec",
        (444., 77., 120., 25.),
        view.filter_menu == Some(LootFilter::Spec),
    );
    out.extend(crate::adventure_guide_art::dropdown(
        "JournalSlotFilter",
        slot,
        "journal:filter:slot",
        (574., 77., 90., 25.),
        view.filter_menu == Some(LootFilter::Slot),
    ));
    out
}
pub(crate) fn loot_filter_entries(view: &View, menu: LootFilter) -> Vec<(u32, String)> {
    match menu {
        LootFilter::Class => std::iter::once((0, "All classes".into()))
            .chain(
                view.catalog
                    .classes
                    .iter()
                    .map(|(id, name)| (*id, name.clone())),
            )
            .collect(),
        LootFilter::Spec => std::iter::once((0, "All specs".into()))
            .chain(
                view.catalog
                    .specs
                    .iter()
                    .filter(|s| s.class == view.class && s.name != "Initial")
                    .map(|s| (s.id, s.name.clone())),
            )
            .collect(),
        LootFilter::Slot => {
            let mut unfiltered = view.clone();
            unfiltered.slot = 0;
            let slots: BTreeSet<_> = unfiltered
                .loot()
                .iter()
                .flat_map(|item| slot_filters(item.slot))
                .collect();
            std::iter::once((0, "All slots".into()))
                .chain(slots.into_iter().map(|id| (id, slot_name(id).into())))
                .collect()
        }
    }
}
fn search_results(ctx: &SharedContext, view: &View) -> Element {
    let search = view.search.to_lowercase();
    let instances = view
        .catalog
        .instances
        .iter()
        .filter(|i| i.name.to_lowercase().contains(&search))
        .map(|i| (i.name.clone(), format!("journal:instance:{}", i.id)));
    let encounters = view
        .catalog
        .encounters
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&search))
        .map(|e| (e.name.clone(), format!("journal:encounter:{}", e.id)));
    let sections = view
        .catalog
        .sections
        .values()
        .filter(|s| s.title.to_lowercase().contains(&search))
        .map(|s| {
            (
                s.title.clone(),
                format!("journal:encounter:{}", s.encounter),
            )
        });
    let matches: Vec<_> = instances.chain(encounters).chain(sections).collect();
    let rows: Element = matches
        .iter()
        .enumerate()
        .map(|(index, (title, action))| {
            crate::selection_list::text_row(
                format!("JournalSearchResult{index}"),
                title,
                action,
                (true, false),
                (0., index as f32 * 26., 546., 24.),
                &crate::selection_list::SEARCH,
            )
        })
        .flatten()
        .collect();
    let content_height = (matches.len() as f32 * 26.).max(330.);
    let list = scroll_in(
        ctx,
        rows,
        (0., 0., 564., 330.),
        content_height,
        "EncounterJournalSearchList",
    );
    rsx! {r#frame {name:"EncounterJournalSearchResults",left:210.0,top:62.0,width:568.0,height:334.0,pos_type:"absolute",mouse_enabled:true,frame_level:25.0,background_color:"0.08,0.07,0.06,1.0",{list}}}
}

pub(crate) fn scroll(
    ctx: &SharedContext,
    children: Element,
    rect: (f32, f32, f32, f32),
    content_height: f32,
) -> Element {
    scroll_in(ctx, children, rect, content_height, LIST)
}
pub(crate) fn scroll_in(
    ctx: &SharedContext,
    children: Element,
    rect: (f32, f32, f32, f32),
    content_height: f32,
    list: &'static str,
) -> Element {
    let (left, top, width, height) = rect;
    // XML InstanceSelect.ScrollBar +12,-6 / +12,-4; BossesScrollBar +5,-5 / +5,+5.
    let (bar_left, bar_top, bar_height, child_width) = if list == "EncounterJournalJourneysList" {
        // JourneysFrameTemplate: ScrollBar LEFT list.RIGHT-20, TOP/BOTTOM10.
        (left + width - 20., top + 10., height - 20., width)
    } else if list == "EncounterJournalBosses" {
        (left + width + 5., top + 5., height - 10., width)
    } else if list == LIST && width == 521. {
        // MonthlyActivities.ScrollBar TOPRIGHT+8,-6 / BOTTOMRIGHT+8,+6.
        (left + width + 8., top + 6., height - 12., width)
    } else if list == LIST && width == 762. {
        // LootJournalItems.xml ItemSetsFrame.ScrollBar: TOPRIGHT+8,-3 / BOTTOMRIGHT+8,+5.
        (left + width + 8., top + 3., height - 8., width)
    } else if list == LIST && width == 345. {
        (left + width + 5., top + 5., height - 10., width)
    } else if list == LIST && width == 748. {
        (left + width + 12., top + 6., height - 2., width)
    } else {
        (left + width - 14., top, height, width - 18.)
    };
    let geometry = pixel_geometry(height, content_height, bar_height);
    let offset = geometry.clamp(ctx.scroll_first_row(list));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let bar = MinimalScrollBar {
        list,
        left: bar_left,
        top: bar_top,
        height: bar_height,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let list_name = DynName(list.into());
    let child_name = DynName(format!("{list}Child"));
    let mut out = rsx! {r#frame {name:{list_name},left,top,width,height,pos_type:"absolute",mouse_enabled:true,scroll_list:config,
    r#frame {name:{child_name},left:0.0,top:child_top,width:child_width,height:content_height,pos_type:"absolute",{children}}}};
    out.extend(bar.element());
    out
}
