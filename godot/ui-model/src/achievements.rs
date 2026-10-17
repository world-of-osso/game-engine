//! On-demand AchievementFrame: ordered category tree, cached cursor pages, live refresh.
//! Retail AchievementUI.lua:572 (categories), :1814 (rows), :1952 (criteria).
use crate::quest_art::{DynName, panel_button};
#[path = "achievement_art.rs"]
mod artwork;
#[path = "achievement_comparison.rs"]
mod comparison_view;
use crate::ui::strata::FrameStrata;
use shared::protocol::{
    ACHIEVEMENT_SEARCH_MAX_CHARS, AchievementCatalogEntry, AchievementCatalogPage,
    AchievementCategoryEntry, AchievementCriterionLine, AchievementScope, AchievementStateUpdate,
    AchievementSnapshot, AchievementSummary, QueryAchievementCatalog,
};
use std::collections::BTreeMap;
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const OPEN_ACTION: &str = "micro:AchievementMicroButton";
pub const FRAME: &str = "AchievementFrame";
pub const ROWS_SCROLLBOX: &str = "AchievementRowsScrollBox";
/// Local Mainline UI atlas1771: alert plate, icon frame and point shield.
pub const TOAST_ATLAS_FDID: u32 = 4_035_524;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
/// UiTextureAtlasMember12156/12180/12183/12749, identical in both pinned skin catalogs.
const ALERT_BACKGROUND: AtlasArt = AtlasArt {
    fdid: TOAST_ATLAS_FDID,
    atlas: (1024.0, 1024.0),
    rect: (640.0, 950.0, 174.0, 275.0),
};
const ALERT_ICON_FRAME: AtlasArt = AtlasArt {
    rect: (110.0, 184.0, 518.0, 592.0),
    ..ALERT_BACKGROUND
};
const ALERT_SHIELD: AtlasArt = AtlasArt {
    rect: (110.0, 172.0, 710.0, 765.0),
    ..ALERT_BACKGROUND
};
const ALERT_NO_POINTS: AtlasArt = AtlasArt {
    rect: (110.0, 170.0, 825.0, 881.0),
    ..ALERT_BACKGROUND
};
const STATISTIC_ROW_HEIGHT: f32 = 24.0;
const CRITERIA: usize = 6;
/// Local Retail art: UI-Achievement-Category-Background, AchievementBackground, Shield.
pub const ART: &[u32] = &[
    130652,
    235397,
    130666,
    TOAST_ATLAS_FDID,
    137012,
    130751,
    130755,
    130656,
    130657,
    130658,
    130659,
    130660,
    130661,
    130662,
    130671,
    130672,
    235400,
    235402,
    235404,
    235405,
    235408,
    235411,
    235412,
    452109,
    HEADER_SHIELD_FDID,
    235401,
];
/// Retail Mainline/Blizzard_AchievementUI.xml:1979-1984, UI-Achievement-TinyShield.
const HEADER_SHIELD_FDID: u32 = 235_415;

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
    categories_loaded: bool,
    pending: Vec<QueryAchievementCatalog>,
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
                self.comparison = None;
            }
            "achievement:stop_compare" => self.comparison = None,
            "achievement:statistics" => self.change_statistics(true),
            "achievement:achievements" => self.change_tab(false, false),
            "achievement:guild" => self.change_tab(false, true),
            "achievement:filter_menu" => self.filter_menu = !self.filter_menu,
            "achievement:filter:all" => self.change_filter(AchievementFilter::All),
            "achievement:filter:completed" => self.change_filter(AchievementFilter::Completed),
            "achievement:filter:incomplete" => self.change_filter(AchievementFilter::Incomplete),
            "achievement:summary" => {
                self.selected_category = None;
                self.selected_achievement = None;
                self.search_text.clear();
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
        self.search_text.clear();
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

    fn search(&mut self, text: &str, requests: &mut Vec<QueryAchievementCatalog>) {
        self.search_text = text.chars().take(ACHIEVEMENT_SEARCH_MAX_CHARS).collect();
        self.search_results = CategoryPage::default();
        self.selected_category = None;
        self.selected_achievement = None;
        self.detail = None;
        self.expanded_achievement = None;
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
            self.search_text.clear();
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
                if self.search_text.trim().chars().count() >= 3 {
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
        self.search_text.clear();
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
        if self.search_text.trim().chars().count() >= 3 {
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
        self.search_text.clear();
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
        comparison_view::cache_page(comparison, &page);
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
        None
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
        self.expanded_achievement = None;
        self.selected_achievement = self
            .displayed_entries()
            .first()
            .map(|entry| entry.achievement_id);
        self.criterion_offset = 0;
    }
    pub fn entries(&self) -> &[AchievementCatalogEntry] {
        if self.search_text.trim().chars().count() >= 3 {
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
    // Mainline AchievementUI.xml:632,1091,1113,2002: the files are sheets, not whole-frame images.
    let coords = match fdid {
        130652 => "0,0.6640625,0,1",
        130656 => "0,0.5625,0,0.5625",
        130666 => "0,0.5,0,0.5",
        235397 => "0,1,0,0.5",
        _ => "0,1,0,1",
    };
    let tint = artwork::skin_tint(fdid);
    rsx! { texture { name: {DynName(name)}, texture_fdid: fdid, tex_coords:coords, vertex_color:tint, width: w, height: h, pos_type: "absolute", left: x, top: y, } }
}
pub fn achievement_screen(ctx: &SharedContext) -> Element {
    let window = ctx
        .get::<AchievementWindow>()
        .expect("achievement cache in SharedContext");
    let hide = !window.visible;
    let show_summary = window.selected_category.is_none()
        && window.selected_achievement.is_none()
        && window.search_text.trim().chars().count() < 3
        && !window.statistics;
    let mut content = artwork::chrome(show_summary);
    content.extend(points_header(
        window
            .summary
            .as_ref()
            .filter(|_| !window.guild)
            .map(|summary| summary.points),
    ));
    content.extend(category_panel(ctx, window));
    content.extend(browse_controls(window));
    if window.comparison.is_some() {
        content.extend(comparison_view::content(ctx, window, show_summary));
    } else if show_summary {
        content.extend(summary_panel(window));
    } else {
        content.extend(achievement_rows(ctx, window));
    }
    // Godot Control input follows scene order, not CanvasItem z_index. Mount
    // dropdown controls after the rows they overlap, matching their visual order.
    content.extend(filter_controls(window, show_summary));
    let width = if window.comparison.is_some() {
        888.0
    } else {
        768.0
    };
    rsx! { r#frame {
        name: {DynName(FRAME.into())}, width, height: 500.0, strata: FrameStrata::Dialog,
        hidden: hide, mouse_enabled: true, pos_type: "absolute", left: 0.0, top: 104.0,
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
    // Header center: 26 + 726/2, then PointBorder's +20 center offset (XML:1927-1965).
    let left = 409.0 - width / 2.0;
    let shield_left = left + width + 3.0;
    let mut content = label(
        "AchievementFrameHeaderPoints".into(),
        &text,
        // The native font needs 15 units; a shield-height box keeps both centers stable.
        (left, -8.0, width, 20.0),
        WHITE,
        "",
    );
    content.extend(rsx! { texture {
        name: {DynName("AchievementFrameHeaderShield".into())},
        texture_fdid: HEADER_SHIELD_FDID, tex_coords: "0,0.625,0,0.625",
        width: 20.0, height: 20.0, pos_type: "absolute", left: shield_left, top: -7.0,
    } });
    content
}

fn category_panel(ctx: &SharedContext, window: &AchievementWindow) -> Element {
    let mut rows = Vec::new();
    if !window.statistics {
        rows.extend(artwork::category_row(
            "AchievementCategorySummary",
            "Summary",
            "achievement:summary",
            0.0,
            0,
            window.selected_category.is_none(),
        ));
    }
    let first_y = if window.statistics { 0.0 } else { 24.0 };
    let tree = window.category_tree();
    for (index, (category, depth)) in tree.iter().enumerate() {
        rows.extend(artwork::category_row(
            &format!("AchievementCategory{}", category.category_id),
            &category.name,
            &format!("achievement:category:{}", category.category_id),
            first_y + index as f32 * 24.0,
            *depth,
            Some(category.category_id) == window.selected_category,
        ));
    }
    artwork::scroll_pane(
        ctx,
        "AchievementCategoriesScrollBox",
        rows,
        (21.0, 24.0, 175.0, 451.0),
        first_y + tree.len() as f32 * 24.0,
    )
}

// Retail AchievementUI.lua:28,2334-2512: latest three, overall progress, authored summary categories.
fn summary_panel(window: &AchievementWindow) -> Element {
    if window.guild {
        return label(
            "AchievementGuildSummaryUnavailable".into(),
            "Guild achievement progress is not available from this server.",
            (234.0, 90.0, 470.0, 60.0),
            WHITE,
            "",
        );
    }
    let Some(summary) = &window.summary else {
        return label(
            "AchievementSummaryLoading".into(),
            "Loading Summary...",
            (234.0, 90.0, 470.0, 30.0),
            WHITE,
            "",
        );
    };
    let totals = format!("{} / {}", summary.completed, summary.total);
    let mut caption = label(
        "AchievementSummaryCompletedLabel".into(),
        "Achievements completed",
        (6.0, 0.0, 320.0, 21.0),
        GOLD,
        "",
    );
    caption.extend(rsx! {fontstring {name:"AchievementSummaryTotalText",text:totals.as_str(),width:153.0,height:21.0,
        left:330.0,top:0.0,pos_type:"absolute",justify_h:"RIGHT",font:GameFont::FrizQuadrata,font_size:12.0,font_color:WHITE,
    }});
    let mut content = summary_progress_bar(
        "AchievementSummaryTotal",
        summary.completed,
        summary.total,
        (238.5, 251.0, 488.0, 21.0),
        caption,
    );
    content.extend(artwork::section_header(
        "AchievementSummaryRecentHeader",
        "Recent Achievements",
        65.0,
    ));
    for (index, row) in summary.recent.iter().enumerate() {
        content.extend(summary_achievement_row(row, 83.0 + index as f32 * 45.0));
    }
    if summary.recent.is_empty() {
        content.extend(label(
            "AchievementSummaryEmpty".into(),
            "No achievements earned yet",
            (234.0, 110.0, 470.0, 30.0),
            WHITE,
            "",
        ));
    }
    content.extend(summary_category_counts(window, summary));
    content
}

fn summary_category_counts(window: &AchievementWindow, summary: &AchievementSummary) -> Element {
    const SUMMARY_CATEGORIES: [u32; 11] = [92, 96, 97, 15522, 95, 168, 169, 201, 155, 15117, 15246];
    let mut content = artwork::section_header(
        "AchievementSummaryCategoriesHeader",
        "Progress Overview",
        225.0,
    );
    for (index, id) in SUMMARY_CATEGORIES.into_iter().enumerate() {
        let Some(category) = window.categories.get(&id) else {
            continue;
        };
        let counts = summary.categories.iter().find(|row| row.category_id == id);
        let (completed, total) = counts.map_or((0, 0), |row| (row.completed, row.total));
        let text = format!("{completed} / {total}");
        let action = format!("achievement:category:{id}");
        let mut caption = label(
            format!("AchievementSummaryCategory{id}"),
            &category.name,
            (6.0, 0.0, 152.0, 21.0),
            GOLD,
            &action,
        );
        caption.extend(rsx! {fontstring {name:{DynName(format!("AchievementSummaryCategory{id}Count"))},text:text.as_str(),
            width:70.0,height:21.0,left:159.0,top:0.0,pos_type:"absolute",justify_h:"RIGHT",font:GameFont::FrizQuadrata,
            font_size:12.0,font_color:WHITE,onclick:action.as_str(),
        }});
        let x = 238.5 + (index % 2) as f32 * 255.0;
        let y = 285.0 + (index / 2) as f32 * 34.0;
        content.extend(summary_progress_bar(
            &format!("AchievementSummaryCategory{id}"),
            completed,
            total,
            (x, y, 234.0, 21.0),
            caption,
        ));
    }
    content
}

fn filter_controls(window: &AchievementWindow, summary: bool) -> Element {
    if summary || window.statistics || matches!(window.selected_category, Some(81 | 15093)) {
        return Vec::new();
    }
    let title = match window.filter {
        AchievementFilter::All => "All",
        AchievementFilter::Completed => "Completed",
        AchievementFilter::Incomplete => "Incomplete",
    };
    use crate::menu_primitives::*;
    // HeaderDetails right -23; Filters TOPRIGHT -8,-6; dropdown topPadding 6.
    let left = if window.comparison.is_some() {
        745.0
    } else {
        625.0
    };
    let mut content = dropdown_button_with_state(
        DropdownButton {
            frame_name: "AchievementFilterDropdown",
            label_name: "AchievementFilterText",
            arrow_name: "AchievementFilterArrow",
            text: title,
            width: 112.0,
            height: 22.0,
            x: left,
            y: -31.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("achievement:filter_menu"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: window.filter_menu,
            ..Default::default()
        },
    );
    if window.filter_menu {
        let mut choices = Vec::new();
        for (index, (text, action)) in [
            ("All", "all"),
            ("Completed", "completed"),
            ("Incomplete", "incomplete"),
        ]
        .into_iter()
        .enumerate()
        {
            let name = format!("AchievementFilter{action}");
            let action = format!("achievement:filter:{action}");
            choices.extend(menu_element(
                ContextMenuItem {
                    name: &name,
                    label: text,
                    action: &action,
                },
                MenuElementKind::Radio {
                    checked: title == text,
                },
                true,
                (8.0, 8.0 + index as f32 * 20.0, 112.0, 20.0),
            ));
        }
        let background = crate::selection_list::menu_background(
            "AchievementFilterMenuBackground".into(),
            (128.0, 83.0),
        );
        content.extend(
            rsx! {r#frame {name:"AchievementFilterMenu",width:128.0,height:83.0,
                strata:FrameStrata::Dialog,frame_level:30.0,mouse_enabled:true,
            pos_type:"absolute",left,top:53.0, {background} {choices}
            }},
        );
    }
    content
}

fn summary_progress_bar(
    name: &str,
    current: u32,
    total: u32,
    rect: (f32, f32, f32, f32),
    caption: Element,
) -> Element {
    let fraction = if total == 0 {
        0.0
    } else {
        current.min(total) as f32 / total as f32
    };
    artwork::progress(name, fraction, rect, caption)
}

// SummaryAchievementTemplate is 48 high (Mainline XML:1138-1139).
fn summary_achievement_row(entry: &AchievementCatalogEntry, y: f32) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let ink = if thread_skin() == ActiveSkin::Modern {
        "0.1,0.08,0.05,1"
    } else {
        WHITE
    };
    let mut content = artwork::texture(
        &format!("{prefix}Background"),
        130661,
        "0,1,0,0.25",
        (3.0, 3.0, 477.0, 42.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}TitleBar"),
        452109,
        "0,0.9765625,0.66015625,0.73828125",
        (5.0, 4.0, 473.0, 20.0),
    ));
    content.extend(art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (9.0, 6.0, 36.0, 36.0),
    ));
    content.extend(art(
        format!("{prefix}IconFrame"),
        130656,
        (3.0, 2.0, 46.0, 46.0),
    ));
    content.extend(art(
        format!("{prefix}Shield"),
        130666,
        (425.0, 4.0, 48.0, 48.0),
    ));
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Name"))},text:entry.name.as_str(),
        width:260.0,height:20.0,left:111.5,top:4.0,pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:12.0,font_color:WHITE,onclick:action.as_str(),
    }});
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Description"))},text:entry.description.as_str(),
        width:380.0,height:13.0,left:51.5,top:30.0,pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:10.0,font_color:ink,onclick:action.as_str(),
    }});
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Points"))},text:entry.points.to_string().as_str(),
        width:40.0,height:26.0,left:429.0,top:12.0,pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:12.0,font_color:GOLD,onclick:action.as_str(),
    }});
    crate::selection_list::row(
        format!("{prefix}SummaryRow"),
        &action,
        true,
        (241.0, y, 483.0, 48.0),
        (&crate::selection_list::AUTHORED, false),
        content,
    )
}

