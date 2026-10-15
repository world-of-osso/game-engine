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
pub const STORAGE_SEARCH_FIELD: &str = "HouseEditorStorageSearchBox";
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
    /// The roster from the server's reply to using the neighborhood's bulletin board.
    BulletinBoard,
    Editor,
    HouseSettings,
}

/// Retail NeighborhoodRosterMixin column attributes (NEIGHBORHOOD_ROSTER_COLUMN_INFO).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RosterColumn {
    Name,
    Status,
    Plot,
}

/// The bulletin board roster as Retail NeighborhoodRosterMixin keeps it: the server's
/// alphabetical member list and the order of the last column sort.
#[derive(Clone, Debug, PartialEq)]
pub struct BulletinBoard {
    pub neighborhood_id: u32,
    alphabetical: Vec<shared::protocol::NeighborhoodResident>,
    pub residents: Vec<shared::protocol::NeighborhoodResident>,
    sort: RosterColumn,
    reverse: bool,
}

impl BulletinBoard {
    /// SetAlphabeticalSortedMemberList: shown as received, sorted by name.
    pub fn new(roster: shared::protocol::NeighborhoodRoster) -> Self {
        Self {
            neighborhood_id: roster.neighborhood_id,
            residents: roster.residents.clone(),
            alphabetical: roster.residents,
            sort: RosterColumn::Name,
            reverse: false,
        }
    }

