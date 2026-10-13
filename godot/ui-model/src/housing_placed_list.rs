//! Retail Expert placed-decor list; local selection/menu state never grants decor.
use crate::housing::{HousingAuthority, HousingDashboard, HousingSurface};
use crate::housing_editor::EditorMode;
use game_engine_core::housing::Decor;
use shared::protocol::{HouseLayout, HousingCommand, PlacedDecor};
use std::collections::{HashMap, HashSet};

pub const LIST: &str = "HouseEditorPlacedDecorScroll";
pub const ENTRY_PREFIX: &str = "HouseEditorPlacedDecorEntry";
pub const ROW_PITCH: f32 = 30.0;
pub const SIZE: [f32; 2] = [400.0, 400.0];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlacedDecorListState {
    pub open: bool,
    pub position: Option<[f32; 2]>,
    pub menu: Option<(u64, [f32; 2])>,
}

impl PlacedDecorListState {
    pub fn set_position(&mut self, position: [f32; 2], viewport: [f32; 2]) -> Result<(), String> {
        if !position
            .iter()
            .chain(viewport.iter())
            .all(|value| value.is_finite())
            || viewport.iter().any(|value| *value <= 0.0)
        {
            return Err(
                "Placed-decor list position/viewport must be finite and viewport positive.".into(),
            );
        }
        self.position = Some([
            position[0].clamp(0.0, (viewport[0] - SIZE[0]).max(0.0)),
            position[1].clamp(0.0, (viewport[1] - SIZE[1]).max(0.0)),
        ]);
        Ok(())
    }
}

fn layout(authority: &HousingAuthority) -> Result<&HouseLayout, String> {
    authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref())
        .ok_or("Enter a house and wait for its confirmed placed-decor layout.".into())
}

fn require_instance(authority: &HousingAuthority, instance: u64) -> Result<&HouseLayout, String> {
    let layout = layout(authority)?;
    if !layout
        .decor
        .iter()
        .any(|placed| placed.instance_id == instance)
    {
        return Err("Placed-decor list target is not in the confirmed layout.".into());
    }
    Ok(layout)
}

impl HousingDashboard {
    pub fn placed_list_entries<'a>(
        &'a self,
        authority: &'a HousingAuthority,
    ) -> Result<Vec<(&'a PlacedDecor, &'a Decor)>, String> {
        let sources: HashMap<_, _> = self
            .catalog
            .decor
            .iter()
            .map(|decor| (decor.id, decor))
            .collect();
        let mut rows = layout(authority)?
            .decor
            .iter()
            .map(|placed| {
                let source = sources.get(&placed.decor_id).copied().ok_or_else(|| {
                    format!(
                        "Placed decor{} has no matching authenticated catalog name.",
                        placed.decor_id
                    )
                })?;
                Ok((placed, source))
            })
            .collect::<Result<Vec<_>, String>>()?;
        rows.sort_by_cached_key(|(_, source)| source.name.to_lowercase());
        Ok(rows)
    }

    pub fn toggle_placed_list(&mut self) -> Result<(), String> {
        if self.surface != HousingSurface::Editor || self.editor.mode != EditorMode::Expert {
            return Err("Placed Decor List is an Expert Mode control.".into());
        }
        self.placed_list.open = !self.placed_list.open;
        self.placed_list.menu = None;
        Ok(())
    }

    pub fn close_placed_list(&mut self) {
        self.placed_list.open = false;
        self.placed_list.menu = None;
    }

    pub fn toggle_placed_list_selection(
        &mut self,
        authority: &HousingAuthority,
        instance: u64,
    ) -> Result<(), String> {
        require_instance(authority, instance)?;
        self.placed_list.menu = None;
        if self.editor.selected == Some(instance) {
            self.editor.cancel();
        } else {
            self.editor.select(authority, instance)?;
        }
        Ok(())
    }

    pub fn open_placed_list_menu(
        &mut self,
        authority: &HousingAuthority,
        instance: u64,
        point: [f32; 2],
    ) -> Result<(), String> {
        require_instance(authority, instance)?;
        if !point.iter().all(|value| value.is_finite()) {
            return Err("Placed-decor context position must be finite.".into());
        }
        self.placed_list.menu = Some((instance, point));
        Ok(())
    }

    pub fn placed_list_remove_request(
        &self,
        authority: &HousingAuthority,
    ) -> Result<HousingCommand, String> {
        let (instance, _) = self
            .placed_list
            .menu
            .ok_or("Open a placed-decor context menu first.")?;
        let layout = require_instance(authority, instance)?;
        if !layout.can_edit {
            return Err("Visitors may inspect placed decor but cannot remove it.".into());
        }
        Ok(HousingCommand::RemoveDecor {
            house_id: layout.house_id,
            instance_id: instance,
        })
    }

    pub fn reconcile_placed_list(&mut self, authority: &HousingAuthority) {
        let ids: HashSet<_> = authority
            .state
            .as_ref()
            .and_then(|state| state.layout.as_ref())
            .into_iter()
            .flat_map(|layout| layout.decor.iter().map(|placed| placed.instance_id))
            .collect();
        if self.editor.selected.is_some_and(|id| !ids.contains(&id)) {
            self.editor.cancel();
        }
        if self.editor.hovered.is_some_and(|id| !ids.contains(&id)) {
            self.editor.hovered = None;
        }
        if self
            .placed_list
            .menu
            .is_some_and(|(id, _)| !ids.contains(&id))
        {
            self.placed_list.menu = None;
        }
    }
}
