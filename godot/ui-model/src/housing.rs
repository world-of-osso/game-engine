//! Retail HousingDashboardCatalog.lua search/category/preview decisions.
use game_engine_core::housing::{Decor, HousingCatalog};
use std::sync::Arc;

pub const SEARCH_FIELD: &str = "HousingCatalogSearchBox";
pub const VISIBLE_ENTRIES: usize = 12;
pub const COLUMNS: usize = 3;
/// HousingCatalogConstantsDocumentation.lua HOUSING_CATALOG_ALL_CATEGORY_ID.
pub const ALL_CATEGORY: u32 = 18;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DashboardTab {
    #[default]
    HouseInfo,
    Catalog,
    Collection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HousingDashboard {
    pub catalog: Arc<HousingCatalog>,
    pub search: String,
    pub category: Option<u32>,
    pub subcategory: Option<u32>,
    pub offset: usize,
    pub selection: Option<u32>,
    pub tab: DashboardTab,
    pub filters_open: bool,
}

impl HousingDashboard {
    pub fn new(catalog: Arc<HousingCatalog>) -> Self {
        let selection = catalog.decor.first().map(|decor| decor.id);
        Self {
            catalog,
            search: String::new(),
            category: None,
            subcategory: None,
            offset: 0,
            selection,
            tab: DashboardTab::HouseInfo,
            filters_open: false,
        }
    }

    pub fn matching(&self) -> Vec<&Decor> {
        let search = self.search.to_lowercase();
        self.catalog
            .decor
            .iter()
            .filter(|decor| {
                let category = self.category.is_none_or(|id| {
                    self.catalog.subcategories.iter().any(|subcategory| {
                        subcategory.category_id == id
                            && decor.subcategories.contains(&subcategory.id)
                    })
                });
                let subcategory = self
                    .subcategory
                    .is_none_or(|id| decor.subcategories.contains(&id));
                category && subcategory && decor.name.to_lowercase().contains(&search)
            })
            .collect()
    }

    pub fn visible(&self) -> Vec<&Decor> {
        self.matching()
            .into_iter()
            .skip(self.offset)
            .take(VISIBLE_ENTRIES)
            .collect()
    }

    pub fn selected(&self) -> Option<&Decor> {
        let id = self.selection?;
        self.matching().into_iter().find(|decor| decor.id == id)
    }

    pub fn set_search(&mut self, search: String) {
        self.search = search;
        self.reset_results();
    }

    pub fn set_category(&mut self, id: Option<u32>) {
        self.category = id.filter(|id| *id != ALL_CATEGORY);
        self.subcategory = None;
        self.reset_results();
    }

    pub fn set_subcategory(&mut self, id: Option<u32>) {
        self.subcategory = id;
        self.reset_results();
    }

    fn reset_results(&mut self) {
        self.offset = 0;
        self.selection = self.matching().first().map(|decor| decor.id);
    }

    pub fn select(&mut self, id: u32) -> bool {
        if self.matching().iter().any(|decor| decor.id == id) {
            self.selection = Some(id);
            true
        } else {
            false
        }
    }

    pub fn scroll(&mut self, rows: i32) {
        let count = self.matching().len();
        let last_row = count.saturating_sub(VISIBLE_ENTRIES).div_ceil(COLUMNS) * COLUMNS;
        self.offset = self
            .offset
            .saturating_add_signed(rows as isize * COLUMNS as isize)
            .min(last_row);
    }
}
