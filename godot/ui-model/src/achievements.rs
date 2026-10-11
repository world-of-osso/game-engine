//! On-demand AchievementFrame: ordered category tree, cached cursor pages, live refresh.
//! Retail AchievementUI.lua:572 (categories), :1814 (rows), :1952 (criteria).
use crate::quest_art::{DynName, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::{
    AchievementCatalogEntry, AchievementCatalogPage, AchievementCategoryEntry,
    AchievementCriterionLine, AchievementScope, AchievementStateUpdate, AchievementSummary,
    QueryAchievementCatalog,
};
use std::collections::BTreeMap;
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const OPEN_ACTION: &str = "micro:AchievementMicroButton";
pub const FRAME: &str = "AchievementFrame";
/// Local Mainline UI atlas1771: alert plate, icon frame and point shield.
pub const TOAST_ATLAS_FDID: u32 = 4_035_524;
const ROWS: usize = 3;
const STATISTIC_ROWS: usize = 11;
const STATISTIC_ROW_HEIGHT: f32 = 24.0;
const CATEGORIES: usize = 17;
const CRITERIA: usize = 6;
/// Local Retail art: UI-Achievement-Category-Background, AchievementBackground, Shield.
pub const ART: &[u32] = &[
    130652,
    235397,
    130665,
    TOAST_ATLAS_FDID,
    137012,
    130656,
    HEADER_SHIELD_FDID,
];
/// Retail Mainline/Blizzard_AchievementUI.xml:1979-1984, UI-Achievement-TinyShield.
const HEADER_SHIELD_FDID: u32 = 235_415;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CategoryPage {
    pub entries: Vec<AchievementCatalogEntry>,
    pub next: Option<u32>,
    pub loaded: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AchievementWindow {
    pub visible: bool,
    pub categories: BTreeMap<u32, AchievementCategoryEntry>,
    pub pages: BTreeMap<u32, CategoryPage>,
    pub selected_category: Option<u32>,
    pub selected_achievement: Option<u32>,
    pub row_offset: usize,
    pub category_offset: usize,
    pub criterion_offset: usize,
    pub scope: AchievementScope,
    pub summary: Option<AchievementSummary>,
    pub detail: Option<AchievementCatalogEntry>,
    pub tracked: Vec<AchievementCatalogEntry>,
    pub tracked_ids: Vec<u32>,
    pub tracking_error: Option<String>,
    pub statistics: bool,
    pub search_text: String,
    pub search_results: CategoryPage,
    categories_loaded: bool,
    pending: Vec<QueryAchievementCatalog>,
}
impl AchievementWindow {
    pub fn action(&mut self, action: &str) -> Vec<QueryAchievementCatalog> {
        let mut requests = Vec::new();
        match action {
            OPEN_ACTION => self.toggle(&mut requests),
            "achievement:close" => self.visible = false,
            "achievement:statistics" => self.change_statistics(true),
            "achievement:achievements" => self.change_statistics(false),
            "achievement:summary" => {
                self.selected_category = None;
                self.selected_achievement = None;
                self.search_text.clear();
                self.statistics = false;
                self.criterion_offset = 0;
                self.row_offset = 0;
                if self.summary.is_none() {
                    requests.push(QueryAchievementCatalog::Summary { scope: self.scope });
                }
            }
            "achievement:scope:all" => self.change_scope(AchievementScope::All, &mut requests),
            "achievement:scope:account" => {
                self.change_scope(AchievementScope::Account, &mut requests)
            }
            "achievement:scope:character" => {
                self.change_scope(AchievementScope::Character, &mut requests)
            }
            "achievement:categories_next" => {
                self.category_offset =
                    (self.category_offset + CATEGORIES).min(self.categories.len().saturating_sub(1))
            }
            "achievement:categories_prev" => {
                self.category_offset = self.category_offset.saturating_sub(CATEGORIES)
            }
            "achievement:rows_prev" => {
                self.row_offset = self.row_offset.saturating_sub(self.visible_row_count());
                self.select_first();
            }
            "achievement:more_rows" => self.advance_rows(&mut requests),
            "achievement:criteria_prev" => {
                self.criterion_offset = self.criterion_offset.saturating_sub(CRITERIA)
            }
            "achievement:more_criteria" => self.advance_criteria(&mut requests),
            _ => self.select_action(action, &mut requests),
        }
        self.enqueue(requests)
    }
    fn change_statistics(&mut self, statistics: bool) {
        if self.statistics == statistics {
            return;
        }
        self.statistics = statistics;
        self.category_offset = 0;
        self.pages.clear();
        self.selected_category = None;
        self.selected_achievement = None;
        self.detail = None;
        self.search_text.clear();
        self.row_offset = 0;
    }

    fn category_request(&self, category_id: u32, after_id: u32) -> QueryAchievementCatalog {
        if self.statistics {
            QueryAchievementCatalog::Statistics {
                category_id,
                after_id,
            }
        } else {
            QueryAchievementCatalog::Category {
                category_id,
                after_id,
            }
        }
    }

    fn search(&mut self, text: &str, requests: &mut Vec<QueryAchievementCatalog>) {
        self.search_text = text.chars().take(256).collect();
        self.search_results = CategoryPage::default();
        self.selected_category = None;
        self.selected_achievement = None;
        self.detail = None;
        self.row_offset = 0;
        if self.search_text.trim().chars().count() >= 3 {
            requests.push(QueryAchievementCatalog::Search {
                text: self.search_text.trim().into(),
                scope: self.scope,
                after_id: 0,
            });
        }
    }

    fn change_scope(
        &mut self,
        scope: AchievementScope,
        requests: &mut Vec<QueryAchievementCatalog>,
    ) {
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        self.summary = None;
        self.row_offset = 0;
        self.selected_achievement = None;
        self.select_first();
        requests.push(QueryAchievementCatalog::Summary { scope });
        if self.search_text.trim().chars().count() >= 3 {
            self.search_results = CategoryPage::default();
            requests.push(QueryAchievementCatalog::Search {
                text: self.search_text.trim().into(),
                scope,
                after_id: 0,
            });
        }
    }

    fn toggle(&mut self, requests: &mut Vec<QueryAchievementCatalog>) {
        self.visible = !self.visible;
        if !self.visible {
            return;
        }
        if !self.categories_loaded {
            requests.push(QueryAchievementCatalog::Categories { after_id: 0 });
        }
        if self.summary.is_none() {
            requests.push(QueryAchievementCatalog::Summary { scope: self.scope });
        }
        if let Some(category_id) = self.selected_category {
            if !self.pages.get(&category_id).is_some_and(|page| page.loaded) {
                requests.push(self.category_request(category_id, 0));
            }
        }
    }
    fn select_action(&mut self, action: &str, requests: &mut Vec<QueryAchievementCatalog>) {
        if let Some(text) = action.strip_prefix("achievement:search:") {
            self.search(text, requests);
        } else if let Some(id) = action
            .strip_prefix("achievement:restore:")
            .and_then(|id| id.parse().ok())
        {
            if self.tracked_ids.len() < 10 && !self.tracked_ids.contains(&id) {
                self.tracked_ids.push(id);
                requests.push(QueryAchievementCatalog::Achievement { achievement_id: id });
            }
        } else if let Some(id) = action
            .strip_prefix("achievement:track:")
            .and_then(|id| id.parse().ok())
        {
            self.toggle_tracking(id);
            if let Some(entry) = self.tracked.iter().find(|entry| entry.achievement_id == id) {
                if let Some(after_id) = entry.next_criteria_id {
                    requests.push(QueryAchievementCatalog::Criteria {
                        achievement_id: id,
                        after_id,
                    });
                }
            }
        } else if let Some(id) = action
            .strip_prefix("achievement:open:")
            .and_then(|id| id.parse().ok())
        {
            self.open_achievement(id, requests);
        } else if let Some(id) = action
            .strip_prefix("achievement:category:")
            .and_then(|id| id.parse().ok())
        {
            if !self.categories.contains_key(&id) {
                return;
            }
            self.selected_category = Some(id);
            self.search_text.clear();
            self.detail = None;
            self.selected_achievement = None;
            self.row_offset = 0;
            self.criterion_offset = 0;
            if !self.pages.get(&id).is_some_and(|page| page.loaded) {
                requests.push(self.category_request(id, 0));
            } else {
                self.select_first();
            }
        } else if let Some(id) = action
            .strip_prefix("achievement:row:")
            .and_then(|id| id.parse().ok())
        {
            if self
                .displayed_entries()
                .iter()
                .any(|entry| entry.achievement_id == id)
            {
                if self.search_text.trim().chars().count() >= 3 {
                    self.open_achievement(id, requests);
                } else {
                    self.selected_achievement = Some(id);
                    self.criterion_offset = 0;
                }
            }
        }
    }
    /// Retail AchievementUI.lua:1616-1643: manual tracking, ten-entry cap, no earned rows.
    fn toggle_tracking(&mut self, id: u32) {
        self.tracking_error = None;
        if self.tracked_ids.contains(&id) {
            self.tracked_ids.retain(|tracked| *tracked != id);
            self.tracked.retain(|entry| entry.achievement_id != id);
            return;
        }
        let Some(entry) = self
            .entries()
            .iter()
            .find(|entry| entry.achievement_id == id)
        else {
            return;
        };
        if entry.earned || entry.flags & 1 != 0 {
            self.tracking_error = Some("Completed achievements cannot be tracked".into());
        } else if self.tracked_ids.len() == 10 {
            self.tracking_error = Some("You can track at most 10 achievements".into());
        } else {
            self.tracked.push(entry.clone());
            self.tracked_ids.push(id);
        }
    }

    fn open_achievement(&mut self, id: u32, requests: &mut Vec<QueryAchievementCatalog>) {
        self.visible = true;
        self.statistics = false;
        self.category_offset = 0;
        self.selected_category = None;
        self.selected_achievement = Some(id);
        self.detail = None;
        self.search_text.clear();
        self.row_offset = 0;
        self.criterion_offset = 0;
        if self.scope != AchievementScope::All {
            self.scope = AchievementScope::All;
            self.summary = None;
        }
        if !self.categories_loaded {
            requests.push(QueryAchievementCatalog::Categories { after_id: 0 });
        }
        if self.summary.is_none() {
            requests.push(QueryAchievementCatalog::Summary { scope: self.scope });
        }
        requests.push(QueryAchievementCatalog::Achievement { achievement_id: id });
    }

    fn visible_row_count(&self) -> usize {
        if self.statistics && self.search_text.trim().chars().count() < 3 {
            STATISTIC_ROWS
        } else {
            ROWS
        }
    }

    fn advance_rows(&mut self, requests: &mut Vec<QueryAchievementCatalog>) {
        if self.search_text.trim().chars().count() >= 3 {
            if self.row_offset + ROWS < self.displayed_entries().len() {
                self.row_offset += ROWS;
                self.select_first();
            } else if let Some(after_id) = self.search_results.next {
                requests.push(QueryAchievementCatalog::Search {
                    text: self.search_text.trim().into(),
                    scope: self.scope,
                    after_id,
                });
            }
            return;
        }
        let Some(category_id) = self.selected_category else {
            return;
        };
        let Some(page) = self.pages.get(&category_id) else {
            return;
        };
        if self.row_offset + self.visible_row_count() < self.displayed_entries().len() {
            self.row_offset += self.visible_row_count();
            self.select_first();
        } else if let Some(after_id) = page.next {
            requests.push(self.category_request(category_id, after_id));
        }
    }
    fn advance_criteria(&mut self, requests: &mut Vec<QueryAchievementCatalog>) {
        let Some(entry) = self.selected() else {
            return;
        };
        if self.criterion_offset + CRITERIA < entry.criteria.len() {
            self.criterion_offset += CRITERIA;
        } else if let Some(after_id) = entry.next_criteria_id {
            requests.push(QueryAchievementCatalog::Criteria {
                achievement_id: entry.achievement_id,
                after_id,
            });
        }
    }
    fn enqueue(&mut self, requests: Vec<QueryAchievementCatalog>) -> Vec<QueryAchievementCatalog> {
        requests
            .into_iter()
            .filter(|request| {
                if self.pending.contains(request) {
                    return false;
                }
                self.pending.push(request.clone());
                true
            })
            .collect()
    }
    pub fn request_failed(&mut self, request: QueryAchievementCatalog) {
        self.pending.retain(|pending| *pending != request);
    }
    pub fn apply(&mut self, page: AchievementCatalogPage) -> Vec<QueryAchievementCatalog> {
        let pending = self
            .pending
            .iter()
            .position(|request| reply_matches(request, &page))
            .map(|index| self.pending.remove(index));
        let requests = match page {
            AchievementCatalogPage::Search {
                text,
                scope,
                achievements,
                next_id,
            } => {
                if text == self.search_text.trim() && scope == self.scope {
                    if matches!(
                        pending,
                        Some(QueryAchievementCatalog::Search { after_id: 0, .. })
                    ) {
                        self.search_results.entries.clear();
                    }
                    for entry in achievements {
                        update_achievement(&mut self.search_results.entries, entry);
                    }
                    self.search_results.next = next_id;
                    self.search_results.loaded = true;
                    self.select_first();
                }
                Vec::new()
            }
            AchievementCatalogPage::Statistics {
                category_id,
                achievements,
                next_id,
            } => {
                if self.statistics {
                    let first = matches!(
                        pending,
                        Some(QueryAchievementCatalog::Statistics { after_id: 0, .. })
                    );
                    self.cache_achievements(category_id, achievements, next_id, first);
                }
                Vec::new()
            }
            AchievementCatalogPage::Achievement {
                achievement_id,
                entry,
            } => {
                if self.tracked_ids.contains(&achievement_id) {
                    if let Some(row) = entry
                        .as_ref()
                        .filter(|row| !row.earned && row.flags & 1 == 0)
                    {
                        if let Some(tracked) = self
                            .tracked
                            .iter_mut()
                            .find(|row| row.achievement_id == achievement_id)
                        {
                            *tracked = row.clone();
                        } else {
                            self.tracked.push(row.clone());
                        }
                    } else {
                        self.tracked_ids.retain(|id| *id != achievement_id);
                        self.tracked
                            .retain(|row| row.achievement_id != achievement_id);
                    }
                }
                if self.selected_category.is_none()
                    && self.selected_achievement == Some(achievement_id)
                {
                    self.statistics = entry.as_ref().is_some_and(|row| row.flags & 1 != 0);
                    self.detail = entry;
                }
                self.tracked_criteria_request(achievement_id)
                    .into_iter()
                    .collect()
            }
            AchievementCatalogPage::Summary(summary) => {
                if summary.scope == self.scope {
                    self.summary = Some(summary);
                }
                Vec::new()
            }
            AchievementCatalogPage::Categories {
                categories,
                next_id,
            } => self.cache_categories(categories, next_id),
            AchievementCatalogPage::Category {
                category_id,
                achievements,
                next_id,
            } => {
                let first = matches!(
                    pending,
                    Some(QueryAchievementCatalog::Category { after_id: 0, .. })
                );
                if !self.statistics {
                    self.cache_achievements(category_id, achievements, next_id, first);
                }
                Vec::new()
            }
            AchievementCatalogPage::Criteria {
                achievement_id,
                criteria,
                next_id,
            } => {
                self.cache_criteria(achievement_id, criteria, next_id);
                self.tracked_criteria_request(achievement_id)
                    .into_iter()
                    .collect()
            }
        };
        self.enqueue(requests)
    }
    fn tracked_criteria_request(&self, id: u32) -> Option<QueryAchievementCatalog> {
        let after_id = self
            .tracked
            .iter()
            .find(|row| row.achievement_id == id)?
            .next_criteria_id?;
        Some(QueryAchievementCatalog::Criteria {
            achievement_id: id,
            after_id,
        })
    }

    fn cache_categories(
        &mut self,
        categories: Vec<AchievementCategoryEntry>,
        next: Option<u32>,
    ) -> Vec<QueryAchievementCatalog> {
        self.categories.extend(
            categories
                .into_iter()
                .map(|category| (category.category_id, category)),
        );
        self.categories_loaded = next.is_none();
        next.map(|after_id| QueryAchievementCatalog::Categories { after_id })
            .into_iter()
            .collect()
    }
    fn cache_achievements(
        &mut self,
        category_id: u32,
        achievements: Vec<AchievementCatalogEntry>,
        next: Option<u32>,
        first: bool,
    ) {
        let cache = self.pages.entry(category_id).or_default();
        if first {
            cache.entries.clear();
        }
        for entry in achievements {
            update_achievement(&mut cache.entries, entry);
        }
        cache.next = next;
        cache.loaded = true;
        let should_select_first = self.selected_category == Some(category_id)
            && (first || self.selected_achievement.is_none());
        if should_select_first {
            self.select_first();
        }
    }
    fn cache_criteria(
        &mut self,
        achievement_id: u32,
        criteria: Vec<AchievementCriterionLine>,
        next: Option<u32>,
    ) {
        let category_rows = self
            .pages
            .values_mut()
            .flat_map(|page| page.entries.iter_mut());
        let recent_rows = self
            .summary
            .iter_mut()
            .flat_map(|summary| summary.recent.iter_mut());
        for entry in category_rows
            .chain(recent_rows)
            .chain(self.detail.iter_mut())
            .chain(self.tracked.iter_mut())
            .filter(|entry| entry.achievement_id == achievement_id)
        {
            merge_criteria(&mut entry.criteria, &criteria);
            entry.next_criteria_id = next;
        }
    }
    pub fn refresh(&mut self, _update: &AchievementStateUpdate) -> Vec<QueryAchievementCatalog> {
        for page in self.pages.values_mut() {
            page.loaded = false;
        }
        self.summary = None;
        let mut requests = Vec::new();
        if self.visible {
            requests.push(QueryAchievementCatalog::Summary { scope: self.scope });
            if self.search_text.trim().chars().count() >= 3 {
                requests.push(QueryAchievementCatalog::Search {
                    text: self.search_text.trim().into(),
                    scope: self.scope,
                    after_id: 0,
                });
            } else if let Some(category_id) = self.selected_category {
                requests.push(self.category_request(category_id, 0));
            } else if let Some(achievement_id) = self.selected_achievement {
                requests.push(QueryAchievementCatalog::Achievement { achievement_id });
            }
        }
        for entry in &self.tracked {
            let request = QueryAchievementCatalog::Achievement {
                achievement_id: entry.achievement_id,
            };
            if !requests.contains(&request) {
                requests.push(request);
            }
        }
        // A live change needs a post-change response even if an older first page is
        // already in flight. Same-type reliable replies consume these in send order.
        self.pending.extend(requests.iter().cloned());
        requests
    }
    fn select_first(&mut self) {
        self.row_offset = self
            .row_offset
            .min(self.displayed_entries().len().saturating_sub(1));
        self.selected_achievement = self
            .displayed_entries()
            .get(self.row_offset)
            .map(|entry| entry.achievement_id);
        self.criterion_offset = 0;
    }
    pub fn entries(&self) -> &[AchievementCatalogEntry] {
        if self.search_text.trim().chars().count() >= 3 {
            return &self.search_results.entries;
        }
        if self.selected_category.is_none() {
            if let Some(entry) = self
                .detail
                .as_ref()
                .filter(|entry| Some(entry.achievement_id) == self.selected_achievement)
            {
                return std::slice::from_ref(entry);
            }
            let recent = self
                .summary
                .as_ref()
                .map_or(&[][..], |summary| summary.recent.as_slice());
            return if self.selected_achievement.is_none()
                || recent
                    .iter()
                    .any(|entry| Some(entry.achievement_id) == self.selected_achievement)
            {
                recent
            } else {
                &[]
            };
        }
        self.selected_category
            .and_then(|id| self.pages.get(&id))
            .map_or(&[][..], |page| page.entries.as_slice())
    }
    pub fn displayed_entries(&self) -> Vec<&AchievementCatalogEntry> {
        self.entries()
            .iter()
            .filter(|entry| {
                let account = entry.flags & 0x20000 != 0;
                match self.scope {
                    AchievementScope::All => true,
                    AchievementScope::Account => account,
                    AchievementScope::Character => !account,
                }
            })
            .collect()
    }
    pub fn selected(&self) -> Option<&AchievementCatalogEntry> {
        self.entries()
            .iter()
            .find(|entry| Some(entry.achievement_id) == self.selected_achievement)
    }
    pub fn category_tree(&self) -> Vec<(&AchievementCategoryEntry, usize)> {
        let mut rows = Vec::new();
        self.append_children(if self.statistics { 1 } else { -1 }, 0, &mut rows);
        rows
    }
    fn append_children<'a>(
        &'a self,
        parent: i32,
        depth: usize,
        rows: &mut Vec<(&'a AchievementCategoryEntry, usize)>,
    ) {
        let mut children: Vec<_> = self
            .categories
            .values()
            .filter(|row| row.parent_id == parent && (self.statistics || row.category_id != 1))
            .collect();
        children.sort_by_key(|row| (row.order_index, row.category_id));
        for row in children {
            rows.push((row, depth));
            self.append_children(row.category_id as i32, depth + 1, rows);
        }
    }
}
fn update_achievement(entries: &mut Vec<AchievementCatalogEntry>, entry: AchievementCatalogEntry) {
    if let Some(old) = entries
        .iter_mut()
        .find(|old| old.achievement_id == entry.achievement_id)
    {
        *old = entry;
    } else {
        entries.push(entry);
    }
}

fn reply_matches(request: &QueryAchievementCatalog, page: &AchievementCatalogPage) -> bool {
    match (request, page) {
        (
            QueryAchievementCatalog::Statistics {
                category_id: requested,
                ..
            },
            AchievementCatalogPage::Statistics { category_id, .. },
        ) => requested == category_id,
        (
            QueryAchievementCatalog::Search {
                text: requested,
                scope: requested_scope,
                ..
            },
            AchievementCatalogPage::Search { text, scope, .. },
        ) => requested == text && requested_scope == scope,
        (
            QueryAchievementCatalog::Achievement {
                achievement_id: requested,
            },
            AchievementCatalogPage::Achievement { achievement_id, .. },
        ) => requested == achievement_id,
        (QueryAchievementCatalog::Summary { scope }, AchievementCatalogPage::Summary(summary)) => {
            *scope == summary.scope
        }
        (QueryAchievementCatalog::Categories { .. }, AchievementCatalogPage::Categories { .. }) => {
            true
        }
        (
            QueryAchievementCatalog::Category {
                category_id: requested,
                ..
            },
            AchievementCatalogPage::Category { category_id, .. },
        ) => requested == category_id,
        (
            QueryAchievementCatalog::Criteria {
                achievement_id: requested,
                ..
            },
            AchievementCatalogPage::Criteria { achievement_id, .. },
        ) => requested == achievement_id,
        _ => false,
    }
}
fn merge_criteria(
    target: &mut Vec<AchievementCriterionLine>,
    incoming: &[AchievementCriterionLine],
) {
    for criterion in incoming {
        if let Some(old) = target
            .iter_mut()
            .find(|old| old.tree_id == criterion.tree_id)
        {
            *old = criterion.clone();
        } else {
            target.push(criterion.clone());
        }
    }
    target.sort_by_key(|line| (line.order_index, line.tree_id));
}
fn label(
    name: String,
    text: &str,
    rect: (f32, f32, f32, f32),
    color: &str,
    action: &str,
) -> Element {
    let (x, y, w, h) = rect;
    rsx! { fontstring {
        name: {DynName(name)}, width: w, height: h, text, font: GameFont::FrizQuadrata,
        font_size: 12.0, font_color: color, justify_h: "LEFT", onclick: action,
        pos_type: "absolute", left: x, top: y,
    } }
}
const GOLD: &str = "1.0,0.82,0.0,1.0";
const WHITE: &str = "1.0,1.0,1.0,1.0";
const GREY: &str = "0.7,0.7,0.7,1.0";
fn art(name: String, fdid: u32, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, w, h) = rect;
    rsx! { texture { name: {DynName(name)}, texture_fdid: fdid, width: w, height: h, pos_type: "absolute", left: x, top: y, } }
}
pub fn achievement_screen(ctx: &SharedContext) -> Element {
    let window = ctx
        .get::<AchievementWindow>()
        .expect("achievement cache in SharedContext");
    let hide = !window.visible;
    let mut content = flat_panel_chrome(FRAME, (820.0, 600.0), "Achievements", "achievement:close");
    content.extend(points_header(
        window.summary.as_ref().map(|summary| summary.points),
    ));
    content.extend(category_panel(window));
    content.extend(scope_tabs(window));
    content.extend(browse_controls(window));
    if window.selected_category.is_none()
        && window.selected_achievement.is_none()
        && window.search_text.trim().chars().count() < 3
        && !window.statistics
    {
        content.extend(summary_panel(window));
    } else {
        content.extend(achievement_rows(window));
        content.extend(criteria_panel(window));
    }
    rsx! { r#frame {
        name: {DynName(FRAME.into())}, width: 820.0, height: 600.0, strata: FrameStrata::Dialog,
        hidden: hide, mouse_enabled: true, pos_type: "absolute", left: 32.0, top: 100.0,
        {content}
    } }
}
/// Retail Mainline/Blizzard_AchievementUI.lua:298: bare formatted points;
/// XML:1973-1984: white text with a 20x20 shield three units to its right.
fn points_header(points: Option<u32>) -> Element {
    let text = points.map_or_else(
        || "—".into(),
        |points| crate::damage_meter_data::break_up_large_number(u64::from(points)),
    );
    let (width, _) = ui_toolkit::text_measure::measure_text(&text, GameFont::FrizQuadrata, 12.0)
        .expect("achievement header font must be loaded");
    let left = (820.0 - width) / 2.0;
    let shield_left = left + width + 3.0;
    let mut content = label(
        "AchievementFrameHeaderPoints".into(),
        &text,
        // The native font needs 15 units; a shield-height box keeps both centers stable.
        (left, 32.0, width, 20.0),
        WHITE,
        "",
    );
    content.extend(rsx! { texture {
        name: {DynName("AchievementFrameHeaderShield".into())},
        texture_fdid: HEADER_SHIELD_FDID, tex_coords: "0,0.625,0,0.625",
        width: 20.0, height: 20.0, pos_type: "absolute", left: shield_left, top: 33.0,
    } });
    content
}

