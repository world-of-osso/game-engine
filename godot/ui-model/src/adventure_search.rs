//! Retail EncounterSearchSM/LG and SearchBoxListMixin. Local title matching and
//! instance → encounter → section catalog order are project policy, not EJ C API ranking.
use crate::{
    adventure_guide::{Page, Tab, View, scroll_in},
    bank_art::{cropped, label, texture},
    quest_art::{DynName, named_atlas_texture},
    selection_list::{self, Art, RowArt, RowTemplate},
};
use game_engine_core::adventure_guide::{Encounter, Instance, Section, difficulty_matches};
use std::collections::BTreeSet;
const GENERIC_CREATURE: u32 = 533888;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchMode {
    #[default]
    Hidden,
    Preview,
    Full,
}
#[derive(Clone, Copy)]
enum Target {
    Instance(u32),
    Encounter(u32),
    Section(u32),
}
struct ResultRow<'a> {
    target: Target,
    title: &'a str,
    icon: u32,
    kind: &'static str,
    path: String,
}
impl View {
    pub fn clear_search(&mut self) {
        self.search.clear();
        self.search_mode = SearchMode::Hidden;
        self.search_selected = 0;
        self.search_section = None;
    }
    pub fn update_search(&mut self, query: String) {
        self.search = query;
        self.search_selected = 0;
        self.search_section = None;
        // OnTextChanged refreshes the active result presentation; synchronous local
        // matching has no artificial progress state or delay.
    }
    fn search_rows(&self) -> Vec<ResultRow<'_>> {
        if self.search.len() < 3 {
            return vec![];
        }
        let query = self.search.to_lowercase();
        let instances = self
            .catalog
            .instances
            .iter()
            .filter(|row| row.name.to_lowercase().contains(&query))
            .map(instance_result);
        let encounters = self
            .catalog
            .encounters
            .iter()
            .filter(|row| row.name.to_lowercase().contains(&query))
            .map(|row| self.encounter_result(row));
        let sections = self
            .catalog
            .sections
            .values()
            .filter(|row| row.title.to_lowercase().contains(&query))
            .filter_map(|row| self.section_result(row));
        instances.chain(encounters).chain(sections).collect()
    }
    fn encounter_result<'a>(&self, encounter: &'a Encounter) -> ResultRow<'a> {
        let path = self
            .catalog
            .instance(encounter.instance)
            .map(|row| row.name.clone())
            .unwrap_or_default();
        ResultRow {
            target: Target::Encounter(encounter.id),
            title: &encounter.name,
            icon: GENERIC_CREATURE,
            kind: "ENCOUNTER_JOURNAL_ENCOUNTER",
            path,
        }
    }
    fn section_result<'a>(&self, section: &'a Section) -> Option<ResultRow<'a>> {
        let encounter = self.catalog.encounter(section.encounter)?;
        let instance = self.catalog.instance(encounter.instance)?;
        let creature = section.creature_display > 0;
        Some(ResultRow {
            target: Target::Section(section.id),
            title: &section.title,
            icon: if creature {
                GENERIC_CREATURE
            } else {
                section.icon
            },
            kind: if creature {
                "ENCOUNTER_JOURNAL_ENCOUNTER_ADD"
            } else {
                "ENCOUNTER_JOURNAL_ABILITY"
            },
            path: format!("{} > {}", instance.name, encounter.name),
        })
    }
    pub(crate) fn move_search_preview(&mut self, step: isize) {
        let count = self.search_rows().len().min(6);
        if self.search_mode == SearchMode::Preview && count > 0 {
            self.search_selected =
                (self.search_selected as isize + step).rem_euclid(count as isize) as usize;
        }
    }
    pub(crate) fn show_full_search(&mut self) {
        self.search_mode = if self.search_rows().is_empty() {
            SearchMode::Hidden
        } else {
            SearchMode::Full
        };
    }
    pub(crate) fn submit_search(&mut self) -> Result<(), String> {
        if self.search_mode != SearchMode::Preview || self.search_rows().is_empty() {
            return Ok(());
        }
        if self.search_selected == 5 {
            self.show_full_search();
            Ok(())
        } else {
            self.select_search(self.search_selected)
        }
    }
    pub(crate) fn select_search(&mut self, index: usize) -> Result<(), String> {
        let target = self
            .search_rows()
            .get(index)
            .map(|row| row.target)
            .ok_or("Unknown journal search result")?;
        match target {
            Target::Instance(id) => {
                self.apply(&format!("journal:instance:{id}"))?;
                let instance = self.catalog.instance(id).ok_or("Search instance missing")?;
                self.tab = if instance.dungeon {
                    Tab::Dungeons
                } else {
                    Tab::Raids
                };
                self.tier = *instance.tiers.first().ok_or("Instance tier data missing")?;
            }
            Target::Encounter(id) => self.select_encounter(id)?,
            Target::Section(id) => self.select_search_section(id)?,
        }
        self.search_mode = SearchMode::Hidden;
        Ok(())
    }
    fn select_search_section(&mut self, id: u32) -> Result<(), String> {
        let section = self
            .catalog
            .sections
            .get(&id)
            .ok_or("Search section missing")?;
        let encounter = section.encounter;
        let (ancestors, overview) = self.section_search_path(id)?;
        self.select_encounter(encounter)?;
        self.select_section_difficulty(encounter, &ancestors)?;
        self.page = if overview || !self.instance_has_overview() {
            Page::Overview
        } else {
            Page::Abilities
        };
        for ancestor in ancestors {
            self.collapsed.remove(&ancestor);
        }
        if !self.sections()?.iter().any(|(section, _)| section.id == id) {
            return Err(format!(
                "Search section {id} is not reachable on the selected page"
            ));
        }
        self.search_section = Some(id);
        Ok(())
    }
    fn section_search_path(&self, mut current: u32) -> Result<(BTreeSet<u32>, bool), String> {
        let mut ancestors = BTreeSet::new();
        let mut overview = false;
        while current != 0 {
            if !ancestors.insert(current) {
                return Err(format!("Journal section parent cycle at {current}"));
            }
            let section = self
                .catalog
                .sections
                .get(&current)
                .ok_or("Section ancestor missing")?;
            overview |= section.kind == 3;
            current = section.parent;
        }
        Ok((ancestors, overview))
    }
    fn select_section_difficulty(
        &mut self,
        encounter: u32,
        ancestors: &BTreeSet<u32>,
    ) -> Result<(), String> {
        // Local results lack EJ_GetSearchResult's difficulty field. Retain the current
        // authored difficulty when possible, otherwise use the first supported one.
        let instance = self.instance.ok_or("Search encounter instance missing")?;
        let supports = |difficulty| {
            ancestors
                .iter()
                .all(|id| difficulty_matches(&self.catalog.section_difficulties, *id, difficulty))
        };
        if !supports(self.difficulty) {
            self.difficulty = self
                .available_difficulties(instance, Some(encounter))
                .into_iter()
                .find(|difficulty| supports(*difficulty))
                .ok_or("Search section has no supported difficulty")?;
        }
        Ok(())
    }
}
fn instance_result(instance: &Instance) -> ResultRow<'_> {
    ResultRow {
        target: Target::Instance(instance.id),
        title: &instance.name,
        icon: instance.button,
        kind: "ENCOUNTER_JOURNAL_INSTANCE",
        path: String::new(),
    }
}

