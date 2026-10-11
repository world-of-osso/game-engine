//! Retail Mainline Adventure Guide; both skins share the same 800×496 layout.
use crate::bank_art::{WHITE, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, portrait_border};
use game_engine_core::adventure_guide::{Catalog, Instance, Item, Section, difficulty_matches};
use std::{collections::BTreeSet, sync::Arc};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "EncounterJournalSearchBox";
pub const LIST: &str = "EncounterJournalContent";
pub const SCROLL_LISTS: [&str; 4] = [
    LIST,
    "EncounterJournalSearchList",
    "EncounterJournalFilterList",
    "EncounterJournalDifficultyList",
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Dungeons,
    Raids,
    ItemSets,
    Tutorials,
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
    pub instance: Option<u32>,
    pub encounter: Option<u32>,
    pub difficulty: u32,
    pub role: u32,
    pub class: u32,
    pub spec: u32,
    pub slot: u32,
    pub filter_menu: Option<LootFilter>,
    pub search: String,
    pub collapsed: BTreeSet<u32>,
    pub lockouts: Vec<(u32, String)>,
    pub challenge: Option<shared::protocol::ChallengeRun>,
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
            instance: None,
            encounter: None,
            difficulty: 2,
            role: 0,
            class: 0,
            spec: 0,
            slot: 0,
            filter_menu: None,
            search: String::new(),
            collapsed: BTreeSet::new(),
            lockouts: Vec::new(),
            challenge: None,
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
    pub fn instances(&self) -> Vec<&Instance> {
        let search = self.search.to_lowercase();
        self.catalog
            .instances
            .iter()
            .filter(|i| {
                i.dungeon == (self.tab == Tab::Dungeons)
                    && i.tiers.contains(&self.tier)
                    && i.name.to_lowercase().contains(&search)
            })
            .collect()
    }
    fn available_difficulties(&self, instance: u32, encounter: Option<u32>) -> Vec<u32> {
        let Some(instance) = self.catalog.instance(instance) else {
            return Vec::new();
        };
        let Some(difficulties) = self.catalog.map_difficulties.get(&instance.map) else {
            return Vec::new();
        };
        difficulties
            .iter()
            .copied()
            .filter(|difficulty| {
                encounter.is_none_or(|id| {
                    difficulty_matches(&self.catalog.encounter_difficulties, id, *difficulty)
                })
            })
            .collect()
    }
    pub fn select_encounter(&mut self, id: u32) -> Result<(), String> {
        let e = self
            .catalog
            .encounter(id)
            .ok_or_else(|| format!("Unknown journal encounter {id}"))?;
        self.instance = Some(e.instance);
        self.encounter = Some(id);
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
        self.page = Page::Overview;
        self.visible = true;
        self.collapsed.clear();
        Ok(())
    }
    pub fn sections(&self) -> Result<Vec<(&Section, usize)>, String> {
        let Some(encounter) = self.encounter.and_then(|id| self.catalog.encounter(id)) else {
            return Ok(Vec::new());
        };
        let mut output = Vec::new();
        let mut visited = BTreeSet::new();
        self.walk_sections(encounter.root, 0, false, &mut visited, &mut output)?;
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
            let role_matches =
                self.role == 0 || section.roles & 7 == 0 || section.roles & self.role != 0;
            let page_matches = match self.page {
                Page::Overview => overview,
                Page::Abilities => !overview,
                _ => false,
            };
            if difficulty_matches(&self.catalog.section_difficulties, id, self.difficulty)
                && role_matches
            {
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
    pub fn loot(&self) -> Vec<&Item> {
        let search = self.search.to_lowercase();
        self.catalog
            .loot
            .iter()
            .filter(|l| {
                Some(l.encounter) == self.encounter
                    && difficulty_matches(&self.catalog.loot_difficulties, l.id, self.difficulty)
            })
            .filter_map(|l| self.catalog.items.get(&l.item))
            .filter(|i| {
                slot_matches(i.slot, self.slot)
                    && (self.class == 0
                        || i.class_mask == -1
                        || i.class_mask & (1 << (self.class - 1)) != 0)
                    && self.matches_specialization(i.id)
                    && i.name.to_lowercase().contains(&search)
            })
            .collect()
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
                self.instance = None;
                self.encounter = None;
                self.search.clear();
            }
            "tab" => {
                self.tab = match value {
                    Some("dungeons") => Tab::Dungeons,
                    Some("raids") => Tab::Raids,
                    Some("sets") => Tab::ItemSets,
                    Some("tutorials") => Tab::Tutorials,
                    _ => return Err("Unknown journal tab".into()),
                };
                self.instance = None;
                self.encounter = None;
                self.search.clear();
            }
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
                self.page = Page::Overview;
                self.search.clear();
            }
            "encounter" => {
                self.select_encounter(number()?)?;
                self.search.clear();
            }
            "page" => {
                self.page = match value {
                    Some("overview") => Page::Overview,
                    Some("abilities") => Page::Abilities,
                    Some("loot") => Page::Loot,
                    Some("model") => Page::Model,
                    _ => return Err("Unknown journal page".into()),
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
                self.spec = 0;
                self.filter_menu = None;
            }
            "spec" => {
                self.spec = number()?;
                self.filter_menu = None;
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
            _ => return Err(format!("Unknown journal action {action}")),
        }
        Ok(())
    }
}
fn button(name: String, text: &str, action: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    rsx! {button {name:{DynName(name)},text,onclick:action,left,top,width,height,pos_type:"absolute",font_size:12.0,font_color:NORMAL_FONT_COLOR,button_atlas_up:"defaultbutton-nineslice-up",button_atlas_pressed:"defaultbutton-nineslice-pressed",button_atlas_highlight:"defaultbutton-nineslice-highlight"}}
}
fn text(name: &str, value: &str, rect: (f32, f32, f32, f32)) -> Element {
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
        "Adventure Guide",
        "journal:close",
    );
    body.extend(button(
        "EncounterJournalBack".into(),
        "Adventure Guide",
        "journal:back",
        (64., 29., 170., 22.),
    ));
    let search = view.search.as_str();
    body.extend(rsx!{editbox {name:"EncounterJournalSearchBox",text:search,left:580.0,top:32.0,width:210.0,height:20.0,font_size:12.0,font_color:WHITE,max_letters:80,text_insets:"4,4,0,0",pos_type:"absolute"}});
    for (index, (title, action)) in [
        ("Dungeons", "dungeons"),
        ("Raids", "raids"),
        ("Item Sets", "sets"),
        ("Tutorials", "tutorials"),
    ]
    .into_iter()
    .enumerate()
    {
        body.extend(button(
            format!("EncounterJournalTab{index}"),
            title,
            &format!("journal:tab:{action}"),
            (12. + index as f32 * 120., 496., 117., 24.),
        ));
    }
    body.extend(if view.loading {
        text(
            "EncounterJournalLoading",
            "Loading Adventure Guide…",
            (24., 100., 740., 40.),
        )
    } else {
        match view.tab {
            Tab::Tutorials => tutorials(view),
            Tab::ItemSets => sets(ctx, view),
            _ => {
                if view.instance.is_some() {
                    encounter_page(ctx, view)
                } else {
                    instance_list(ctx, view)
                }
            }
        }
    });
    if !view.loading && view.search.chars().count() >= 3 && view.page != Page::Loot {
        body.extend(search_results(ctx, view));
    }
    let left = (view.viewport[0] - 800.) / 2.;
    let top = (view.viewport[1] - 496.) / 2.;
    rsx! {r#frame {name:"EncounterJournalRoot",stretch:true,r#frame {name:"EncounterJournal",width:800.0,height:496.0,left,top,pos_type:"absolute",mouse_enabled:true,background_color:"0.08,0.07,0.06,1.0",strata:crate::ui::strata::FrameStrata::Dialog,{body}}}}
}
fn instance_list(ctx: &SharedContext, view: &View) -> Element {
    let mut controls = Vec::new();
    // Retail tier dropdown. Native entries are separate named buttons when expanded.
    let title = view
        .catalog
        .tiers
        .iter()
        .find(|t| t.id == view.tier)
        .map(|t| t.name.as_str())
        .unwrap_or("");
    controls.extend(button(
        "EncounterJournalTierDropdown".into(),
        title,
        "journal:tier-menu",
        (15., 65., 240., 24.),
    ));
    let rows = view.instances();
    let content = rows
        .iter()
        .enumerate()
        .flat_map(|(index, instance)| instance_card(index, instance))
        .collect();
    controls.extend(scroll(
        ctx,
        content,
        (15., 96., 770., 387.),
        (rows.len().div_ceil(4) as f32 * 130.).max(387.),
    ));
    if view.tier_open {
        let options: Element = view
            .catalog
            .tiers
            .iter()
            .enumerate()
            .flat_map(|(index, tier)| {
                button(
                    format!("EncounterJournalTier{}", tier.id),
                    &tier.name,
                    &format!("journal:tier:{}", tier.id),
                    (0., index as f32 * 24., 240., 23.),
                )
            })
            .collect();
        controls.extend(rsx! { r#frame {name:"EncounterJournalTierMenu",left:15.0,top:89.0,width:240.0,height:312.0,pos_type:"absolute",frame_level:20.0,mouse_enabled:true,background_color:"0.08,0.07,0.06,1.0",{options}} });
    }
    controls
}
fn instance_card(index: usize, instance: &Instance) -> Element {
    let left = (index % 4) as f32 * 191.;
    let top = (index / 4) as f32 * 130.;
    let mut art = texture(
        format!("JournalInstance{}Art", instance.id),
        instance.button,
        (0., 0., 185., 105.),
        WHITE,
    );
    art.extend(text(
        &format!("JournalInstance{}Title", instance.id),
        &instance.name,
        (4., 106., 177., 22.),
    ));
    let name = DynName(format!("JournalInstance{}", instance.id));
    let action = format!("journal:instance:{}", instance.id);
    rsx! {r#frame {name:{name},left,top,width:185.0,height:128.0,pos_type:"absolute",mouse_enabled:true,onclick:{action.as_str()},{art}}}
}
fn encounter_page(ctx: &SharedContext, view: &View) -> Element {
    let Some(instance) = view.instance.and_then(|id| view.catalog.instance(id)) else {
        return vec![];
    };
    let mut out = texture(
        "EncounterJournalInstanceBackground".into(),
        instance.background,
        (8., 62., 784., 424.),
        WHITE,
    );
    out.extend(button(
        "EncounterJournalInstanceOverview".into(),
        &instance.name,
        &format!("journal:instance:{}", instance.id),
        (12., 64., 172., 28.),
    ));
    for (index, e) in view
        .catalog
        .encounters
        .iter()
        .filter(|e| {
            e.instance == instance.id
                && difficulty_matches(&view.catalog.encounter_difficulties, e.id, view.difficulty)
        })
        .enumerate()
    {
        out.extend(button(
            format!("JournalBoss{}", e.id),
            &e.name,
            &format!("journal:encounter:{}", e.id),
            (12., 100. + index as f32 * 27., 172., 25.),
        ));
    }
    if let Some((_, status)) = view.lockouts.iter().find(|(map, _)| *map == instance.map) {
        out.extend(text(
            "EncounterJournalLockout",
            status,
            (190., 65., 360., 22.),
        ));
    }
    let Some(encounter) = view.encounter.and_then(|id| view.catalog.encounter(id)) else {
        if let Some(challenge) = view.challenge_text() {
            out.extend(text(
                "EncounterJournalKeystone",
                &challenge,
                (205., 295., 550., 175.),
            ));
        }
        out.extend(text(
            "EncounterJournalInstanceLore",
            &instance.description,
            (
                205.,
                105.,
                550.,
                if view.challenge_text().is_some() {
                    175.
                } else {
                    340.
                },
            ),
        ));
        return out;
    };
    out.extend(text(
        "EncounterJournalEncounterTitle",
        &encounter.name,
        (205., 65., 350., 24.),
    ));
    for (index, (title, page)) in [
        ("Overview", "overview"),
        ("Abilities", "abilities"),
        ("Loot", "loot"),
        ("Model", "model"),
    ]
    .into_iter()
    .enumerate()
    {
        out.extend(button(
            format!("EncounterJournalPage{index}"),
            title,
            &format!("journal:page:{page}"),
            (202. + index as f32 * 140., 96., 137., 23.),
        ));
    }
    out.extend(difficulty_button(view));
    match view.page {
        Page::Model => {
            out.extend(rsx!{r#frame {name:"EncounterJournalModelScene",left:210.0,top:155.0,width:565.0,height:325.0,pos_type:"absolute",mouse_enabled:true}});
        }
        Page::Loot => out.extend(loot_page(ctx, view)),
        _ => {
            for (index, (id, title)) in
                [(0, "All roles"), (1, "Tank"), (2, "Damage"), (4, "Healer")]
                    .into_iter()
                    .enumerate()
            {
                out.extend(button(
                    format!("JournalRole{id}"),
                    title,
                    &format!("journal:role:{id}"),
                    (202. + index as f32 * 140., 149., 136., 22.),
                ));
            }
            let mut y = 0.;
            let mut content = Vec::new();
            if view.page == Page::Overview {
                content.extend(text(
                    "JournalEncounterLore",
                    &encounter.description,
                    (4., y, 545., 70.),
                ));
                y += 76.;
            }
            match view.sections() {
                Ok(sections) => {
                    for (section, depth) in sections {
                        let indent = depth as f32 * 15.;
                        let title = format!(
                            "{} {}",
                            if view.collapsed.contains(&section.id) {
                                "+"
                            } else {
                                "−"
                            },
                            section.title
                        );
                        content.extend(button(
                            format!("JournalSection{}", section.id),
                            &title,
                            &format!("journal:section:{}", section.id),
                            (indent, y, 545. - indent, 24.),
                        ));
                        if section.icon != 0 {
                            content.extend(texture(
                                format!("JournalSection{}Icon", section.id),
                                section.icon,
                                (indent + 2., y + 2., 20., 20.),
                                WHITE,
                            ));
                        }
                        y += 26.;
                        if !view.collapsed.contains(&section.id) && !section.body.is_empty() {
                            let height =
                                (section.body.chars().count() as f32 / 72.).ceil() * 16. + 12.;
                            content.extend(text(
                                &format!("JournalSection{}Body", section.id),
                                &section.body,
                                (indent + 10., y, 535. - indent, height),
                            ));
                            y += height + 8.;
                        }
                    }
                }
                Err(error) => {
                    content.extend(text("JournalSectionError", &error, (4., 0., 545., 60.)));
                    y = 60.;
                }
            }
            out.extend(scroll(ctx, content, (202., 179., 575., 300.), y.max(300.)));
        }
    }
    if view.difficulty_open {
        out.extend(difficulty_menu(ctx, view, instance));
    }
    out
}
fn difficulty_button(view: &View) -> Element {
    let name = view
        .catalog
        .difficulties
        .get(&view.difficulty)
        .map(String::as_str)
        .unwrap_or("");
    button(
        "JournalDifficultyDropdown".into(),
        name,
        "journal:difficulty-menu",
        (602., 123., 174., 22.),
    )
}
fn difficulty_menu(ctx: &SharedContext, view: &View, instance: &Instance) -> Element {
    let entries = view.available_difficulties(instance.id, view.encounter);
    let height = (entries.len() as f32 * 24.).min(300.);
    let rows = entries
        .iter()
        .enumerate()
        .flat_map(|(index, id)| {
            let title = view
                .catalog
                .difficulties
                .get(id)
                .map(String::as_str)
                .unwrap_or("");
            button(
                format!("JournalDifficulty{id}"),
                title,
                &format!("journal:difficulty:{id}"),
                (0., index as f32 * 24., 164., 23.),
            )
        })
        .collect();
    let list = scroll_in(
        ctx,
        rows,
        (0., 0., 174., height),
        entries.len() as f32 * 24.,
        "EncounterJournalDifficultyList",
    );
    rsx! {r#frame {name:"EncounterJournalDifficultyMenu",left:602.0,top:147.0,width:174.0,height,pos_type:"absolute",frame_level:30.0,mouse_enabled:true,background_color:"0.08,0.07,0.06,1.0",{list}}}
}
fn loot_page(ctx: &SharedContext, view: &View) -> Element {
    let mut out = loot_filter_buttons(view);
    let items = view.loot();
    let content = items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            let y = index as f32 * 44.;
            let mut row = texture(
                format!("JournalLoot{}Icon", item.id),
                item.icon,
                (2., y + 2., 36., 36.),
                WHITE,
            );
            row.extend(text(
                &format!("JournalLoot{}Name", item.id),
                &item.name,
                (46., y + 2., 475., 22.),
            ));
            row.extend(text(
                &format!("JournalLoot{}Slot", item.id),
                &format!("Item level {} · {}", item.level, slot_name(item.slot)),
                (46., y + 24., 475., 16.),
            ));
            row
        })
        .collect();
    out.extend(scroll(
        ctx,
        content,
        (202., 185., 575., 294.),
        (items.len() as f32 * 44.).max(294.),
    ));
    if let Some(menu) = view.filter_menu {
        out.extend(loot_filter_menu(ctx, view, menu));
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
        .unwrap_or("All classes");
    let spec = view
        .catalog
        .specs
        .iter()
        .find(|s| s.id == view.spec)
        .map(|s| s.name.as_str())
        .unwrap_or("All specs");
    let slot = if view.slot == 0 {
        "All slots"
    } else {
        slot_name(view.slot)
    };
    [
        ("Class", class, "class"),
        ("Spec", spec, "spec"),
        ("Slot", slot, "slot"),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (name, title, filter))| {
        button(
            format!("Journal{name}Filter"),
            title,
            &format!("journal:filter:{filter}"),
            (202. + index as f32 * 190., 149., 184., 24.),
        )
    })
    .collect()
}
fn loot_filter_entries(view: &View, menu: LootFilter) -> Vec<(u32, String)> {
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
fn loot_filter_menu(ctx: &SharedContext, view: &View, menu: LootFilter) -> Element {
    let (name, key, index) = match menu {
        LootFilter::Class => ("Class", "class", 0),
        LootFilter::Spec => ("Spec", "spec", 1),
        LootFilter::Slot => ("Slot", "slot", 2),
    };
    let entries = loot_filter_entries(view, menu);
    let height = (entries.len() as f32 * 24.).min(300.);
    let children = entries
        .iter()
        .enumerate()
        .flat_map(|(row, (id, title))| {
            button(
                format!("Journal{name}{id}"),
                title,
                &format!("journal:{key}:{id}"),
                (0., row as f32 * 24., 174., 23.),
            )
        })
        .collect();
    let content = scroll_in(
        ctx,
        children,
        (0., 0., 188., height),
        entries.len() as f32 * 24.,
        "EncounterJournalFilterList",
    );
    let left = 202. + index as f32 * 190.;
    rsx! {r#frame {name:"EncounterJournalFilterMenu",left,top:175.0,width:190.0,height,pos_type:"absolute",frame_level:30.0,mouse_enabled:true,background_color:"0.08,0.07,0.06,1.0",{content}}}
}
fn sets(ctx: &SharedContext, view: &View) -> Element {
    let search = view.search.to_lowercase();
    let offset = ctx.scroll_first_row(LIST) as f32;
    let visible = |top: f32, height: f32| top + height >= offset && top <= offset + 416.;
    let mut y = 0.;
    let mut content = Vec::new();
    for set in view
        .catalog
        .sets
        .iter()
        .filter(|s| s.name.to_lowercase().contains(&search))
    {
        if visible(y, 24.) {
            content.extend(text(
                &format!("JournalSet{}", set.id),
                &set.name,
                (4., y, 730., 24.),
            ));
        }
        y += 26.;
        for item in set.items.iter().filter_map(|id| view.catalog.items.get(id)) {
            if visible(y, 20.) {
                content.extend(text(
                    &format!("JournalSet{}Item{}", set.id, item.id),
                    &item.name,
                    (20., y, 714., 20.),
                ));
            }
            y += 22.;
        }
        y += 10.;
    }
    scroll(ctx, content, (15., 65., 770., 416.), y.max(416.))
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
            button(
                format!("JournalSearchResult{index}"),
                title,
                action,
                (0., index as f32 * 26., 546., 24.),
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

fn tutorials(view: &View) -> Element {
    let title = view
        .catalog
        .text
        .get("RPE_TITLE")
        .map(String::as_str)
        .unwrap_or("");
    let body = view
        .catalog
        .text
        .get("RPE_TUTORIAL_TEXT")
        .map(String::as_str)
        .unwrap_or("");
    let mut out = rsx! {texture {name:"EncounterJournalTutorialArt",texture_atlas:"adventureguide-tutorial-rpe",left:34.0,top:76.0,width:270.0,height:390.0,pos_type:"absolute"}};
    out.extend(text(
        "EncounterJournalTutorialTitle",
        title,
        (330., 110., 380., 60.),
    ));
    out.extend(text(
        "EncounterJournalTutorials",
        body,
        (330., 190., 380., 210.),
    ));
    out.extend(text(
        "EncounterJournalTutorialUnavailable",
        "Catch Up Experience is not available on this realm.",
        (330., 420., 380., 44.),
    ));
    out
}
fn scroll(
    ctx: &SharedContext,
    children: Element,
    rect: (f32, f32, f32, f32),
    content_height: f32,
) -> Element {
    scroll_in(ctx, children, rect, content_height, LIST)
}
fn scroll_in(
    ctx: &SharedContext,
    children: Element,
    rect: (f32, f32, f32, f32),
    content_height: f32,
    list: &'static str,
) -> Element {
    let (left, top, width, height) = rect;
    let geometry = pixel_geometry(height, content_height, height);
    let offset = geometry.clamp(ctx.scroll_first_row(list));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let bar = MinimalScrollBar {
        list,
        left: width - 14.,
        top: 0.,
        height,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let child_width = width - 18.0;
    let list_name = DynName(list.into());
    let child_name = DynName(format!("{list}Child"));
    rsx! {r#frame {name:{list_name},left,top,width,height,pos_type:"absolute",mouse_enabled:true,scroll_list:config,r#frame {name:{child_name},left:0.0,top:child_top,width:child_width,height:content_height,pos_type:"absolute",{children}}{bar.element()}}}
}