fn category_panel(window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    if thread_skin() == ActiveSkin::Modern {
        content.extend(art(
            "AchievementCategoriesBackground".into(),
            130652,
            (12.0, 64.0, 204.0, 478.0),
        ));
    }
    if !window.statistics {
        content.extend(label(
            "AchievementCategorySummary".into(),
            "Summary",
            (20.0, 72.0, 180.0, 22.0),
            GOLD,
            "achievement:summary",
        ));
    }
    for (index, (category, depth)) in window
        .category_tree()
        .into_iter()
        .skip(window.category_offset)
        .take(CATEGORIES)
        .enumerate()
    {
        let x = 20.0 + depth as f32 * 12.0;
        let color = if Some(category.category_id) == window.selected_category {
            GOLD
        } else {
            WHITE
        };
        content.extend(label(
            format!("AchievementCategory{}", category.category_id),
            &category.name,
            (x, 96.0 + index as f32 * 24.0, 210.0 - x, 22.0),
            color,
            &format!("achievement:category:{}", category.category_id),
        ));
    }
    content.extend(panel_button(
        "AchievementCategoriesPrev".into(),
        "Previous",
        "achievement:categories_prev",
        window.category_offset > 0,
        (16.0, 551.0, 90.0, 24.0),
    ));
    content.extend(panel_button(
        "AchievementCategoriesNext".into(),
        "Next",
        "achievement:categories_next",
        window.category_offset + CATEGORIES < window.categories.len(),
        (112.0, 551.0, 90.0, 24.0),
    ));
    content
}