    /// SortByColumnIndex: the active column flips direction. Name restores the
    /// alphabetical list; Status puts online residents first; Plot orders by plot.
    pub fn sort_by(&mut self, column: RosterColumn) {
        let reverse = self.sort == column && !self.reverse;
        self.sort = column;
        self.reverse = reverse;
        let direct = |order: std::cmp::Ordering| if reverse { order.reverse() } else { order };
        match column {
            RosterColumn::Name => {
                self.residents = self.alphabetical.clone();
                if reverse {
                    self.residents.reverse();
                }
            }
            RosterColumn::Status => self
                .residents
                .sort_by(|left, right| direct(right.online.cmp(&left.online))),
            RosterColumn::Plot => self
                .residents
                .sort_by(|left, right| direct(left.plot_index.cmp(&right.plot_index))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HousingDashboard {
    pub surface: HousingSurface,
    pub neighborhood: u32,
    pub finder_plot: Option<u32>,
    pub maps: Option<Arc<crate::ui_map_data::UiMapCatalog>>,
    pub pending: bool,
    pub catalog: Arc<HousingCatalog>,
    pub search: String,
    pub search_focused: bool,
    pub storage_search: String,
    pub storage_search_focused: bool,
    pub storage_category: Option<u32>,
    pub storage_subcategory: Option<u32>,
    pub storage_filters_open: bool,
    pub storage_sort_open: bool,
    pub storage_dyeable_only: bool,
    pub storage_alphabetical: bool,
    pub storage_panel: game_engine_core::housing::StoragePanelSettings,
    pub storage_transition: Option<crate::housing_storage::StorageTransition>,
    pub category: Option<u32>,
    pub subcategory: Option<u32>,
    pub offset: usize,
    pub selection: Option<u32>,
    pub tab: DashboardTab,
    pub filters_open: bool,
    pub collection: CollectionFilter,
    pub dyeable_only: bool,
    pub editor: crate::housing_editor::EditorInteraction,
    /// Native account bindings copied into the view; absent keys remain source-unbound.
    pub editor_bindings: game_engine_core::input_bindings_data::InputBindingsData,
    pub placed_list: crate::housing_placed_list::PlacedDecorListState,
    /// The roster from the last bulletin board use; later snapshots do not carry it.
    pub bulletin_board: Option<BulletinBoard>,
    pub settings: Option<crate::housing_settings::HouseSettings>,
}

impl HousingDashboard {
    /// Cleanup ShowDecorInstanceTooltip uses GameTooltip at ANCHOR_CURSOR.
    /// Pet/lock/removability fields are not supplied by the authoritative snapshot.
    pub fn cleanup_hover_tooltip(
        &self,
        authority: &HousingAuthority,
    ) -> Option<crate::game_tooltip::GameTooltip> {
        use crate::game_tooltip::GameTooltip;
        use crate::housing_editor::EditorMode;
        use crate::tooltip_presentation::{TOOLTIP_WHITE, TooltipPresentation};
        if self.surface != HousingSurface::Editor
            || self.editor.mode != EditorMode::Cleanup
            || self.pending
        {
            return None;
        }
        let instance = self.editor.hovered?;
        let layout = authority.state.as_ref()?.layout.as_ref()?;
        let placed = layout
            .decor
            .iter()
            .find(|decor| decor.instance_id == instance)?;
        let source = self
            .catalog
            .decor
            .iter()
            .find(|decor| decor.id == placed.decor_id)?;
        Some(
            GameTooltip::new(
                TooltipPresentation {
                    title: source.name.clone(),
                    title_color: TOOLTIP_WHITE,
                    ..TooltipPresentation::hidden()
                },
                None,
            )
            .at_cursor(),
        )
    }

    pub fn new(catalog: Arc<HousingCatalog>) -> Self {
        let selection = catalog.decor.first().map(|decor| decor.id);
        Self {
            surface: HousingSurface::Dashboard,
            neighborhood: 1,
            finder_plot: None,
            maps: None,
            pending: false,
            catalog,
            search: String::new(),
            search_focused: false,
            storage_search: String::new(),
            storage_search_focused: false,
            storage_category: None,
            storage_subcategory: None,
            storage_filters_open: false,
            storage_sort_open: false,
            storage_dyeable_only: false,
            storage_alphabetical: false,
            storage_panel: Default::default(),
            storage_transition: None,
            category: None,
            subcategory: None,
            offset: 0,
            selection,
            tab: DashboardTab::HouseInfo,
            filters_open: false,
            collection: CollectionFilter::All,
            dyeable_only: false,
            editor: crate::housing_editor::EditorInteraction::default(),
            editor_bindings: Default::default(),
            placed_list: Default::default(),
            bulletin_board: None,
            settings: None,
        }
    }

    /// HouseEditorStorageFrame uses SetStoredOnly(true), independent of the dashboard search.
    pub fn stored_entries<'a>(
        &'a self,
        authority: &'a HousingAuthority,
    ) -> Vec<(&'a shared::protocol::StoredDecor, &'a Decor)> {
        let Some(state) = &authority.state else {
            return Vec::new();
        };
        let search = self.storage_search_query().trim().to_lowercase();
        let mut entries: Vec<_> = state
            .stored
            .iter()
            .filter(|stored| stored.quantity > 0)
            .filter_map(|stored| {
                let decor = self
                    .catalog
                    .decor
                    .iter()
                    .find(|decor| decor.id == stored.decor_id)?;
                let matches = decor.name.to_lowercase().contains(&search)
                    && self.storage_decor_matches(decor)
                    && (!self.storage_dyeable_only || decor.dyeable);
                matches.then_some((stored, decor))
            })
            .collect();
        if self.storage_alphabetical {
            entries.sort_by_cached_key(|(_, decor)| (decor.name.to_lowercase(), decor.id));
        }
        entries
    }

    pub fn select_finder_plot(
        &mut self,
        authority: &HousingAuthority,
        id: u32,
    ) -> Result<(), String> {
        use shared::protocol::HousingPlotOwnerType;
        let plot = authority
            .state
            .as_ref()
            .and_then(|state| {
                state
                    .plots
                    .iter()
                    .find(|plot| plot.plot_id == id && plot.neighborhood_id == self.neighborhood)
            })
            .ok_or("Plot data has not been confirmed by the server.")?;
        if !matches!(
            plot.owner_type,
            HousingPlotOwnerType::None | HousingPlotOwnerType::Friend
        ) {
            return Err("Retail Finder displays only sale and friend plots.".into());
        }
        self.finder_plot = Some(id);
        Ok(())
    }

    pub fn settings_request(
        &mut self,
        authority: &HousingAuthority,
        action: &str,
    ) -> Result<Option<shared::protocol::HousingCommand>, String> {
        use crate::housing_settings::{AccessMode, HouseSettings};
        if let Some(id) = action.strip_prefix("housing:settings:open:") {
            let id = id.parse::<u64>().map_err(|_| "Invalid house identity.")?;
            let house = authority
                .state
                .as_ref()
                .and_then(|state| {
                    state
                        .owned_houses
                        .iter()
                        .find(|house| house.house_id == id && house.can_edit)
                })
                .ok_or("Only a confirmed owner may open House Settings.")?;
            self.settings = Some(HouseSettings::new(id, house.house_access));
            self.surface = HousingSurface::HouseSettings;
            return Ok(None);
        }
        let draft = self
            .settings
            .as_mut()
            .ok_or("House Settings is not open.")?;
        match action {
            "housing:settings:menu" => draft.menu_open = !draft.menu_open,
            "housing:settings:anyone" => draft.select_mode(AccessMode::Anyone),
            "housing:settings:none" => draft.select_mode(AccessMode::NoOne),
            "housing:settings:limited" => draft.select_mode(AccessMode::Limited),
            "housing:settings:save" => return draft.save().map(Some),
            _ => draft.toggle(
                action
                    .strip_prefix("housing:settings:toggle:")
                    .ok_or("Unknown House Settings action.")?,
            )?,
        }
        Ok(None)
    }

    pub fn navigation_request(
        &mut self,
        action: &str,
    ) -> Result<Option<shared::protocol::HousingCommand>, String> {
        use shared::protocol::HousingCommand;
        if action == "housing:leave" {
            return Ok(Some(HousingCommand::LeaveHouse));
        }
        if let Some(column) = action.strip_prefix("housing:roster-sort:") {
            let column = match column {
                "name" => RosterColumn::Name,
                "status" => RosterColumn::Status,
                "plot" => RosterColumn::Plot,
                _ => return Err("Invalid roster column.".into()),
            };
            self.bulletin_board
                .as_mut()
                .ok_or("The bulletin board is not open.")?
                .sort_by(column);
            return Ok(None);
        }
        if let Some(value) = action.strip_prefix("housing:home:") {
            let house_id = value
                .parse::<u64>()
                .map_err(|_| "Invalid house identity.")?;
            return Ok(Some(HousingCommand::EnterHouse { house_id }));
        }
        if action == "housing:finder-back" {
            self.finder_plot = None;
            return Ok(None);
        }
        if action == "housing:finder" {
            self.finder_plot = None;
            self.surface = HousingSurface::Finder;
            return Ok(Some(HousingCommand::ListPlots {
                neighborhood_id: self.neighborhood,
            }));
        }
        if action == "housing:visit-plot" {
            return Ok(Some(HousingCommand::VisitPlot {
                plot_id: self.finder_plot.ok_or("Select an authored plot first.")?,
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
            self.finder_plot = None;
            return Ok(Some(HousingCommand::ListPlots {
                neighborhood_id: id,
            }));
        }
        Ok(None)
    }

    /// Retail catalog searcher results: search, category, subcategory, Dyeable Only and
    /// collection filters, in catalog order.
    pub fn matching(&self, authority: &HousingAuthority) -> Vec<&Decor> {
        let search = housing_search_query(&self.search).to_lowercase();
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
        let changed = housing_search_query(&self.search) != housing_search_query(&search);
        self.search = search;
        if !changed {
            return;
        }
        if !housing_search_query(&self.search).is_empty() {
            self.category = None;
            self.subcategory = None;
        }
        self.reset_results(authority);
    }

    pub fn set_category(&mut self, id: Option<u32>, authority: &HousingAuthority) {
        let id = id.filter(|id| *id != ALL_CATEGORY);
        if self.category == id && self.subcategory.is_none() {
            return;
        }
        self.category = id;
        self.subcategory = None;
        self.clear_search_for_category();
        self.reset_results(authority);
    }

    pub fn set_subcategory(&mut self, id: Option<u32>, authority: &HousingAuthority) {
        if self.subcategory == id {
            return;
        }
        self.subcategory = id;
        self.clear_search_for_category();
        self.reset_results(authority);
    }

    fn clear_search_for_category(&mut self) {
        if self.category.is_some() {
            self.search.clear();
            self.search_focused = false;
        }
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

/// Default MIN_CHARACTER_SEARCH; localized one-character overrides remain separate.
pub(crate) fn housing_search_query(raw: &str) -> &str {
    if raw.chars().take(3).count() < 3 {
        ""
    } else {
        raw
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

/// `NORMAL_FONT_COLOR`.
const NORMAL_FONT_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];

/// HouseEditorStorageButtonMixin: GlobalStrings54006 HOUSE_EDITOR_CATALOG_BUTTON_TOOLTIP.
pub fn storage_handle_tooltip() -> crate::game_tooltip::GameTooltip {
    crate::game_tooltip::GameTooltip::new(
        crate::tooltip_presentation::TooltipPresentation {
            title: "House Chest".into(),
            title_color: crate::tooltip_presentation::TOOLTIP_WHITE,
            ..crate::tooltip_presentation::TooltipPresentation::hidden()
        },
        None,
    )
}

/// HousingCatalogDecorEntryMixin tooltip: the name (Common quality white), one
/// `HOUSING_DECOR_CATEGORIES_TOOLTIP` line per subcategory (the subcategory alone when it
/// already names its category) and, once anything is owned, `HOUSING_DECOR_OWNED_COUNT_FORMAT`.
pub fn decor_entry_tooltip(
    decor: &Decor,
    catalog: &HousingCatalog,
    authority: &HousingAuthority,
) -> crate::game_tooltip::GameTooltip {
    use crate::tooltip_presentation::{TOOLTIP_WHITE, TooltipLineState, TooltipPresentation};
    let mut lines: Vec<TooltipLineState> = decor
        .subcategories
        .iter()
        .filter_map(|id| catalog.subcategories.iter().find(|sub| sub.id == *id))
        .filter_map(|subcategory| {
            let category = catalog
                .categories
                .iter()
                .find(|category| category.id == subcategory.category_id)?;
            let text = if subcategory.name.contains(&category.name) {
                subcategory.name.clone()
            } else {
                format!("{} - {}", category.name, subcategory.name)
            };
            Some(TooltipLineState::colored(text, TOOLTIP_WHITE))
        })
        .collect();
    if let (Some(owned), Some(placed), Some(stored)) = (
        authority.owned(decor.id),
        authority.placed(decor.id),
        authority.quantity(decor.id),
    ) && owned != 0
    {
        lines.push(TooltipLineState::colored(
            format!("Total Owned: {owned} (Placed: {placed}, Storage: {stored})"),
            NORMAL_FONT_COLOR,
        ));
    }
    crate::game_tooltip::GameTooltip::new(
        TooltipPresentation {
            title: decor.name.clone(),
            title_color: TOOLTIP_WHITE,
            lines,
            ..TooltipPresentation::hidden()
        },
        None,
    )
}
