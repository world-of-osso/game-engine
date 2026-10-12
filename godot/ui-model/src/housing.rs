//! Retail HousingDashboardCatalog.lua search/category/preview decisions.
use game_engine_core::housing::{Decor, HousingCatalog};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HousingAuthority {
    pub state: Option<shared::protocol::HousingState>,
    pub error: Option<String>,
}

impl HousingAuthority {
    pub fn receive(&mut self, reply: shared::protocol::HousingReply) -> Result<(), String> {
        if reply.state.build_key != game_engine_core::housing::RETAIL_BUILD_KEY {
            self.state = None;
            let message = match reply.error {
                Some(message) => message,
                None => "Housing snapshot does not match the supported Retail build.".into(),
            };
            self.error = Some(message.clone());
            return Err(message);
        }
        self.state = Some(reply.state);
        self.error = reply.error;
        Ok(())
    }

    pub fn quantity(&self, decor_id: u32) -> Option<u32> {
        self.state.as_ref().map(|state| {
            state
                .stored
                .iter()
                .find(|decor| decor.decor_id == decor_id)
                .map_or(0, |decor| decor.quantity)
        })
    }

    /// Retail HousingCatalogEntryInfo.totalNumPlaced across the account's houses.
    pub fn placed(&self, decor_id: u32) -> Option<u32> {
        self.state.as_ref().map(|state| {
            state
                .placed
                .iter()
                .find(|decor| decor.decor_id == decor_id)
                .map_or(0, |decor| decor.quantity)
        })
    }