// Retail AchievementUI.lua:28,2334-2512: latest three, overall progress, authored summary categories.
fn summary_panel(window: &AchievementWindow) -> Element {
    let Some(summary) = &window.summary else {
        return label(
            "AchievementSummaryLoading".into(),
            "Loading Summary...",
            (240.0, 90.0, 546.0, 30.0),
            WHITE,
            "",
        );
    };
    let mut content = summary_progress_bar(
        "AchievementSummaryTotal",
        summary.completed,
        summary.total,
        (240.0, 70.0, 546.0, 22.0),
    );
    content.extend(label(
        "AchievementSummaryTotalText".into(),
        &format!("{} / {}", summary.completed, summary.total),
        (240.0, 70.0, 546.0, 22.0),
        WHITE,
        "",
    ));
    content.extend(label(
        "AchievementSummaryRecentHeader".into(),
        "Recent Achievements",
        (240.0, 98.0, 546.0, 22.0),
        GOLD,
        "",
    ));
    for (index, row) in summary.recent.iter().enumerate() {
        content.extend(summary_achievement_row(row, 130.0 + index as f32 * 60.0));
    }
    if summary.recent.is_empty() {
        content.extend(label(
            "AchievementSummaryEmpty".into(),
            "No achievements earned yet",
            (240.0, 138.0, 546.0, 30.0),
            WHITE,
            "",
        ));
    }
    content.extend(summary_category_counts(window, summary));
    content
}