fn browse_controls(window: &AchievementWindow) -> Element {
    let mut content = artwork::tab(
        "AchievementTabAchievements",
        "Achievements",
        "achievement:achievements",
        !window.statistics && !window.guild,
        17.0,
        115.0,
    );
    content.extend(artwork::tab(
        "AchievementTabGuild",
        "Guild",
        "achievement:guild",
        window.guild,
        136.0,
        70.0,
    ));
    content.extend(artwork::tab(
        "AchievementTabStatistics",
        "Statistics",
        "achievement:statistics",
        window.statistics,
        210.0,
        115.0,
    ));
    content.extend(artwork::search_box(&window.search_text));
    content
}

/// Mainline AchievementStatTemplateMixin:Init uses additive half-alpha on every second row.
pub fn apply_achievement_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    // AchievementCategoryTemplate HighlightTexture uses ADD, not an opaque green overlay.
    // Selection can be outside the current category tree; style the emitted textures,
    // not a second reconstruction of which categories the tree contains.
    let highlights: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| frame.name.as_ref())
        .filter(|name| name.starts_with("AchievementCategory") && name.ends_with("Highlight"))
        .cloned()
        .collect();
    for name in highlights {
        set_additive_texture(registry, &name, 1.0);
    }
    let alternate_stripes: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_ref()?;
            let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = frame.widget_data.as_ref()
            else {
                return None;
            };
            (name.starts_with("Achievement")
                && name.ends_with("Stripe")
                && texture.tex_coords[2] == 0.375)
                .then(|| name.clone())
        })
        .collect();
    for name in alternate_stripes {
        set_additive_texture(registry, &name, 0.5);
    }
}

