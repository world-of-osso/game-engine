//! On-demand AchievementFrame: ordered category tree, cached cursor pages, live refresh.
//! Retail AchievementUI.lua:572 (categories), :1814 (rows), :1952 (criteria).
#[path = "achievement_comparison.rs"]
mod comparison_state;
#[path = "achievement_search_catalog.rs"]
pub mod search_catalog;
use shared::protocol::{
    ACHIEVEMENT_SEARCH_MAX_CHARS, AchievementCatalogEntry, AchievementCatalogPage,
    AchievementCategoryEntry, AchievementCriterionLine, AchievementDelta,
    AchievementProgressSnapshot, AchievementScope, AchievementSnapshot, AchievementStateUpdate,
    AchievementSummary, QueryAchievementCatalog,
};
use std::collections::BTreeMap;

pub const OPEN_ACTION: &str = "micro:AchievementMicroButton";
pub const SEARCH_PREVIEW_LIMIT: usize = 5;
pub const CRITERIA: usize = 6;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CategoryPage {
    pub entries: Vec<AchievementCatalogEntry>,
    pub next: Option<u32>,
    pub loaded: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AchievementComparison {
    pub target_entity: u64,
    pub name: String,
    pub error: Option<String>,
    pub window: AchievementWindow,
    /// Exact own-row IDs queried independently of the other faction's category cursor.
    pub entries: BTreeMap<u32, Option<AchievementCatalogEntry>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AchievementFilter {
    #[default]
    All,
    Completed,
    Incomplete,
}

// The requested Mainline top-level set. Guild/Statistics have separate trees;
// DB2 Housing (15606) is outside this screen's specified top-level set.
const RETAIL_CATEGORIES: [u32; 14] = [
    92, 96, 97, 15522, 95, 168, 169, 201, 155, 15117, 15246, 15301, 15234, 81,
];
const GUILD_CATEGORY: i32 = 15076;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AchievementWindow {
    pub visible: bool,
    pub comparison: Option<Box<AchievementComparison>>,
    /// Mainline categories reveal one parent's children at a time.
    pub expanded_category: Option<u32>,
    /// Selection and expansion differ: a newly loaded page starts collapsed.
    pub expanded_achievement: Option<u32>,
    pub categories: BTreeMap<u32, AchievementCategoryEntry>,
    pub pages: BTreeMap<u32, CategoryPage>,
    pub selected_category: Option<u32>,
    pub selected_achievement: Option<u32>,
    pub criterion_offset: usize,
    pub scope: AchievementScope,
    pub summary: Option<AchievementSummary>,
    pub detail: Option<AchievementCatalogEntry>,
    pub tracked: Vec<AchievementCatalogEntry>,
    pub tracked_ids: Vec<u32>,
    pub tracking_error: Option<String>,
    pub statistics: bool,
    pub guild: bool,
    pub filter: AchievementFilter,
    pub filter_menu: bool,
    pub search_text: String,
    pub search_results: CategoryPage,
    pub search_focused: bool,
    pub full_search: bool,
    /// Zero-based visible preview index; Show All follows the received rows.
    pub search_preview_selected: usize,
    categories_loaded: bool,
    pending: Vec<QueryAchievementCatalog>,
    /// The connection's last full snapshot with every later delta applied.
    baseline: Option<AchievementSnapshot>,
}
impl AchievementWindow {
    pub fn action(&mut self, action: &str) -> Vec<QueryAchievementCatalog> {
        let mut requests = Vec::new();
        match action {
            OPEN_ACTION => {
                self.comparison = None;
                self.toggle(&mut requests);
            }
            "achievement:close" => {
                self.visible = false;
                self.search_focused = false;
                self.comparison = None;
            }
            "achievement:stop_compare" => self.comparison = None,
            "achievement:statistics" => self.change_statistics(true),
            "achievement:achievements" => self.change_tab(false, false),
            "achievement:guild" => self.change_tab(false, true),
            "achievement:search_focus" => {
                self.search_focused = true;
                self.search_preview_selected = 0;
            }
            "achievement:search_blur" => self.search_focused = false,
            "achievement:search_dismiss" => self.full_search = false,
            "achievement:search_submit" => {
                if let Some(selected) = self.search_preview_action() {
                    return self.action(&selected);
                }
            }
            "achievement:search_all" => {
                if self.has_more_search_results() {
                    self.full_search = true;
                    self.search_focused = false;
                    self.selected_achievement = None;
                    self.expanded_achievement = None;
                    self.detail = None;
                    self.select_first();
                }
            }
            "achievement:filter_menu" => self.filter_menu = !self.filter_menu,
            "achievement:filter:all" => self.change_filter(AchievementFilter::All),
            "achievement:filter:completed" => self.change_filter(AchievementFilter::Completed),
            "achievement:filter:incomplete" => self.change_filter(AchievementFilter::Incomplete),
            "achievement:summary" => {
                self.selected_category = None;
                self.selected_achievement = None;
                self.clear_search();
                self.statistics = false;
                self.criterion_offset = 0;
                self.expanded_achievement = None;
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
            "achievement:more_rows" => self.advance_rows(&mut requests),
            "achievement:criteria_prev" => {
                self.criterion_offset = self.criterion_offset.saturating_sub(CRITERIA)
            }
            "achievement:more_criteria" => self.advance_criteria(&mut requests),
            _ => self.select_action(action, &mut requests),
        }
        self.enqueue(requests)
    }
    fn change_filter(&mut self, filter: AchievementFilter) {
        self.filter = filter;
        self.filter_menu = false;
        self.expanded_achievement = None;
        self.selected_achievement = None;
        self.select_first();
    }

    fn change_statistics(&mut self, statistics: bool) {
        self.change_tab(statistics, false);
    }

    fn change_tab(&mut self, statistics: bool, guild: bool) {
        if self.statistics == statistics && self.guild == guild {
            return;
        }
        self.statistics = statistics;
        self.guild = guild;
        self.filter_menu = false;
        self.expanded_category = None;
        self.pages.clear();
        self.selected_category = None;
        self.selected_achievement = None;
        self.detail = None;
        self.clear_search();
        self.expanded_achievement = None;
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

    pub fn has_more_search_results(&self) -> bool {
        self.search_results.entries.len() > SEARCH_PREVIEW_LIMIT
            || self.search_results.next.is_some()
    }

    /// Read only the existing achievement-request predicate; pending order stays private.
    pub fn is_achievement_request_pending(&self, achievement_id: u32) -> bool {
        self.pending
            .contains(&QueryAchievementCatalog::Achievement { achievement_id })
    }

    /// Mainline Lua3552–3585: wrap only among visible, ready preview items.
    pub fn search_preview_count(&self) -> usize {
        if !self.search_focused
            || self.full_search
            || !self.search_results.loaded
            || self.search_text.trim().chars().count() < 3
        {
            return 0;
        }
        let rows = self.search_results.entries.len().min(SEARCH_PREVIEW_LIMIT);
        if rows == 0 {
            return 0;
        }
        rows + usize::from(self.has_more_search_results())
    }

    fn move_search_preview(&mut self, step: i32) {
        let count = self.search_preview_count() as i64;
        if count == 0 {
            return;
        }
        let next = (self.search_preview_selected as i64 + i64::from(step)).rem_euclid(count);
        self.search_preview_selected = next as usize;
    }

    fn search_preview_action(&self) -> Option<String> {
        if self.search_preview_count() == 0 {
            return None;
        }
        let rows = self.search_results.entries.len().min(SEARCH_PREVIEW_LIMIT);
        if self.search_preview_selected < rows {
            let id = self.search_results.entries[self.search_preview_selected].achievement_id;
            Some(format!("achievement:preview:{id}"))
        } else if self.search_preview_selected == rows && self.has_more_search_results() {
            Some("achievement:search_all".into())
        } else {
            None
        }
    }

    fn clear_search(&mut self) {
        self.search_text.clear();
        self.search_results = CategoryPage::default();
        self.full_search = false;
        self.search_focused = false;
        self.search_preview_selected = 0;
    }

    fn search(&mut self, text: &str, requests: &mut Vec<QueryAchievementCatalog>) {
        self.search_text = text.chars().take(ACHIEVEMENT_SEARCH_MAX_CHARS).collect();
        self.search_results = CategoryPage::default();
        self.full_search = false;
        self.search_preview_selected = 0;
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
        if let Some(target) = action
            .strip_prefix("achievement:compare:")
            .and_then(|id| id.parse().ok())
        {
            self.start_comparison(target, requests);
        } else if let Some(text) = action.strip_prefix("achievement:search:") {
            self.search(text, requests);
        } else if let Some(step) = action
            .strip_prefix("achievement:search_move:")
            .and_then(|value| value.parse::<i32>().ok())
        {
            self.move_search_preview(step);
        } else if let Some(index) = action
            .strip_prefix("achievement:search_select:")
            .and_then(|value| value.parse::<usize>().ok())
        {
            if index < self.search_preview_count() {
                self.search_preview_selected = index;
            }
        } else if let Some(id) = action
            .strip_prefix("achievement:preview:")
            .and_then(|id| id.parse().ok())
        {
            if self
                .search_results
                .entries
                .iter()
                .take(5)
                .any(|entry| entry.achievement_id == id)
            {
                self.open_achievement(id, requests);
            }
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
            let category = &self.categories[&id];
            if category.parent_id == self.category_root() {
                self.expanded_category = if self.expanded_category == Some(id) {
                    None
                } else {
                    Some(id)
                };
            } else {
                self.expanded_category = Some(category.parent_id as u32);
            }
            self.selected_category = Some(id);
            self.expanded_achievement = None;
            self.clear_search();
            self.detail = None;
            self.selected_achievement = None;
            self.criterion_offset = 0;
            let other_missing = self
                .comparison
                .as_ref()
                .is_some_and(|other| !other.window.pages.get(&id).is_some_and(|page| page.loaded));
            if other_missing || !self.pages.get(&id).is_some_and(|page| page.loaded) {
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
                if self.full_search {
                    self.open_achievement(id, requests);
                } else {
                    self.selected_achievement = Some(id);
                    self.expanded_achievement = if self.expanded_achievement == Some(id) {
                        None
                    } else {
                        Some(id)
                    };
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
        self.expanded_category = None;
        self.selected_category = None;
        self.selected_achievement = Some(id);
        self.expanded_achievement = Some(id);
        self.detail = None;
        self.clear_search();
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

    /// The native ScrollBox requests the next cursor at its bottom, never a local row slice.
    fn advance_rows(&mut self, requests: &mut Vec<QueryAchievementCatalog>) {
        if self.full_search {
            if let Some(after_id) = self.search_results.next {
                requests.push(QueryAchievementCatalog::Search {
                    text: self.search_text.trim().into(),
                    scope: self.scope,
                    after_id,
                });
            }
            return;
        }
        if let Some(category_id) = self.selected_category {
            if let Some(after_id) = self.pages.get(&category_id).and_then(|page| page.next) {
                requests.push(self.category_request(category_id, after_id));
            }
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
    fn start_comparison(
        &mut self,
        target_entity: u64,
        requests: &mut Vec<QueryAchievementCatalog>,
    ) {
        self.visible = true;
        self.selected_category = None;
        self.selected_achievement = None;
        self.expanded_achievement = None;
        self.clear_search();
        self.statistics = false;
        self.pending
            .retain(|request| !matches!(request, QueryAchievementCatalog::Comparison { .. }));
        self.comparison = Some(Box::new(AchievementComparison {
            target_entity,
            name: String::new(),
            error: None,
            window: AchievementWindow::default(),
            entries: BTreeMap::new(),
        }));
        requests.push(QueryAchievementCatalog::Summary { scope: self.scope });
        if !self.categories_loaded {
            requests.push(QueryAchievementCatalog::Categories { after_id: 0 });
        }
    }

    fn comparison_requests(
        &mut self,
        requests: &[QueryAchievementCatalog],
    ) -> Vec<QueryAchievementCatalog> {
        let Some(comparison) = self.comparison.as_mut() else {
            return Vec::new();
        };
        let other = &mut comparison.window;
        other.selected_category = self.selected_category;
        other.selected_achievement = self.selected_achievement;
        other.expanded_achievement = self.expanded_achievement;
        other.statistics = self.statistics;
        other.scope = self.scope;
        other.search_text = self.search_text.clone();
        other.full_search = self.full_search;
        let queries = requests
            .iter()
            .filter(|request| {
                !matches!(
                    request,
                    QueryAchievementCatalog::Categories { .. }
                        | QueryAchievementCatalog::Comparison { .. }
                )
            })
            .cloned()
            .collect();
        other
            .enqueue(queries)
            .into_iter()
            .map(|query| QueryAchievementCatalog::Comparison {
                target_entity: comparison.target_entity,
                query: Box::new(query.try_into().expect("ordinary comparison query")),
            })
            .collect()
    }

    fn enqueue(
        &mut self,
        mut requests: Vec<QueryAchievementCatalog>,
    ) -> Vec<QueryAchievementCatalog> {
        let comparisons = self.comparison_requests(&requests);
        requests.extend(comparisons);
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
            AchievementCatalogPage::Comparison {
                target_entity,
                target_name,
                page,
                error,
            } => self.apply_comparison(target_entity, target_name, page, error),
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
                    if !self.full_search {
                        self.search_preview_selected = 0;
                    }
                    if self.full_search {
                        self.select_first();
                    }
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
        let mut requests = requests;
        requests.extend(self.request_comparison_rows());
        self.enqueue(requests)
    }

    fn request_comparison_rows(&self) -> Vec<QueryAchievementCatalog> {
        let Some(comparison) = self
            .comparison
            .as_ref()
            .filter(|other| other.error.is_none())
        else {
            return Vec::new();
        };
        let rows = if self.selected_category.is_none() && self.selected_achievement.is_none() {
            self.summary
                .as_ref()
                .map_or_else(Vec::new, |summary| summary.recent.iter().collect())
        } else {
            self.displayed_entries()
        };
        rows.into_iter()
            .filter(|entry| !comparison.entries.contains_key(&entry.achievement_id))
            .map(|entry| QueryAchievementCatalog::Comparison {
                target_entity: comparison.target_entity,
                query: Box::new(shared::protocol::AchievementComparisonQuery::Achievement {
                    achievement_id: entry.achievement_id,
                }),
            })
            .collect()
    }

    fn apply_comparison(
        &mut self,
        target: u64,
        name: String,
        page: Option<Box<shared::protocol::AchievementComparisonPage>>,
        error: Option<String>,
    ) -> Vec<QueryAchievementCatalog> {
        let Some(comparison) = self
            .comparison
            .as_mut()
            .filter(|value| value.target_entity == target)
        else {
            return Vec::new();
        };
        comparison.error = error;
        if !name.is_empty() {
            comparison.name = name;
        }
        let Some(page) = page else {
            return Vec::new();
        };
        let page: AchievementCatalogPage = (*page).into();
        comparison_state::cache_page(comparison, &page);
        comparison
            .window
            .apply(page)
            .into_iter()
            .map(|query| QueryAchievementCatalog::Comparison {
                target_entity: target,
                query: Box::new(query.try_into().expect("ordinary comparison continuation")),
            })
            .collect()
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
    /// The connection's earned set and progress: last full snapshot plus later deltas.
    pub fn snapshot(&self) -> Option<&AchievementSnapshot> {
        self.baseline.as_ref()
    }
    /// A full snapshot (initial login or reconnect) replaces the baseline; an earned alert
    /// carries none and keeps it. Either invalidates the cached pages.
    pub fn refresh(&mut self, update: &AchievementStateUpdate) -> Vec<QueryAchievementCatalog> {
        if let Some(snapshot) = &update.snapshot {
            self.baseline = Some(snapshot.clone());
        }
        self.invalidate()
    }
    /// Reduces a delta to the state of the full snapshot it replaces, with the same
    /// invalidation. A delta without a full baseline on this connection is an error.
    pub fn apply_delta(
        &mut self,
        delta: &AchievementDelta,
    ) -> Result<Vec<QueryAchievementCatalog>, String> {
        let baseline = self
            .baseline
            .as_mut()
            .ok_or("Achievement delta before a full achievement snapshot on this connection")?;
        if let Some(earned_ids) = &delta.earned_ids {
            baseline.earned_ids = earned_ids.clone();
        }
        let progress = &mut baseline.progress;
        progress.retain(|row| !delta.removed_progress_ids.contains(&row.achievement_id));
        for row in &delta.progress_upserts {
            upsert_progress(progress, row);
        }
        Ok(self.invalidate())
    }
    fn invalidate(&mut self) -> Vec<QueryAchievementCatalog> {
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
        self.expanded_achievement = None;
        self.selected_achievement = self
            .displayed_entries()
            .first()
            .map(|entry| entry.achievement_id);
        self.criterion_offset = 0;
    }
    pub fn entries(&self) -> &[AchievementCatalogEntry] {
        if self.full_search {
            return &self.search_results.entries;
        }
        if self.selected_category.is_none() {
            if self.statistics {
                return &[];
            }
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
                self.statistics
                    || match self.filter {
                        AchievementFilter::All => true,
                        AchievementFilter::Completed => entry.earned,
                        AchievementFilter::Incomplete => !entry.earned,
                    }
            })
            .filter(|entry| !matches!(self.selected_category, Some(81 | 15093)) || entry.earned)
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
    fn category_root(&self) -> i32 {
        if self.statistics {
            1
        } else if self.guild {
            GUILD_CATEGORY
        } else {
            -1
        }
    }
    pub fn category_tree(&self) -> Vec<(&AchievementCategoryEntry, usize)> {
        let mut rows = Vec::new();
        self.append_children(self.category_root(), 0, &mut rows);
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
        if parent == -1 {
            children.retain(|row| RETAIL_CATEGORIES.contains(&row.category_id));
            children.sort_by_key(|row| {
                RETAIL_CATEGORIES
                    .iter()
                    .position(|id| *id == row.category_id)
            });
        } else {
            children.sort_by_key(|row| (row.order_index, row.category_id));
        }
        for row in children {
            rows.push((row, depth));
            if self.expanded_category == Some(row.category_id) {
                self.append_children(row.category_id as i32, depth + 1, rows);
            }
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
            QueryAchievementCatalog::Comparison {
                target_entity: requested,
                query,
            },
            AchievementCatalogPage::Comparison {
                target_entity,
                page,
                error,
                ..
            },
        ) => {
            requested == target_entity
                && (error.is_some()
                    || page.as_ref().is_some_and(|page| {
                        reply_matches(
                            &query.as_ref().clone().into(),
                            &page.as_ref().clone().into(),
                        )
                    }))
        }
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
/// Full snapshots list progress in ascending achievement id (the catalog's order).
fn upsert_progress(
    progress: &mut Vec<AchievementProgressSnapshot>,
    row: &AchievementProgressSnapshot,
) {
    match progress.binary_search_by_key(&row.achievement_id, |existing| existing.achievement_id) {
        Ok(index) => progress[index] = row.clone(),
        Err(index) => progress.insert(index, row.clone()),
    }
}
/// Own-context chat links carry only a real catalog ID; no fabricated GUID/comparison facts.
pub fn achievement_link(id: u32, name: &str) -> String {
    format!("|Hachievement:{id}|h[{name}]|h")
}