fn summary_category_counts(window: &AchievementWindow, summary: &AchievementSummary) -> Element {
    const SUMMARY_CATEGORIES: [u32; 11] = [92, 96, 97, 15522, 95, 168, 169, 201, 155, 15117, 15246];
    let mut content = label(
        "AchievementSummaryCategoriesHeader".into(),
        "Progress Overview",
        (240.0, 330.0, 546.0, 22.0),
        GOLD,
        "",
    );
    for (index, id) in SUMMARY_CATEGORIES.into_iter().enumerate() {
        let Some(category) = window.categories.get(&id) else {
            continue;
        };
        let counts = summary.categories.iter().find(|row| row.category_id == id);
        let (completed, total) = counts.map_or((0, 0), |row| (row.completed, row.total));
        let text = format!("{}  {completed} / {total}", category.name);
        let x = 240.0 + (index % 2) as f32 * 274.0;
        let y = 360.0 + (index / 2) as f32 * 31.0;
        content.extend(summary_progress_bar(
            &format!("AchievementSummaryCategory{id}"),
            completed,
            total,
            (x, y, 254.0, 21.0),
        ));
        content.extend(label(
            format!("AchievementSummaryCategory{id}"),
            &text,
            (x + 6.0, y, 242.0, 21.0),
            WHITE,
            &format!("achievement:category:{id}"),
        ));
    }
    content
}