fn set_additive_texture(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    name: &str,
    alpha: f32,
) {
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::widgets::texture::BlendMode;
    let id = registry
        .get_by_name(name)
        .expect("rendered achievement texture");
    let frame = registry.get_mut(id).expect("achievement texture frame");
    frame.alpha = alpha;
    let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut() else {
        panic!("achievement texture");
    };
    texture.blend_mode = BlendMode::Additive;
}

fn statistic_row(entry: &AchievementCatalogEntry, y: f32, index: usize) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let stripe = index % 2;
    let uv = if stripe == 0 {
        "0,1,0.1875,0.3671875"
    } else {
        "0,1,0.375,0.5390625"
    };
    let mut content = artwork::texture(
        &format!("{prefix}Stripe"),
        235411,
        uv,
        (0.0, y, 500.0, 24.0),
    );
    content.extend(label(
        format!("{prefix}Name"),
        &entry.name,
        (5.0, y, 380.0, STATISTIC_ROW_HEIGHT),
        WHITE,
        &action,
    ));
    let value = entry.statistic_value.map_or_else(
        || "Unevaluated".into(),
        crate::damage_meter_data::break_up_large_number,
    );
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Statistic"))},text:value.as_str(),
        width:106.0,height:STATISTIC_ROW_HEIGHT,left:389.0,top:y,pos_type:"absolute",justify_h:"RIGHT",
        font:GameFont::FrizQuadrata,font_size:12.0,font_color:WHITE,onclick:action.as_str(),
    }});
    content
}