const SMALL: RowTemplate = RowTemplate {
    normal: None,
    hover: None,
    selected: None,
};
const LARGE: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        art: Art::File(522972, [0.00195313, 0.50195313, 0.80468750, 0.85058594]),
        insets: [0.; 4],
        alpha: 1.,
        additive: true,
    }),
    selected: None,
};
const NAME: &str = "0.96875,0.8984375,0.578125,1";
const PATH: &str = "0.66796875,0.51171875,0.3359375,1";

pub(crate) fn search_results(ctx: &SharedContext, view: &View) -> Element {
    let rows = view.search_rows();
    if rows.is_empty() {
        return vec![];
    }
    match view.search_mode {
        SearchMode::Hidden => vec![],
        SearchMode::Preview => preview(view, &rows),
        SearchMode::Full => full_results(ctx, view, &rows),
    }
}
fn row_background(name: &str, width: f32, height: f32, small: bool) -> Element {
    let coords = if small {
        "0,1,0.16601563,0.21875"
    } else {
        "0,1,0.74804688,0.84375"
    };
    rsx! {texture {name:{DynName(format!("{name}Background"))},texture_fdid:522973,tex_coords:coords,left:0.0,top:0.0,width,height,horiz_tile:true,pos_type:"absolute",draw_layer:"BACKGROUND"}}
}
fn selected(name: &str, width: f32, height: f32, chosen: bool) -> Element {
    let art = named_atlas_texture(
        format!("{name}SelectedArt"),
        "search-highlight",
        (0., 0., width, height),
    );
    let hidden = !chosen;
    rsx! {r#frame {name:{DynName(format!("{name}Selected"))},left:0.0,top:0.0,width,height,pos_type:"absolute",hidden,{art}}}
}
fn result_icon(name: &str, row: &ResultRow<'_>, small: bool, height: f32) -> Element {
    // EncounterSearchSM overrides the inherited38px frame to21; LG retains38.
    let (left, frame_size) = if small { (5., 21.) } else { (10., 38.) };
    let top = (height - frame_size) / 2. - if small { 1. } else { 0. };
    let mut out = cropped(
        format!("{name}IconFrame"),
        522972,
        "0.89843750,0.97265625,0.21386719,0.25097656",
        (left, top, frame_size, frame_size),
    );
    let rect = (left + 1., top + 2., frame_size - 2., frame_size - 3.);
    out.extend(if matches!(row.target, Target::Instance(_)) && !small {
        cropped(
            format!("{name}Icon"),
            row.icon,
            "0.16796875,0.51171875,0.03125,0.71875",
            rect,
        )
    } else {
        texture(format!("{name}Icon"), row.icon, rect, "1,1,1,1")
    });
    out
}
fn preview(view: &View, rows: &[ResultRow<'_>]) -> Element {
    let mut children = Vec::new();
    let mut top = 0.;
    for (index, row) in rows.iter().take(5).enumerate() {
        let name = format!("JournalSearchPreview{index}");
        let height = if index == 0 { 24. } else { 27. };
        let mut content = row_background(&name, 210., height, true);
        content.extend(result_icon(&name, row, true, height));
        content.extend(label(
            format!("{name}Name"),
            row.title,
            (30., (height - 10.) / 2., 175., 10.),
            (10., NAME, "LEFT"),
        ));
        content.extend(selected(&name, 210., height, view.search_selected == index));
        children.extend(selection_list::row(
            name,
            &format!("journal:search-select:{index}"),
            true,
            (0., top, 210., height),
            (&SMALL, false),
            content,
        ));
        top += height;
    }
    if rows.len() > 5 {
        let title = crate::auction_mail::text("SEARCH_RESULTS_SHOW_COUNT")
            .replace("%d", &rows.len().to_string());
        let mut content = row_background("JournalSearchAll", 210., 24., true);
        content.extend(label(
            "JournalSearchAllName".into(),
            &title,
            (0., 7., 210., 10.),
            (12., NAME, "CENTER"),
        ));
        content.extend(selected(
            "JournalSearchAll",
            210.,
            24.,
            view.search_selected == 5,
        ));
        children.extend(selection_list::row(
            "JournalSearchAll".into(),
            "journal:search-full",
            true,
            (0., top, 210., 24.),
            (&SMALL, false),
            content,
        ));
        top += 24.;
    }
    children.extend(preview_border(top));
    rsx! {r#frame {name:"EncounterJournalSearchPreview",left:580.0,top:52.0,width:210.0,height:top,pos_type:"absolute",mouse_enabled:true,frame_level:25.0,{children}}}
}
fn full_results(ctx: &SharedContext, view: &View, rows: &[ResultRow<'_>]) -> Element {
    let mut content = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let name = format!("JournalSearchResult{index}");
        let mut text = row_background(&name, 575., 49., false);
        text.extend(result_icon(&name, row, false, 49.));
        text.extend(label(
            format!("{name}Name"),
            row.title,
            (58., 5.5, 460., 12.),
            (16., NAME, "LEFT"),
        ));
        text.extend(label(
            format!("{name}Path"),
            &row.path,
            (58., 24.5, 400., 12.),
            (12., PATH, "LEFT"),
        ));
        text.extend(label(
            format!("{name}Type"),
            crate::auction_mail::text(row.kind),
            (421., 18.5, 140., 12.),
            (12., PATH, "RIGHT"),
        ));
        content.extend(selection_list::row(
            name,
            &format!("journal:search-select:{index}"),
            true,
            (0., index as f32 * 49., 575., 49.),
            (&LARGE, false),
            text,
        ));
    }
    let height = (rows.len() as f32 * 49.).max(371.);
    let list = scroll_in(
        ctx,
        content,
        (1., 28., 579., 371.),
        height,
        "EncounterJournalSearchList",
    );
    let title = crate::auction_mail::text("ENCOUNTER_JOURNAL_SEARCH_RESULTS")
        .replacen("%s", &view.search, 1)
        .replacen("%d", &rows.len().to_string(), 1);
    let mut art = popup_art();
    art.extend(label(
        "EncounterJournalSearchResultsTitle".into(),
        &title,
        (60., 7., 480., 12.),
        (12., NAME, "CENTER"),
    ));
    let close = named_atlas_texture(
        "JournalSearchCloseArt".into(),
        "RedButton-Exit",
        (0., 0., 24., 24.),
    );
    art.extend(rsx! {button {name:"EncounterJournalSearchResultsClose",left:583.0,top:-2.0,width:24.0,height:24.0,pos_type:"absolute",button_default_skin:false,onclick:"journal:search-close",{close}}});
    rsx! {r#frame {name:"EncounterJournalSearchResults",left:100.0,top:89.0,width:600.0,height:400.0,pos_type:"absolute",mouse_enabled:true,frame_level:25.0,{art} {list}}}
}
fn atlas_size(name: &str) -> (f32, f32) {
    let region = ui_toolkit::atlas::resolve_region(name, ui_toolkit::atlas::thread_skin())
        .expect("Retail search template atlas");
    (region.width, region.height)
}
fn atlas_region(name: &str, atlas: &str, rect: (f32, f32, f32, f32)) -> Element {
    let region = ui_toolkit::atlas::resolve_region(atlas, ui_toolkit::atlas::thread_skin())
        .expect("Retail search template atlas");
    let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Search atlas must have a FileDataID")
    };
    let coords = format!(
        "{},{},{},{}",
        region.left, region.right, region.top, region.bottom
    );
    let (left, top, width, height) = rect;
    rsx! {texture {name:{DynName(name.into())},texture_fdid:fdid,tex_coords:{coords.as_str()},left,top,width,height,pos_type:"absolute",horiz_tile:{region.tiles_horizontally},vert_tile:{region.tiles_vertically}}}
}
fn preview_border(height: f32) -> Element {
    let left = atlas_size("UI-Frame-BotCornerLeft");
    let right = atlas_size("UI-Frame-BotCornerRight");
    let bottom = height + 8.;
    let mut art = Vec::new();
    for (suffix, atlas, rect) in [
        (
            "BottomLeft",
            "UI-Frame-BotCornerLeft",
            (-7., bottom - left.1, left.0, left.1),
        ),
        (
            "BottomRight",
            "UI-Frame-BotCornerRight",
            (214. - right.0, bottom - right.1, right.0, right.1),
        ),
        (
            "Bottom",
            "_UI-Frame-Bot",
            (
                left.0 - 7.,
                bottom - atlas_size("_UI-Frame-Bot").1,
                221. - right.0 - left.0,
                atlas_size("_UI-Frame-Bot").1,
            ),
        ),
        (
            "Left",
            "!UI-Frame-LeftTile",
            (-7., -1., 16., bottom - left.1 + 1.),
        ),
        (
            "Right",
            "!UI-Frame-RightTile",
            (205., -1., 10., bottom - right.1 + 1.),
        ),
        ("Top", "_UI-Frame-Bot", (-2., -3., 214., 6.)),
    ] {
        art.extend(atlas_region(
            &format!("JournalSearchPreviewBorder{suffix}"),
            atlas,
            rect,
        ));
    }
    art
}
fn popup_art() -> Element {
    // BottomPopupScrollBoxTemplate: rock background, two rows of top corners and
    // tiled side borders. No invented bottom cap; the source omits it too.
    let mut art = rsx! {texture {name:"JournalSearchPopupBackground",texture_fdid:374155,left:0.0,top:0.0,width:600.0,height:400.0,pos_type:"absolute",horiz_tile:true,vert_tile:true,vertex_color:"0.9,0.8,0.7,1"}};
    let left = atlas_size("UI-Frame-TopCornerLeft");
    let right = atlas_size("UI-Frame-TopCornerRightSimple");
    for (suffix, top) in [("Top", -4.), ("Second", 20.)] {
        art.extend(atlas_region(
            &format!("JournalSearchPopup{suffix}Left"),
            "UI-Frame-TopCornerLeft",
            (-7., top, left.0, left.1),
        ));
        art.extend(atlas_region(
            &format!("JournalSearchPopup{suffix}Right"),
            "UI-Frame-TopCornerRightSimple",
            (604. - right.0, top, right.0, right.1),
        ));
        art.extend(atlas_region(
            &format!("JournalSearchPopup{suffix}Border"),
            "_UI-Frame-Top",
            (
                left.0 - 7.,
                top,
                611. - right.0 - left.0,
                atlas_size("_UI-Frame-Top").1,
            ),
        ));
    }
    art.extend(atlas_region(
        "JournalSearchPopupLeft",
        "!UI-Frame-LeftTile",
        (-7., left.1 - 4., 16., 404. - left.1),
    ));
    art.extend(atlas_region(
        "JournalSearchPopupRight",
        "!UI-Frame-RightTile",
        (595., right.1 - 4., 10., 404. - right.1),
    ));
    art.extend(atlas_region(
        "JournalSearchPopupStreaks",
        "_UI-Frame-TopTileStreaks",
        (0., 0., 600., atlas_size("_UI-Frame-TopTileStreaks").1),
    ));
    art
}