fn scope_tabs(window: &AchievementWindow) -> Element {
    let tabs = [
        (AchievementScope::All, "All", "all"),
        (AchievementScope::Account, "Account", "account"),
        (AchievementScope::Character, "Character", "character"),
    ];
    let mut content = Vec::new();
    for (index, (scope, title, action)) in tabs.into_iter().enumerate() {
        content.extend(panel_button(
            format!("AchievementScope{action}"),
            title,
            &format!("achievement:scope:{action}"),
            window.scope != scope,
            (240.0 + index as f32 * 112.0, 578.0, 106.0, 22.0),
        ));
    }
    content
}

fn summary_progress_bar(
    name: &str,
    current: u32,
    total: u32,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (x, y, width, height) = rect;
    let fill = if total == 0 {
        0.0
    } else {
        width * current.min(total) as f32 / total as f32
    };
    rsx! {
        r#frame { name: {DynName(format!("{name}Bar"))}, width, height, left: x, top: y, pos_type: "absolute",
            background_color: "0.0,0.0,0.0,0.5",
            texture { name: {DynName(format!("{name}Fill"))}, texture_fdid: 137012,
                width: fill, height, pos_type: "absolute", left: 0.0, top: 0.0,
                vertex_color: "0.0,0.6,0.0,1.0",
            }
        }
    }
}

