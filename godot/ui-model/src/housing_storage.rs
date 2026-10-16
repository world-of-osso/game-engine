//! Retail storage owns an independent category focus; only confirmed counts qualify.
use crate::housing::{ALL_CATEGORY, FEATURED_CATEGORY, HousingAuthority, HousingDashboard};
use game_engine_core::housing::{Category, Decor, Subcategory};
use std::collections::BTreeSet;

const MENU_HOVER_SECONDS: f32 = 0.33;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageMenuTarget {
    Sort,
    RootRow,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StorageMenuHover {
    target: Option<StorageMenuTarget>,
    elapsed: f32,
}

const PANEL_SLIDE_SECONDS: f64 = 0.2;
const HANDLE_SLIDE_SECONDS: f64 = 0.15;
const HANDLE_SLIDE_DISTANCE: f32 = 64.0 * 0.67;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawerPart {
    Panel,
    Handle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageTransition {
    elapsed: f64,
}

/// Blizzard_SharedXML/EasingUtil.lua OutExponential; endpoints are exact.
fn out_exponential(elapsed: f64, duration: f64) -> f32 {
    let progress = (elapsed / duration).clamp(0.0, 1.0);
    if progress == 1.0 {
        return 1.0;
    }
    (1.0 - 2.0_f64.powf(-10.0 * progress)) as f32
}

fn transition_presentation(collapsed: bool, elapsed: f64, width: f32) -> (DrawerPart, f32) {
    if collapsed {
        if elapsed < PANEL_SLIDE_SECONDS {
            return (
                DrawerPart::Panel,
                -width * out_exponential(elapsed, PANEL_SLIDE_SECONDS),
            );
        }
        let offset = 1.0 - out_exponential(elapsed - PANEL_SLIDE_SECONDS, HANDLE_SLIDE_SECONDS);
        return (DrawerPart::Handle, -HANDLE_SLIDE_DISTANCE * offset);
    }
    if elapsed < HANDLE_SLIDE_SECONDS {
        return (
            DrawerPart::Handle,
            -HANDLE_SLIDE_DISTANCE * out_exponential(elapsed, HANDLE_SLIDE_SECONDS),
        );
    }
    let offset = 1.0 - out_exponential(elapsed - HANDLE_SLIDE_SECONDS, PANEL_SLIDE_SECONDS);
    (DrawerPart::Panel, -width * offset)
}

fn parse_focus_id(value: &str, clear: &str) -> Result<Option<u32>, String> {
    if value == clear {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| "Invalid storage category identity.".into())
}

impl HousingDashboard {
    pub fn start_storage_transition(&mut self, collapsed: bool) {
        self.storage_panel.collapsed = collapsed;
        self.storage_transition = Some(StorageTransition { elapsed: 0.0 });
    }

    pub fn advance_storage_transition(&mut self, delta: f32) -> Result<(), String> {
        if !delta.is_finite() || delta < 0.0 {
            return Err("Storage transition time must be finite and nonnegative.".into());
        }
        let Some(transition) = &mut self.storage_transition else {
            return Ok(());
        };
        transition.elapsed += f64::from(delta);
        if transition.elapsed >= PANEL_SLIDE_SECONDS + HANDLE_SLIDE_SECONDS {
            self.storage_transition = None;
        }
        Ok(())
    }

    pub fn storage_drawer_presentation(&self) -> (DrawerPart, f32) {
        if let Some(transition) = &self.storage_transition {
            return transition_presentation(
                self.storage_panel.collapsed,
                transition.elapsed,
                self.storage_panel.size[0],
            );
        }
        let part = if self.storage_panel.collapsed {
            DrawerPart::Handle
        } else {
            DrawerPart::Panel
        };
        (part, 0.0)
    }

    pub fn close_storage_menus(&mut self) {
        self.storage_filters_open = false;
        self.storage_sort_open = false;
        self.storage_menu_hover = StorageMenuHover::default();
    }

    /// Menu.lua restarts its0.33s timer on entering another row; leave cancels its target.
    pub fn advance_storage_menu_hover(&mut self, target: Option<StorageMenuTarget>, delta: f32) {
        if !self.storage_filters_open {
            self.storage_menu_hover = StorageMenuHover::default();
            return;
        }
        if self.storage_menu_hover.target != target {
            self.storage_menu_hover = StorageMenuHover {
                target,
                elapsed: 0.0,
            };
        }
        self.storage_menu_hover.elapsed =
            (self.storage_menu_hover.elapsed + delta).min(MENU_HOVER_SECONDS);
        if self.storage_menu_hover.elapsed >= MENU_HOVER_SECONDS {
            match target {
                Some(StorageMenuTarget::Sort) => self.storage_sort_open = true,
                Some(StorageMenuTarget::RootRow) => self.storage_sort_open = false,
                None => {}
            }
        }
    }

    /// Storage disables collection filters; missing acquisition dates cannot define a sort.
    pub fn storage_filter_action(&mut self, action: &str) -> Result<bool, String> {
        match action {
            "housing:storage:filters" => {
                let opening = !self.storage_filters_open;
                self.close_storage_menus();
                self.storage_filters_open = opening;
            }
            "housing:storage:sort" => self.storage_sort_open = true,
            "housing:storage:sort:alphabetical" => {
                self.storage_alphabetical = true;
                self.close_storage_menus();
            }
            "housing:storage:filter:dyeable" => {
                self.storage_dyeable_only = !self.storage_dyeable_only;
                self.storage_sort_open = false;
            }
            "housing:storage:filter:reset" => self.storage_dyeable_only = false,
            action if action.starts_with("housing:storage:sort:") => {
                return Err("Storage acquisition dates have not been authenticated.".into());
            }
            action if action.starts_with("housing:storage:filter:") => {
                return Err("Unsupported owned-storage filter.".into());
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn storage_focus_action(
        &mut self,
        action: &str,
        authority: &HousingAuthority,
    ) -> Result<bool, String> {
        if let Some(value) = action.strip_prefix("housing:storage:category:") {
            let id = parse_focus_id(value, "back")?;
            self.set_storage_category(id, authority)?;
            return Ok(true);
        }
        if let Some(value) = action.strip_prefix("housing:storage:subcategory:") {
            let id = parse_focus_id(value, "all")?;
            self.set_storage_subcategory(id, authority)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn owned_storage_subcategory_ids(&self, authority: &HousingAuthority) -> BTreeSet<u32> {
        let Some(state) = &authority.state else {
            return BTreeSet::new();
        };
        let owned: BTreeSet<_> = state
            .stored
            .iter()
            .chain(&state.placed)
            .filter(|entry| entry.quantity > 0)
            .map(|entry| entry.decor_id)
            .collect();
        self.catalog
            .decor
            .iter()
            .filter(|decor| owned.contains(&decor.id))
            .flat_map(|decor| decor.subcategories.iter().copied())
            .collect()
    }

    pub fn storage_categories(&self, authority: &HousingAuthority) -> Vec<&Category> {
        let owned = self.owned_storage_subcategory_ids(authority);
        self.catalog
            .categories
            .iter()
            .filter(|category| {
                if category.id == ALL_CATEGORY {
                    return true;
                }
                if category.id == FEATURED_CATEGORY {
                    return false;
                }
                self.catalog
                    .subcategories
                    .iter()
                    .any(|child| child.category_id == category.id && owned.contains(&child.id))
            })
            .collect()
    }

    pub fn storage_subcategories(
        &self,
        authority: &HousingAuthority,
        category_id: u32,
    ) -> Vec<&Subcategory> {
        let owned = self.owned_storage_subcategory_ids(authority);
        self.catalog
            .subcategories
            .iter()
            .filter(|child| child.category_id == category_id && owned.contains(&child.id))
            .collect()
    }

    pub fn storage_search_query(&self) -> &str {
        crate::housing::housing_search_query(&self.storage_search)
    }

    pub fn set_storage_search(&mut self, search: String) {
        self.storage_search = search;
        if !self.storage_search_query().is_empty() {
            self.storage_category = None;
            self.storage_subcategory = None;
        }
    }

    pub fn set_storage_category(
        &mut self,
        id: Option<u32>,
        authority: &HousingAuthority,
    ) -> Result<(), String> {
        let id = id.filter(|id| *id != ALL_CATEGORY);
        if let Some(id) = id {
            let available = self
                .storage_categories(authority)
                .iter()
                .any(|category| category.id == id);
            if !available {
                return Err("Storage category has no confirmed owned entries.".into());
            }
            if self.storage_category == Some(id) && self.storage_subcategory.is_none() {
                return Ok(());
            }
            self.storage_search.clear();
            self.storage_search_focused = false;
        }
        self.storage_category = id;
        self.storage_subcategory = None;
        Ok(())
    }

    pub fn set_storage_subcategory(
        &mut self,
        id: Option<u32>,
        authority: &HousingAuthority,
    ) -> Result<(), String> {
        let category = self
            .storage_category
            .ok_or("Select a storage category first.")?;
        if let Some(id) = id {
            let available = self
                .storage_subcategories(authority, category)
                .iter()
                .any(|child| child.id == id);
            if !available {
                return Err("Storage subcategory is not in the confirmed owned category.".into());
            }
        }
        if self.storage_subcategory == id {
            return Ok(());
        }
        self.storage_subcategory = id;
        self.storage_search.clear();
        self.storage_search_focused = false;
        Ok(())
    }

    pub(crate) fn storage_decor_matches(&self, decor: &Decor) -> bool {
        let category = self.storage_category.is_none_or(|id| {
            self.catalog
                .subcategories
                .iter()
                .any(|child| child.category_id == id && decor.subcategories.contains(&child.id))
        });
        let subcategory = self
            .storage_subcategory
            .is_none_or(|id| decor.subcategories.contains(&id));
        category && subcategory
    }

    pub(crate) fn expanded_storage_category(&self) -> Option<&Category> {
        let id = self.storage_category?;
        let count = self
            .catalog
            .subcategories
            .iter()
            .filter(|child| child.category_id == id)
            .count();
        if count <= 1 {
            return None;
        }
        self.catalog
            .categories
            .iter()
            .find(|category| category.id == id)
    }

    pub(crate) fn storage_category_path(&self) -> String {
        let id = self.storage_category.unwrap_or(ALL_CATEGORY);
        let Some(category) = self
            .catalog
            .categories
            .iter()
            .find(|category| category.id == id)
        else {
            return String::new();
        };
        let child = self.storage_subcategory.and_then(|id| {
            self.catalog
                .subcategories
                .iter()
                .find(|child| child.id == id)
        });
        match child {
            Some(child) => format!("{} | {}", category.name, child.name),
            None => category.name.clone(),
        }
    }
}