fn achievement_row_height(entry: &AchievementCatalogEntry, expanded: bool, offset: usize) -> f32 {
    if !expanded {
        return 84.0;
    }
    let lines = entry.criteria.len().saturating_sub(offset).min(CRITERIA);
    let paging = entry.criteria.len() > CRITERIA || entry.next_criteria_id.is_some();
    88.0 + lines as f32 * 18.0 + if paging { 22.0 } else { 6.0 }
}

fn achievement_rows(ctx: &SharedContext, window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    let mut y = 0.0;
    for (index, entry) in window.displayed_entries().into_iter().enumerate() {
        if entry.flags & 1 != 0 {
            content.extend(statistic_row(entry, y, index));
            y += STATISTIC_ROW_HEIGHT;
            continue;
        }
        let expanded = Some(entry.achievement_id) == window.expanded_achievement;
        let criteria = if expanded {
            criteria_panel(window)
        } else {
            Vec::new()
        };
        let height = achievement_row_height(entry, expanded, window.criterion_offset);
        let children = achievement_row(entry, 0.0, expanded, height);
        let mut children = children;
        children.extend(criteria);
        content.extend(crate::selection_list::row(
            format!("Achievement{}Row", entry.achievement_id),
            &format!("achievement:row:{}", entry.achievement_id),
            true,
            (0.0, y, 500.0, height),
            (&crate::selection_list::AUTHORED, false),
            children,
        ));
        y += height + 1.0;
    }
    if window.displayed_entries().is_empty() {
        content.extend(label(
            "AchievementEmpty".into(),
            empty_message(window),
            (12.0, 30.0, 470.0, 30.0),
            WHITE,
            "",
        ));
    }
    artwork::scroll_pane(
        ctx,
        ROWS_SCROLLBOX,
        content,
        (222.0, 58.0, 500.0, 417.0),
        y.max(417.0),
    )
}