// SummaryAchievementTemplate is 48 high (Mainline XML:1138-1139).
fn summary_achievement_row(entry: &AchievementCatalogEntry, y: f32) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let mut content = art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (240.0, y, 40.0, 40.0),
    );
    content.extend(art(
        format!("{prefix}IconFrame"),
        130656,
        (237.0, y - 3.0, 46.0, 46.0),
    ));
    content.extend(label(
        format!("{prefix}Name"),
        &entry.name,
        (292.0, y, 400.0, 20.0),
        GOLD,
        &action,
    ));
    content.extend(label(
        format!("{prefix}Description"),
        &entry.description,
        (292.0, y + 22.0, 400.0, 26.0),
        WHITE,
        &action,
    ));
    content.extend(art(
        format!("{prefix}Shield"),
        130665,
        (728.0, y, 40.0, 40.0),
    ));
    content.extend(label(
        format!("{prefix}Points"),
        &entry.points.to_string(),
        (740.0, y + 12.0, 35.0, 20.0),
        GOLD,
        &action,
    ));
    content
}

fn browse_controls(window: &AchievementWindow) -> Element {
    let mut content = panel_button(
        "AchievementTabAchievements".into(),
        "Achievements",
        "achievement:achievements",
        window.statistics,
        (20.0, 578.0, 106.0, 22.0),
    );
    content.extend(panel_button(
        "AchievementTabStatistics".into(),
        "Statistics",
        "achievement:statistics",
        !window.statistics,
        (130.0, 578.0, 92.0, 22.0),
    ));
    content.extend(rsx! { editbox { name: "AchievementSearchBox", text: {window.search_text.as_str()}, width: 160.0, height: 22.0, left: 622.0, top: 32.0, pos_type: "absolute", max_letters:256, auto_focus:false, background_color:"0.0,0.0,0.0,0.8", } });
    content.extend(label(
        "AchievementSearchLabel".into(),
        "Search",
        (568.0, 32.0, 52.0, 22.0),
        WHITE,
        "",
    ));
    content
}