    /// Blizzard_HousingCatalogUtil.GetEntryTotalOwned: stored + redeemable + placed.
    /// Nothing is lazily redeemable without an authenticated grant source.
    pub fn owned(&self, decor_id: u32) -> Option<u32> {
        Some(self.quantity(decor_id)? + self.placed(decor_id)?)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Retail HousingCatalogFilters collection radio (HOUSING_CATALOG_FILTERS_COLLECTION_*).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CollectionFilter {
    #[default]
    All,
    Collected,
    Uncollected,
}

impl CollectionFilter {
    /// Unconfirmed ownership is neither collected nor uncollected.
    fn allows(self, owned: Option<u32>) -> bool {
        match self {
            Self::All => true,
            Self::Collected => owned.is_some_and(|owned| owned > 0),
            Self::Uncollected => owned == Some(0),
        }
    }
}

pub const SEARCH_FIELD: &str = "HousingCatalogSearchBox";
pub const VISIBLE_ENTRIES: usize = 12;
pub const COLUMNS: usize = 3;
/// HousingCatalogConstantsDocumentation.lua HOUSING_CATALOG_ALL_CATEGORY_ID.
pub const ALL_CATEGORY: u32 = 18;
/// HOUSING_CATALOG_FEATURED_CATEGORY_ID; Retail never expands it into subcategories.
pub const FEATURED_CATEGORY: u32 = 17;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DashboardTab {
    #[default]
    HouseInfo,
    Catalog,
    Collection,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum HousingSurface {
    #[default]
    Dashboard,
    Finder,
    Cornerstone(u32),
    Editor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HousingDashboard {
    pub surface: HousingSurface,
    pub neighborhood: u32,
    pub pending: bool,
    pub catalog: Arc<HousingCatalog>,
    pub search: String,
    pub category: Option<u32>,
    pub subcategory: Option<u32>,
    pub offset: usize,
    pub selection: Option<u32>,
    pub tab: DashboardTab,
    pub filters_open: bool,
    pub collection: CollectionFilter,
    pub dyeable_only: bool,
    pub editor: crate::housing_editor::EditorInteraction,
}

impl HousingDashboard {
    pub fn new(catalog: Arc<HousingCatalog>) -> Self {
        let selection = catalog.decor.first().map(|decor| decor.id);
        Self {
            surface: HousingSurface::Dashboard,
            neighborhood: 1,
            pending: false,
            catalog,
            search: String::new(),
            category: None,
            subcategory: None,
            offset: 0,
            selection,
            tab: DashboardTab::HouseInfo,
            filters_open: false,
            collection: CollectionFilter::All,
            dyeable_only: false,
            editor: crate::housing_editor::EditorInteraction::default(),
        }
    }

    pub fn navigation_request(
        &mut self,
        action: &str,
    ) -> Result<Option<shared::protocol::HousingCommand>, String> {
        use shared::protocol::HousingCommand;
        if let Some(value) = action.strip_prefix("housing:access:") {
            let (house, access) = value
                .split_once(':')
                .ok_or("Invalid house access action.")?;
            let house_id = house.parse().map_err(|_| "Invalid house identity.")?;
            let allowed = match access {
                "on" => true,
                "off" => false,
                _ => return Err("Invalid house access setting.".into()),
            };
            return Ok(Some(HousingCommand::SetHouseAccessAnyone {
                house_id,
                allowed,
            }));
        }
        if action == "housing:leave" {
            return Ok(Some(HousingCommand::LeaveHouse));
        }
        if let Some(value) = action.strip_prefix("housing:home:") {
            let house_id = value
                .parse::<u64>()
                .map_err(|_| "Invalid house identity.")?;
            return Ok(Some(HousingCommand::EnterHouse { house_id }));
        }
        if action == "housing:finder" {
            self.surface = HousingSurface::Finder;
            return Ok(Some(HousingCommand::ListPlots {
                neighborhood_id: self.neighborhood,
            }));
        }
        if action == "housing:visit" {
            return Ok(Some(HousingCommand::EnterNeighborhood {
                neighborhood_id: self.neighborhood,
            }));
        }
        if let Some(value) = action.strip_prefix("housing:neighborhood:") {
            let id = value
                .parse::<u32>()
                .map_err(|_| "Invalid neighborhood selection.")?;
            if !matches!(id, 1 | 2) {
                return Err("Neighborhood is not in the authenticated shipping catalog.".into());
            }
            self.neighborhood = id;
            return Ok(Some(HousingCommand::ListPlots {
                neighborhood_id: id,
            }));
        }
        Ok(None)
    }

    /// Retail catalog searcher results: search, category, subcategory, Dyeable Only and
    /// collection filters, in catalog order.
    pub fn matching(&self, authority: &HousingAuthority) -> Vec<&Decor> {
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
                category
                    && subcategory
                    && (!self.dyeable_only || decor.dyeable)
                    && self.collection.allows(authority.owned(decor.id))
                    && decor.name.to_lowercase().contains(&search)
            })
            .collect()
    }

    pub fn visible(&self, authority: &HousingAuthority) -> Vec<&Decor> {
        self.matching(authority)
            .into_iter()
            .skip(self.offset)
            .take(VISIBLE_ENTRIES)
            .collect()
    }

    pub fn selected(&self, authority: &HousingAuthority) -> Option<&Decor> {
        let id = self.selection?;
        self.matching(authority)
            .into_iter()
            .find(|decor| decor.id == id)
    }

    /// The focused category when Retail shows its subcategories in place of the
    /// top-level column: neither All nor Featured, with more than one subcategory.
    pub fn expanded_category(&self) -> Option<&game_engine_core::housing::Category> {
        let id = self.category.filter(|id| *id != FEATURED_CATEGORY)?;
        let count = self
            .catalog
            .subcategories
            .iter()
            .filter(|subcategory| subcategory.category_id == id)
            .count();
        (count > 1)
            .then(|| {
                self.catalog
                    .categories
                    .iter()
                    .find(|category| category.id == id)
            })
            .flatten()
    }

    pub fn set_search(&mut self, search: String, authority: &HousingAuthority) {
        self.search = search;
        self.reset_results(authority);
    }

    pub fn set_category(&mut self, id: Option<u32>, authority: &HousingAuthority) {
        self.category = id.filter(|id| *id != ALL_CATEGORY);
        self.subcategory = None;
        self.reset_results(authority);
    }

    pub fn set_subcategory(&mut self, id: Option<u32>, authority: &HousingAuthority) {
        self.subcategory = id;
        self.reset_results(authority);
    }

    pub fn set_collection(&mut self, filter: CollectionFilter, authority: &HousingAuthority) {
        self.collection = filter;
        self.reset_results(authority);
    }

    pub fn toggle_dyeable(&mut self, authority: &HousingAuthority) {
        self.dyeable_only = !self.dyeable_only;
        self.reset_results(authority);
    }

    fn reset_results(&mut self, authority: &HousingAuthority) {
        self.offset = 0;
        self.selection = self.matching(authority).first().map(|decor| decor.id);
    }

    pub fn select(&mut self, id: u32, authority: &HousingAuthority) -> bool {
        if self.matching(authority).iter().any(|decor| decor.id == id) {
            self.selection = Some(id);
            true
        } else {
            false
        }
    }

    pub fn scroll(&mut self, rows: i32, authority: &HousingAuthority) {
        let count = self.matching(authority).len();
        let last_row = count.saturating_sub(VISIBLE_ENTRIES).div_ceil(COLUMNS) * COLUMNS;
        self.offset = self
            .offset
            .saturating_add_signed(rows as isize * COLUMNS as isize)
            .min(last_row);
    }
}

/// Retail GlobalStrings for CollectableSource text (local 12.1.0.69933 GlobalStrings.db2).
/// Retail composes HousingCatalogEntryInfo.sourceText in C++; these are the shipped
/// formats its content tracking uses for the same CollectableSource tables.
const VENDOR_ZONE_FORMAT: &str = "Vendor: {} in {}"; // CONTENT_TRACKING_VENDOR_ZONE_FORMAT
const VENDOR_FORMAT: &str = "Vendor: {}"; // CONTENT_TRACKING_VENDOR_FORMAT
const BOSS_DROP_FORMAT: &str = "Boss Drop: {} in {} ({})"; // CONTENT_TRACKING_BOSS_DROP_FORMAT
const BOSS_DROP_MINIMAL_FORMAT: &str = "Boss Drop: {} in {}"; // ..._BOSS_DROP_MINIMAL_FORMAT

/// Substitutes `values` for the `{}` slots (Lua `%s`), never into an inserted value.
fn fill(format: &str, values: &[&str]) -> String {
    let mut parts = format.split("{}");
    let mut text = parts.next().unwrap_or_default().to_string();
    for (value, part) in values.iter().zip(parts) {
        text.push_str(value);
        text.push_str(part);
    }
    text
}

/// Source lines of `decor`. Vendors are named only by the server's creature query
/// (`names`); a vendor it has not named is left out rather than shown unnamed.
pub fn source_lines(decor: &Decor, names: &std::collections::HashMap<u32, String>) -> Vec<String> {
    use game_engine_core::housing::DecorSource;
    decor
        .sources
        .iter()
        .filter_map(|source| match source {
            DecorSource::Vendor { creature_id, area } => {
                let name = names.get(creature_id)?;
                Some(match area {
                    Some(area) => fill(VENDOR_ZONE_FORMAT, &[name, area]),
                    None => fill(VENDOR_FORMAT, &[name]),
                })
            }
            DecorSource::Encounter {
                encounter,
                instance,
                difficulty,
            } => Some(match difficulty {
                Some(difficulty) => fill(BOSS_DROP_FORMAT, &[encounter, instance, difficulty]),
                None => fill(BOSS_DROP_MINIMAL_FORMAT, &[encounter, instance]),
            }),
        })
        .collect()
}
