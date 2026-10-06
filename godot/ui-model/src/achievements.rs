//! On-demand AchievementFrame: ordered category tree, cached cursor pages, live refresh.
//! Retail AchievementUI.lua:572 (categories), :1814 (rows), :1952 (criteria).
use crate::quest_art::{DynName, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::{
    AchievementCatalogEntry, AchievementCatalogPage, AchievementCategoryEntry,
    AchievementCriterionLine, AchievementStateUpdate, QueryAchievementCatalog,
};
use std::collections::BTreeMap;
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const OPEN_ACTION: &str = "micro:AchievementMicroButton";
pub const FRAME: &str = "AchievementFrame";
const ROWS: usize = 3;
const CATEGORIES: usize = 18;
const CRITERIA: usize = 6;
/// Local Retail art: UI-Achievement-Category-Background, AchievementBackground, Shield.
pub const ART: &[u32] = &[130652, 235397, 130665, 130650, HEADER_SHIELD_FDID];
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
    categories_loaded: bool,
    pending: Vec<QueryAchievementCatalog>,
}
impl AchievementWindow {
    pub fn action(&mut self, action: &str) -> Vec<QueryAchievementCatalog> {
        let mut requests = Vec::new();
        match action {
            OPEN_ACTION => self.toggle(&mut requests),
            "achievement:close" => self.visible = false,
            "achievement:categories_next" => {
                self.category_offset =
                    (self.category_offset + CATEGORIES).min(self.categories.len().saturating_sub(1))
            }
            "achievement:categories_prev" => {
                self.category_offset = self.category_offset.saturating_sub(CATEGORIES)
            }
            "achievement:rows_prev" => {
                self.row_offset = self.row_offset.saturating_sub(ROWS);
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
    fn toggle(&mut self, requests: &mut Vec<QueryAchievementCatalog>) {
        self.visible = !self.visible;
        if !self.visible {
            return;
        }
        if !self.categories_loaded {
            requests.push(QueryAchievementCatalog::Categories { after_id: 0 });
        }
        if let Some(category_id) = self.selected_category {
            if !self.pages.get(&category_id).is_some_and(|page| page.loaded) {
                requests.push(QueryAchievementCatalog::Category {
                    category_id,
                    after_id: 0,
                });
            }
        }
    }
    fn select_action(&mut self, action: &str, requests: &mut Vec<QueryAchievementCatalog>) {
        if let Some(id) = action
            .strip_prefix("achievement:category:")
            .and_then(|id| id.parse().ok())
        {
            if !self.categories.contains_key(&id) {
                return;
            }
            self.selected_category = Some(id);
            self.selected_achievement = None;
            self.row_offset = 0;
            self.criterion_offset = 0;
            if !self.pages.get(&id).is_some_and(|page| page.loaded) {
                requests.push(QueryAchievementCatalog::Category {
                    category_id: id,
                    after_id: 0,
                });
            } else {
                self.select_first();
            }
        } else if let Some(id) = action
            .strip_prefix("achievement:row:")
            .and_then(|id| id.parse().ok())
        {
            if self
                .entries()
                .iter()
                .any(|entry| entry.achievement_id == id)
            {
                self.selected_achievement = Some(id);
                self.criterion_offset = 0;
            }
        }
    }
    fn advance_rows(&mut self, requests: &mut Vec<QueryAchievementCatalog>) {
        let Some(category_id) = self.selected_category else {
            return;
        };
        let Some(page) = self.pages.get(&category_id) else {
            return;
        };
        if self.row_offset + ROWS < page.entries.len() {
            self.row_offset += ROWS;
            self.select_first();
        } else if let Some(after_id) = page.next {
            requests.push(QueryAchievementCatalog::Category {
                category_id,
                after_id,
            });
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
                self.pending.push(*request);
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
                self.cache_achievements(category_id, achievements, next_id, first);
                Vec::new()
            }
            AchievementCatalogPage::Criteria {
                achievement_id,
                criteria,
                next_id,
            } => {
                self.cache_criteria(achievement_id, criteria, next_id);
                Vec::new()
            }
        };
        self.enqueue(requests)
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
        for cache in self.pages.values_mut() {
            if let Some(entry) = cache
                .entries
                .iter_mut()
                .find(|entry| entry.achievement_id == achievement_id)
            {
                merge_criteria(&mut entry.criteria, &criteria);
                entry.next_criteria_id = next;
            }
        }
    }
    pub fn refresh(&mut self, _update: &AchievementStateUpdate) -> Vec<QueryAchievementCatalog> {
        for page in self.pages.values_mut() {
            page.loaded = false;
        }
        let requests = if self.visible {
            self.selected_category
                .map(|category_id| QueryAchievementCatalog::Category {
                    category_id,
                    after_id: 0,
                })
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        // A live change needs a post-change response even if an older first page is
        // already in flight. Same-type reliable replies consume these in send order.
        self.pending.extend(requests.iter().copied());
        requests
    }
    fn select_first(&mut self) {
        self.row_offset = self.row_offset.min(self.entries().len().saturating_sub(1));
        self.selected_achievement = self
            .entries()
            .get(self.row_offset)
            .map(|entry| entry.achievement_id);
        self.criterion_offset = 0;
    }
    pub fn entries(&self) -> &[AchievementCatalogEntry] {
        self.selected_category
            .and_then(|id| self.pages.get(&id))
            .map_or(&[], |page| &page.entries)
    }
    pub fn selected(&self) -> Option<&AchievementCatalogEntry> {
        self.entries()
            .iter()
            .find(|entry| Some(entry.achievement_id) == self.selected_achievement)
    }
    pub fn category_tree(&self) -> Vec<(&AchievementCategoryEntry, usize)> {
        let mut rows = Vec::new();
        self.append_children(-1, 0, &mut rows);
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
            .filter(|row| row.parent_id == parent)
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
    let points: u32 = window
        .pages
        .values()
        .flat_map(|page| &page.entries)
        .filter(|entry| entry.earned)
        .map(|entry| entry.points)
        .sum();
    content.extend(points_header(points));
    content.extend(category_panel(window));
    content.extend(achievement_rows(window));
    content.extend(criteria_panel(window));
    rsx! { r#frame {
        name: {DynName(FRAME.into())}, width: 820.0, height: 600.0, strata: FrameStrata::Dialog,
        hidden: hide, mouse_enabled: true, pos_type: "absolute", left: 32.0, top: 100.0,
        {content}
    } }
}
/// Retail Mainline/Blizzard_AchievementUI.lua:298: bare formatted points;
/// XML:1973-1984: white text with a 20x20 shield three units to its right.
fn points_header(points: u32) -> Element {
    let text = crate::damage_meter_data::break_up_large_number(u64::from(points));
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
            (x, 72.0 + index as f32 * 24.0, 210.0 - x, 22.0),
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

fn achievement_rows(window: &AchievementWindow) -> Element {
    let mut content = Vec::new();
    for (index, entry) in window
        .entries()
        .iter()
        .skip(window.row_offset)
        .take(ROWS)
        .enumerate()
    {
        content.extend(achievement_row(
            entry,
            68.0 + index as f32 * 96.0,
            Some(entry.achievement_id) == window.selected_achievement,
        ));
    }
    if window.entries().is_empty() {
        let message = if window.selected_category.is_none() {
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
    let more = window.row_offset + ROWS < window.entries().len()
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
    if let Some(entry) = window.selected() {
        content.extend(label(
            "AchievementCriteriaTitle".into(),
            &format!("Criteria — {}", entry.name),
            (240.0, 397.0, 546.0, 22.0),
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

/// Existing wire toast payload; the native Godot host did not previously subscribe to it.
pub fn achievement_toast_screen(ctx: &SharedContext) -> Element {
    let toast = ctx
        .get::<shared::protocol::AchievementToastSnapshot>()
        .expect("toast payload");
    let mut content = art(
        "AchievementToastBackground".into(),
        130650,
        (0.0, 0.0, 360.0, 80.0),
    );
    content.extend(label(
        "AchievementToastTitle".into(),
        "Achievement earned",
        (64.0, 10.0, 280.0, 20.0),
        GOLD,
        "",
    ));
    content.extend(label(
        "AchievementToastName".into(),
        &toast.name,
        (64.0, 33.0, 280.0, 22.0),
        WHITE,
        "",
    ));
    content.extend(label(
        "AchievementToastPoints".into(),
        &format!("{} points", toast.points),
        (64.0, 57.0, 280.0, 20.0),
        GOLD,
        "",
    ));
    rsx! { r#frame { name: "AchievementToast", width: 360.0, height: 80.0, pos_type: "absolute", left: "50%", margin_left: -180.0, bottom: 170.0, {content} } }
}