fn statistic_row(entry: &AchievementCatalogEntry, y: f32) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let mut content = label(
        format!("{prefix}Name"),
        &entry.name,
        (240.0, y, 440.0, STATISTIC_ROW_HEIGHT),
        WHITE,
        &action,
    );
    let value = entry.statistic_value.map_or_else(
        || "Unevaluated".into(),
        crate::damage_meter_data::break_up_large_number,
    );
    content.extend(label(
        format!("{prefix}Statistic"),
        &value,
        (685.0, y, 110.0, STATISTIC_ROW_HEIGHT),
        GOLD,
        &action,
    ));
    content
}

fn achievement_rows(window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    for (index, entry) in window
        .displayed_entries()
        .into_iter()
        .skip(window.row_offset)
        .take(window.visible_row_count())
        .enumerate()
    {
        let spacing = if window.statistics && window.search_text.trim().chars().count() < 3 {
            STATISTIC_ROW_HEIGHT
        } else {
            96.0
        };
        let y = 68.0 + index as f32 * spacing;
        if entry.flags & 1 != 0 {
            content.extend(statistic_row(entry, y));
        } else {
            content.extend(achievement_row(
                entry,
                y,
                Some(entry.achievement_id) == window.selected_achievement,
            ));
        }
    }
    if window.displayed_entries().is_empty() {
        let message = if window.search_text.trim().chars().count() >= 3 {
            if window.search_results.loaded {
                "No results"
            } else {
                "Searching..."
            }
        } else if let Some(achievement_id) = window
            .selected_achievement
            .filter(|_| window.selected_category.is_none())
        {
            if window
                .pending
                .contains(&QueryAchievementCatalog::Achievement { achievement_id })
            {
                "Loading achievement..."
            } else {
                "Achievement unavailable"
            }
        } else if window.selected_category.is_none() {
            "Select a category"
        } else if window
            .pages
            .get(&window.selected_category.unwrap())
            .is_some_and(|page| page.loaded)
        {
            "No achievements in this category"
        } else {
            "Loading achievements..."
        };
        content.extend(label(
            "AchievementEmpty".into(),
            message,
            (240.0, 90.0, 550.0, 30.0),
            WHITE,
            "",
        ));
    }
    content.extend(panel_button(
        "AchievementRowsPrev".into(),
        "Previous achievements",
        "achievement:rows_prev",
        window.row_offset > 0,
        (240.0, 358.0, 174.0, 24.0),
    ));
    let more = window.row_offset + window.visible_row_count() < window.displayed_entries().len()
        || (window.search_text.trim().chars().count() >= 3 && window.search_results.next.is_some())
        || window
            .selected_category
            .and_then(|id| window.pages.get(&id))
            .is_some_and(|page| page.next.is_some());
    content.extend(panel_button(
        "AchievementRowsNext".into(),
        "More achievements",
        "achievement:more_rows",
        more,
        (612.0, 358.0, 174.0, 24.0),
    ));
    content
}

fn criteria_panel(window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    if let Some(entry) = window.selected().filter(|entry| entry.flags & 1 == 0) {
        content.extend(label(
            "AchievementCriteriaTitle".into(),
            &format!("Criteria — {}", entry.name),
            (240.0, 397.0, 396.0, 22.0),
            GOLD,
            "",
        ));
        for (index, line) in entry
            .criteria
            .iter()
            .skip(window.criterion_offset)
            .take(CRITERIA)
            .enumerate()
        {
            let status = if !line.progress_supported {
                "Unevaluated"
            } else if line.completed {
                "Complete"
            } else {
                "In progress"
            };
            let text = if line.progress_supported {
                format!(
                    "[{status}] {} — {}/{}",
                    line.description, line.current, line.required
                )
            } else {
                format!("[{status}] {}", line.description)
            };
            content.extend(label(
                format!(
                    "Achievement{}Criterion{}",
                    entry.achievement_id, line.tree_id
                ),
                &text,
                (240.0, 425.0 + index as f32 * 20.0, 546.0, 20.0),
                WHITE,
                "",
            ));
        }
        let tracked = window
            .tracked
            .iter()
            .any(|row| row.achievement_id == entry.achievement_id);
        content.extend(panel_button(
            "AchievementTrack".into(),
            if tracked { "Stop tracking" } else { "Track" },
            &format!("achievement:track:{}", entry.achievement_id),
            !entry.earned && entry.flags & 1 == 0,
            (640.0, 397.0, 146.0, 24.0),
        ));
        if let Some(error) = &window.tracking_error {
            content.extend(label(
                "AchievementTrackingError".into(),
                error,
                (240.0, 577.0, 546.0, 20.0),
                WHITE,
                "",
            ));
        }
        content.extend(panel_button(
            "AchievementCriteriaPrev".into(),
            "Previous criteria",
            "achievement:criteria_prev",
            window.criterion_offset > 0,
            (240.0, 551.0, 150.0, 24.0),
        ));
        content.extend(panel_button(
            "AchievementCriteriaNext".into(),
            "More criteria",
            "achievement:more_criteria",
            window.criterion_offset + CRITERIA < entry.criteria.len()
                || entry.next_criteria_id.is_some(),
            (636.0, 551.0, 150.0, 24.0),
        ));
    }
    content
}

