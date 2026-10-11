//! Retail Mainline Adventure Guide; both skins share the same 800×496 layout.
use crate::bank_art::{WHITE, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, portrait_border};
use game_engine_core::adventure_guide::{Catalog, Instance, Item, Section, difficulty_matches};
use std::{collections::BTreeSet, sync::Arc};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "EncounterJournalSearchBox";
pub const LIST: &str = "EncounterJournalContent";
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
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub visible: bool,
    pub viewport: [f32; 2],
    pub catalog: Arc<Catalog>,
    pub tab: Tab,
    pub page: Page,
    pub tier: u32,
    pub instance: Option<u32>,
    pub encounter: Option<u32>,
    pub difficulty: u32,
    pub role: u32,
    pub class: u32,
    pub spec: u32,
    pub slot: u32,
    pub search: String,
    pub collapsed: BTreeSet<u32>,
    pub lockouts: Vec<(u32, String)>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: [1280., 720.],
            catalog: Arc::new(Catalog::default()),
            tab: Tab::Dungeons,
            page: Page::Overview,
            tier: 73,
            instance: None,
            encounter: None,
            difficulty: 2,
            role: 0,
            class: 0,
            spec: 0,
            slot: 0,
            search: String::new(),
            collapsed: BTreeSet::new(),
            lockouts: Vec::new(),
        }
    }
}
impl View {
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
                if !self.collapsed.contains(&id) {
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
                (self.slot == 0 || i.slot == self.slot)
                    && (self.class == 0
                        || i.class_mask == -1
                        || i.class_mask & (1 << (self.class - 1)) != 0)
                    && (self.spec == 0
                        || self
                            .catalog
                            .item_specs
                            .get(&i.id)
                            .is_some_and(|specs| specs.contains(&self.spec)))
                    && i.name.to_lowercase().contains(&search)
            })
            .collect()
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
            "tier" => {
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
            "difficulty" => self.difficulty = number()?,
            "role" => self.role = number()?,
            "class" => {
                self.class = number()?;
                self.spec = 0;
            }
            "spec" => self.spec = number()?,
            "slot" => self.slot = number()?,
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
    body.extend(rsx!{editbox {name:"EncounterJournalSearchBox",left:580.0,top:32.0,width:210.0,height:20.0,font_size:12.0,font_color:WHITE,max_letters:80,text_insets:"4,4,0,0",pos_type:"absolute"}});
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
    body.extend(match view.tab {
        Tab::Tutorials => tutorials(),
        Tab::ItemSets => sets(ctx, view),
        _ => {
            if view.instance.is_some() {
                encounter_page(ctx, view)
            } else {
                instance_list(ctx, view)
            }
        }
    });
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
    controls.extend(text("EncounterJournalTier", title, (15., 65., 190., 22.)));
    let tiers: Vec<_> = view
        .catalog
        .tiers
        .iter()
        .filter(|t| t.id != view.tier)
        .collect();
    for (index, tier) in tiers.into_iter().enumerate() {
        controls.extend(button(
            format!("EncounterJournalTier{}", tier.id),
            &tier.name,
            &format!("journal:tier:{}", tier.id),
            (
                15. + (index % 4) as f32 * 192.,
                90. + (index / 4) as f32 * 24.,
                188.,
                22.,
            ),
        ));
    }
    let rows = view.instances();
    let content:Element=rows.iter().enumerate().flat_map(|(index,i)|{
 let x=(index%4) as f32*191.;let y=(index/4) as f32*130.;let mut art=texture(format!("JournalInstance{}Art",i.id),i.button,(0.,0.,185.,105.),WHITE);
 art.extend(text(&format!("JournalInstance{}Title",i.id),&i.name,(4.,106.,177.,22.)));
 let name=DynName(format!("JournalInstance{}",i.id));let action=format!("journal:instance:{}",i.id);
 rsx!{r#frame {name:{name},left:x,top:y,width:185.0,height:128.0,pos_type:"absolute",mouse_enabled:true,onclick:{action.as_str()},{art}}}
 }).collect();
    controls.extend(scroll(
        ctx,
        content,
        (15., 168., 770., 315.),
        (rows.len().div_ceil(4) as f32 * 130.).max(315.),
    ));
    controls
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
        .filter(|e| e.instance == instance.id)
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
        out.extend(text(
            "EncounterJournalInstanceLore",
            &instance.description,
            (205., 105., 550., 340.),
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
    for (index, (id, title)) in [
        (1, "Normal"),
        (2, "Heroic"),
        (23, "Mythic"),
        (14, "Raid Normal"),
        (15, "Raid Heroic"),
        (16, "Raid Mythic"),
    ]
    .into_iter()
    .filter(|(id, _)| {
        if instance.dungeon {
            *id < 14 || *id == 23
        } else {
            *id >= 14 && *id <= 16
        }
    })
    .enumerate()
    {
        out.extend(button(
            format!("JournalDifficulty{id}"),
            title,
            &format!("journal:difficulty:{id}"),
            (202. + index as f32 * 180., 123., 176., 22.),
        ));
    }
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
    out
}
fn loot_page(ctx: &SharedContext, view: &View) -> Element {
    let mut out = Vec::new();
    let mut filters = Vec::new();
    filters.extend(button(
        "JournalAllClasses".into(),
        "All classes",
        "journal:class:0",
        (0., 0., 92., 22.),
    ));
    for (index, class) in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
        .into_iter()
        .enumerate()
    {
        filters.extend(button(
            format!("JournalClass{class}"),
            &class.to_string(),
            &format!("journal:class:{class}"),
            (94. + index as f32 * 34., 0., 32., 22.),
        ));
    }
    filters.extend(button(
        "JournalAllSpecs".into(),
        "All specs",
        "journal:spec:0",
        (0., 25., 92., 22.),
    ));
    for (index, spec) in view
        .catalog
        .specs
        .iter()
        .filter(|s| s.class == view.class)
        .enumerate()
    {
        filters.extend(button(
            format!("JournalSpec{}", spec.id),
            &spec.name,
            &format!("journal:spec:{}", spec.id),
            (94. + index as f32 * 106., 25., 104., 22.),
        ));
    }
    filters.extend(button(
        "JournalAllSlots".into(),
        "All slots",
        "journal:slot:0",
        (0., 50., 92., 22.),
    ));
    for (index, (id, name)) in [
        (1, "Head"),
        (3, "Shoulder"),
        (5, "Chest"),
        (7, "Legs"),
        (10, "Hands"),
        (11, "Ring"),
        (12, "Trinket"),
        (21, "Weapon"),
    ]
    .into_iter()
    .enumerate()
    {
        filters.extend(button(
            format!("JournalSlot{id}"),
            name,
            &format!("journal:slot:{id}"),
            (94. + index as f32 * 57., 50., 55., 22.),
        ));
    }
    out.extend(rsx!{r#frame {name:"EncounterJournalLootFilters",left:202.0,top:149.0,width:575.0,height:74.0,pos_type:"absolute",{filters}}});
    let items = view.loot();
    let content = items
        .iter()
        .enumerate()
        .flat_map(|(index, i)| {
            let y = index as f32 * 44.;
            let mut row = texture(
                format!("JournalLoot{}Icon", i.id),
                i.icon,
                (2., y + 2., 36., 36.),
                WHITE,
            );
            row.extend(text(
                &format!("JournalLoot{}Name", i.id),
                &i.name,
                (46., y + 2., 475., 22.),
            ));
            row.extend(text(
                &format!("JournalLoot{}Slot", i.id),
                &format!("Item level {} · Slot {}", i.level, i.slot),
                (46., y + 24., 475., 16.),
            ));
            row
        })
        .collect();
    out.extend(scroll(
        ctx,
        content,
        (202., 230., 575., 249.),
        (items.len() as f32 * 44.).max(249.),
    ));
    out
}
fn sets(ctx: &SharedContext, view: &View) -> Element {
    let search = view.search.to_lowercase();
    let mut y = 0.;
    let mut content = Vec::new();
    for set in view
        .catalog
        .sets
        .iter()
        .filter(|s| s.name.to_lowercase().contains(&search))
    {
        content.extend(text(
            &format!("JournalSet{}", set.id),
            &set.name,
            (4., y, 730., 24.),
        ));
        y += 26.;
        for item in set.items.iter().filter_map(|id| view.catalog.items.get(id)) {
            content.extend(text(
                &format!("JournalSet{}Item{}", set.id, item.id),
                &item.name,
                (20., y, 714., 20.),
            ));
            y += 22.;
        }
        y += 10.;
    }
    scroll(ctx, content, (15., 65., 770., 416.), y.max(416.))
}
fn tutorials() -> Element {
    text(
        "EncounterJournalTutorials",
        "Adventure Guide\n\nSelect an expansion, then Dungeons or Raids. Select an instance to read its story and choose a boss.\n\nOverview summarizes the encounter. Abilities lists expandable spell sections. Choose your role and difficulty.\n\nLoot lists authored boss drops. Filter by class, specialization or equipment slot. Model displays the boss in 3D.\n\nJourneys and Traveler's Log require server-backed activity state; unavailable on this realm.",
        (24., 76., 745., 370.),
    )
}
fn scroll(
    ctx: &SharedContext,
    children: Element,
    rect: (f32, f32, f32, f32),
    content_height: f32,
) -> Element {
    let (left, top, width, height) = rect;
    let geometry = pixel_geometry(height, content_height, height);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let bar = MinimalScrollBar {
        list: LIST,
        left: width - 14.,
        top: 0.,
        height,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let child_width = width - 18.0;
    rsx! {r#frame {name:"EncounterJournalContent",left,top,width,height,pos_type:"absolute",mouse_enabled:true,scroll_list:config,r#frame {name:"EncounterJournalContentChild",left:0.0,top:child_top,width:child_width,height:content_height,pos_type:"absolute",{children}}{bar.element()}}}
}