fn empty_message(window: &AchievementWindow) -> &'static str {
    if window.search_text.trim().chars().count() >= 3 {
        return if window.search_results.loaded {
            "No results"
        } else {
            "Searching..."
        };
    }
    if matches!(window.selected_category, Some(81 | 15093)) {
        return "Feats of Strength recognize exceptional accomplishments. Only earned feats are shown.";
    }
    if window.guild {
        return "Guild achievement progress is not available from this server.";
    }
    if let Some(achievement_id) = window
        .selected_achievement
        .filter(|_| window.selected_category.is_none())
    {
        return if window
            .pending
            .contains(&QueryAchievementCatalog::Achievement { achievement_id })
        {
            "Loading achievement..."
        } else {
            "Achievement unavailable"
        };
    }
    let Some(category) = window.selected_category else {
        return "Select a category";
    };
    if window.pages.get(&category).is_some_and(|page| page.loaded) {
        "No achievements in this category"
    } else {
        "Loading achievements..."
    }
}

fn criteria_panel(window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    if let Some(entry) = window.selected().filter(|entry| entry.flags & 1 == 0) {
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
            let name = format!(
                "Achievement{}Criterion{}",
                entry.achievement_id, line.tree_id
            );
            let y = 88.0 + index as f32 * 18.0;
            let progress_bar = line.progress_supported && line.required > 1;
            let color = if progress_bar || thread_skin() == ActiveSkin::Forever {
                WHITE
            } else {
                "0.1,0.08,0.05,1"
            };
            let caption = label(name.clone(), &text, (4.0, 0.0, 396.0, 18.0), color, "");
            if progress_bar {
                let fraction = line.current.min(line.required) as f32 / line.required as f32;
                content.extend(artwork::criterion_progress(
                    &name,
                    fraction,
                    (80.0, y, 398.0, 16.0),
                    caption,
                ));
            } else {
                content.extend(rsx! { r#frame { name:{DynName(format!("{name}Line"))},
                    width:398.0,height:18.0,left:80.0,top:y,pos_type:"absolute", {caption}
                } });
            }
        }
        let tracked = window
            .tracked
            .iter()
            .any(|row| row.achievement_id == entry.achievement_id);
        if !entry.earned {
            content.extend(artwork::track_check(entry.achievement_id, tracked));
        }
        if let Some(error) = &window.tracking_error {
            content.extend(label(
                "AchievementTrackingError".into(),
                error,
                (80.0, 84.0, 285.0, 20.0),
                WHITE,
                "",
            ));
        }
        if entry.criteria.len() > CRITERIA || entry.next_criteria_id.is_some() {
            content.extend(panel_button(
                "AchievementCriteriaPrev".into(),
                "Previous criteria",
                "achievement:criteria_prev",
                window.criterion_offset > 0,
                (
                    80.0,
                    88.0 + entry
                        .criteria
                        .len()
                        .saturating_sub(window.criterion_offset)
                        .min(CRITERIA) as f32
                        * 18.0,
                    150.0,
                    18.0,
                ),
            ));
            content.extend(panel_button(
                "AchievementCriteriaNext".into(),
                "More criteria",
                "achievement:more_criteria",
                window.criterion_offset + CRITERIA < entry.criteria.len()
                    || entry.next_criteria_id.is_some(),
                (
                    336.0,
                    88.0 + entry
                        .criteria
                        .len()
                        .saturating_sub(window.criterion_offset)
                        .min(CRITERIA) as f32
                        * 18.0,
                    150.0,
                    18.0,
                ),
            ));
        }
    }
    content
}