fn achievement_row(entry: &AchievementCatalogEntry, y: f32, selected: bool) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let modern = thread_skin() == ActiveSkin::Modern;
    let ink = if modern { "0.1,0.08,0.05,1.0" } else { WHITE };
    let title_color = if modern {
        ink
    } else if selected {
        GOLD
    } else {
        WHITE
    };
    let date_color = if modern { ink } else { GREY };
    let mut content = if modern {
        art(
            format!("{prefix}Background"),
            235397,
            (232.0, y, 562.0, 92.0),
        )
    } else {
        Vec::new()
    };
    content.extend(art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (240.0, y + 12.0, 48.0, 48.0),
    ));
    content.extend(label(
        format!("{prefix}Name"),
        &entry.name,
        (300.0, y + 6.0, 414.0, 20.0),
        title_color,
        &action,
    ));
    content.extend(label(
        format!("{prefix}Description"),
        &entry.description,
        (300.0, y + 28.0, 414.0, 40.0),
        ink,
        &action,
    ));
    content.extend(art(
        format!("{prefix}Shield"),
        130665,
        (728.0, y + 12.0, 48.0, 48.0),
    ));
    content.extend(label(
        format!("{prefix}Points"),
        &entry.points.to_string(),
        (744.0, y + 27.0, 35.0, 20.0),
        GOLD,
        &action,
    ));
    let date = if entry.earned {
        entry
            .earned_at
            .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
            .map_or("Earned — date unknown".into(), |date| {
                format!("Earned {} (UTC)", date.format("%Y-%m-%d"))
            })
    } else if !entry.progress_supported {
        "Unevaluated".into()
    } else {
        "Not earned".into()
    };
    content.extend(label(
        format!("{prefix}Date"),
        &date,
        (300.0, y + 70.0, 460.0, 18.0),
        date_color,
        &action,
    ));
    content
}

/// Own-context chat links carry only a real catalog ID; no fabricated GUID/comparison facts.
pub fn achievement_link(id: u32, name: &str) -> String {
    format!("|Hachievement:{id}|h[{name}]|h")
}

/// Mainline AlertFrameSystems.xml:173-330 and lua:348-388: authored atlas regions and anchors.
pub fn achievement_toast_screen(ctx: &SharedContext) -> Element {
    use crate::quest_art::named_atlas_texture;
    let toast = ctx
        .get::<shared::protocol::AchievementToastSnapshot>()
        .expect("toast payload");
    let action = format!("achievement:open:{}", toast.achievement_id);
    let mut content = named_atlas_texture(
        "AchievementToastBackground".into(),
        "ui-achievement-alert-background",
        (-5.0, 0.0, 310.0, 101.0),
    );
    content.extend(art(
        "AchievementToastIcon".into(),
        toast.icon_fdid,
        (9.0, 26.5, 52.0, 52.0),
    ));
    content.extend(named_atlas_texture(
        "AchievementToastIconFrame".into(),
        "ui-achievement-iconframe",
        (-3.0, 15.5, 74.0, 74.0),
    ));
    let shield = if toast.points == 0 {
        "UI-Achievement-Shield-NoPoints"
    } else {
        "ui-achievement-shield-2"
    };
    content.extend(named_atlas_texture(
        "AchievementToastShield".into(),
        shield,
        (231.0, 21.0, 62.0, 55.0),
    ));
    let points = toast.points.to_string();
    rsx! {
        r#frame { name: "AchievementToast", width:300.0, height:101.0, mouse_enabled:true, onclick:{action.as_str()},
            pos_type:"absolute", left:"50%", margin_left:-150.0, bottom:128.0,
            {content}
            fontstring { name:"AchievementToastTitle", text:"Achievement earned", font:GameFont::FrizQuadrata,
                font_size:9.0, font_color:"0,0,0,1", justify_h:"CENTER", justify_v:"MIDDLE",
                width:200.0,height:12.0,pos_type:"absolute",left:57.0,top:23.0,
            }
            fontstring { name:"AchievementToastName", text:{toast.name.as_str()}, font:GameFont::FrizQuadrata,
                font_size:12.0,font_color:WHITE,justify_h:"CENTER",justify_v:"MIDDLE",onclick:{action.as_str()},
                width:155.0,height:36.0,pos_type:"absolute",left:79.5,top:35.0,
            }
            fontstring { name:"AchievementToastPoints", text:{points.as_str()}, font:GameFont::FrizQuadrata,
                font_size:12.0,font_color:WHITE,justify_h:"CENTER",justify_v:"MIDDLE",
                width:38.0,height:16.0,pos_type:"absolute",left:243.0,top:41.0,
            }
        }
    }
}
