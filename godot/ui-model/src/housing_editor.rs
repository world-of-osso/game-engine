//! Retail basic-mode selection/commit decisions; quantities remain server-owned.
use crate::housing::HousingAuthority;
use shared::protocol::{DecorTransform, HouseLayout, HousingCommand};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditorInteraction {
    pub placing: Option<u32>,
    pub selected: Option<u64>,
    pub yaw: f32,
}

fn layout(authority: &HousingAuthority) -> Result<&HouseLayout, String> {
    authority
        .state
        .as_ref()
        .and_then(|state| state.layout.as_ref())
        .ok_or_else(|| "Enter a house and wait for its authoritative layout.".into())
}

fn editable_layout(authority: &HousingAuthority) -> Result<&HouseLayout, String> {
    let layout = layout(authority)?;
    if !layout.can_edit {
        return Err("Visitors may inspect decor but cannot edit it.".into());
    }
    Ok(layout)
}

impl EditorInteraction {
    pub fn begin_place(
        &mut self,
        authority: &HousingAuthority,
        decor_id: u32,
    ) -> Result<(), String> {
        editable_layout(authority)?;
        if !authority
            .quantity(decor_id)
            .is_some_and(|quantity| quantity > 0)
        {
            return Err("This decor is not available in your owned storage.".into());
        }
        *self = Self {
            placing: Some(decor_id),
            selected: None,
            yaw: 0.0,
        };
        Ok(())
    }

    pub fn select(&mut self, authority: &HousingAuthority, instance_id: u64) -> Result<(), String> {
        let decor = layout(authority)?
            .decor
            .iter()
            .find(|decor| decor.instance_id == instance_id)
            .ok_or("Placed decor is not in the authoritative layout.")?;
        *self = Self {
            placing: None,
            selected: Some(instance_id),
            yaw: decor.transform.yaw,
        };
        Ok(())
    }

    pub fn commit_at(
        &mut self,
        authority: &HousingAuthority,
        position: [f32; 3],
    ) -> Result<HousingCommand, String> {
        let house_id = editable_layout(authority)?.house_id;
        if !position.iter().all(|value| value.is_finite()) {
            return Err("Decor position must be finite.".into());
        }
        if let Some(decor_id) = self.placing.take() {
            return Ok(HousingCommand::PlaceDecor {
                house_id,
                decor_id,
                transform: DecorTransform {
                    position,
                    yaw: self.yaw,
                },
            });
        }
        let instance_id = self.selected.ok_or("Select owned or placed decor first.")?;
        Ok(HousingCommand::MoveDecor {
            house_id,
            instance_id,
            position,
        })
    }

    pub fn rotate(
        &mut self,
        authority: &HousingAuthority,
        radians: f32,
    ) -> Result<HousingCommand, String> {
        let house_id = editable_layout(authority)?.house_id;
        let instance_id = self
            .selected
            .ok_or("Select placed decor before rotating.")?;
        let yaw = self.yaw + radians;
        if !yaw.is_finite() {
            return Err("Decor rotation must be finite.".into());
        }
        self.yaw = yaw;
        Ok(HousingCommand::RotateDecor {
            house_id,
            instance_id,
            yaw,
        })
    }

    pub fn remove(&mut self, authority: &HousingAuthority) -> Result<HousingCommand, String> {
        let house_id = editable_layout(authority)?.house_id;
        let instance_id = self
            .selected
            .ok_or("Select placed decor before removing.")?;
        *self = Self::default();
        Ok(HousingCommand::RemoveDecor {
            house_id,
            instance_id,
        })
    }
}