fn achievement_row(
    entry: &AchievementCatalogEntry,
    y: f32,
    expanded: bool,
    height: f32,
) -> Element {
    let prefix = format!("Achievement{}", entry.achievement_id);
    let action = format!("achievement:row:{}", entry.achievement_id);
    let modern = thread_skin() == ActiveSkin::Modern;
    let ink = if modern { "0.1,0.08,0.05,1.0" } else { WHITE };
    // Mainline AchievementTemplate Label uses GameFontHighlightMedium on its red title band.
    let title_color = WHITE;
    let date_color = if modern { ink } else { GREY };
    let mut content = art(
        format!("{prefix}Background"),
        if entry.earned { 130661 } else { 130660 },
        (3.0, y + 3.0, 494.0, height - 6.0),
    );
    content.extend(artwork::texture(
        &format!("{prefix}TitleBar"),
        452109,
        "0,0.9765625,0.66015625,0.73828125",
        (5.0, y + 4.0, 494.0, 24.0),
    ));
    content.extend(art(
        format!("{prefix}Icon"),
        entry.icon_fdid,
        (13.0, y + 11.0, 50.0, 50.0),
    ));
    content.extend(art(
        format!("{prefix}IconFrame"),
        130656,
        (1.0, y + 1.0, 72.0, 72.0),
    ));
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Name"))},text:entry.name.as_str(),
        width:320.0,height:20.0,left:90.0,top:{y+6.0},pos_type:"absolute",justify_h:"CENTER",
        font:GameFont::FrizQuadrata,font_size:12.0,font_color:title_color,onclick:action.as_str(),
    }});
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Description"))},text:entry.description.as_str(),
        width:330.0,height:34.0,left:85.0,top:{y+30.0},pos_type:"absolute",justify_h:"CENTER",
        font:GameFont::FrizQuadrata,font_size:12.0,font_color:ink,onclick:action.as_str(),
    }});
    content.extend(art(
        format!("{prefix}Shield"),
        130666,
        (428.0, y + 6.0, 66.0, 64.0),
    ));
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Points"))},text:entry.points.to_string().as_str(),
        width:42.0,height:16.0,left:439.0,top:{y+26.0},pos_type:"absolute",justify_h:"CENTER",font:GameFont::FrizQuadrata,
        font_size:12.0,font_color:GOLD,onclick:action.as_str(),
    }});
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
    content.extend(rsx! {fontstring {name:{DynName(format!("{prefix}Date"))},text:date.as_str(),
        width:190.0,height:14.0,left:300.0,top:{y+64.0},pos_type:"absolute",font:GameFont::FrizQuadrata,
        font_size:10.0,font_color:date_color,justify_h:"RIGHT",onclick:action.as_str(),
    }});
    if !entry.criteria.is_empty() || entry.next_criteria_id.is_some() {
        let uv = match (expanded, entry.earned) {
            (false, true) => "0,0.5,0,0.25",
            (false, false) => "0.5,1,0,0.25",
            (true, true) => "0,0.5,0.25,0.5",
            (true, false) => "0.5,1,0.25,0.5",
        };
        content.extend(artwork::texture(
            &format!("{prefix}PlusMinus"),
            235404,
            uv,
            (72.0, y + 9.0, 15.0, 15.0),
        ));
    }
    content
}

/// Own-context chat links carry only a real catalog ID; no fabricated GUID/comparison facts.
pub fn achievement_link(id: u32, name: &str) -> String {
    format!("|Hachievement:{id}|h[{name}]|h")
}

/// Mainline AlertFrameSystems.xml:173-330 and lua:348-388: authored atlas regions and anchors.
pub fn achievement_toast_screen(ctx: &SharedContext) -> Element {
    use crate::quest_art::atlas_texture;
    let toast = ctx
        .get::<shared::protocol::AchievementToastSnapshot>()
        .expect("toast payload");
    let action = format!("achievement:open:{}", toast.achievement_id);
    let mut content = atlas_texture(
        "AchievementToastBackground".into(),
        &ALERT_BACKGROUND,
        (-5.0, 0.0, 310.0, 101.0),
    );
    content.extend(art(
        "AchievementToastIcon".into(),
        toast.icon_fdid,
        (9.0, 26.5, 52.0, 52.0),
    ));
    content.extend(atlas_texture(
        "AchievementToastIconFrame".into(),
        &ALERT_ICON_FRAME,
        (-3.0, 15.5, 74.0, 74.0),
    ));
    let shield = if toast.points == 0 {
        &ALERT_NO_POINTS
    } else {
        &ALERT_SHIELD
    };
    let shield_width = shield.rect.1 - shield.rect.0;
    let shield_height = shield.rect.3 - shield.rect.2;
    content.extend(atlas_texture(
        "AchievementToastShield".into(),
        shield,
        (293.0 - shield_width, 21.0, shield_width, shield_height),
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
