//! Retail BasicDecorMode and staged ExpertDecorMode; quantities remain server-owned.
use crate::housing::HousingAuthority;
use game_engine_core::housing::Decor;
use shared::protocol::{DecorTransform, HouseLayout, HousingCommand};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum EditorMode {
    #[default]
    Basic,
    Expert,
    Cleanup,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PrecisionSubmode {
    #[default]
    Translate,
    Rotate,
    Scale,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RotationAxis {
    #[default]
    X,
    Y,
    Z,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditorInteraction {
    pub placing: Option<u32>,
    pub selected: Option<u64>,
    pub hovered: Option<u64>,
    pub hover_point: [f32; 2],
    pub yaw: f32,
    pub mode: EditorMode,
    pub submode: PrecisionSubmode,
    pub scale_allowed: bool,
    pub mouse_control_anchor: Option<[f32; 2]>,
    mouse_start: Option<DecorTransform>,
    initial: Option<DecorTransform>,
    draft: Option<DecorTransform>,
    axis: RotationAxis,
    commit_new_decor_on_mouse_up: bool,
    move_overlay_pressed: bool,
}

fn apply_mouse_offset(
    mut start: DecorTransform,
    submode: PrecisionSubmode,
    axis: RotationAxis,
    offset: f32,
) -> Result<DecorTransform, String> {
    if !offset.is_finite() {
        return Err("Mouse manipulation offset must be finite.".into());
    }
    match submode {
        PrecisionSubmode::Translate => {
            let index = match axis {
                RotationAxis::X => 0,
                RotationAxis::Y => 1,
                RotationAxis::Z => 2,
            };
            start.position[index] += offset;
        }
        PrecisionSubmode::Rotate => match axis {
            RotationAxis::X => start.pitch += offset,
            RotationAxis::Y => start.yaw += offset,
            RotationAxis::Z => start.roll += offset,
        },
        PrecisionSubmode::Scale => start.scale += offset,
    }
    validate_mouse_transform(start)
}

fn validate_mouse_transform(transform: DecorTransform) -> Result<DecorTransform, String> {
    let finite = transform.position.iter().all(|value| value.is_finite())
        && [
            transform.yaw,
            transform.pitch,
            transform.roll,
            transform.scale,
        ]
        .iter()
        .all(|value| value.is_finite());
    if !finite || transform.scale <= 0.0 {
        return Err("Decor transform must be finite with positive scale.".into());
    }
    Ok(transform)
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
    pub fn set_mode(&mut self, mode: EditorMode) {
        *self = Self {
            mode,
            ..Default::default()
        };
    }
    pub fn hover(
        &mut self,
        authority: &HousingAuthority,
        instance: Option<u64>,
        point: [f32; 2],
    ) -> Result<(), String> {
        if let Some(instance) = instance {
            let exists = layout(authority)?
                .decor
                .iter()
                .any(|decor| decor.instance_id == instance);
            if !exists {
                return Err("Hovered decor is not in the authoritative layout.".into());
            }
        }
        self.hovered = instance;
        self.hover_point = point;
        Ok(())
    }
    pub fn cancel(&mut self) {
        *self = Self {
            mode: self.mode,
            submode: self.submode,
            ..Default::default()
        };
    }
    pub fn set_submode(&mut self, submode: PrecisionSubmode) {
        if self.submode == submode {
            return;
        }
        self.end_mouse_drag();
        self.submode = submode;
    }
    /// Retail Start/Change/Complete manipulator events stage changes independently
    /// of CommitDecorMovement. Release is not a persistence command.
    pub fn begin_mouse_drag(&mut self, authority: &HousingAuthority) -> Result<(), String> {
        editable_layout(authority)?;
        if self.mode != EditorMode::Expert {
            return Err("Mouse precision manipulation requires Expert Mode.".into());
        }
        if self.submode == PrecisionSubmode::Scale && !self.scale_allowed {
            return Err("Retail expert mode cannot scale WMO decor.".into());
        }
        self.mouse_start = Some(self.draft.ok_or("Select editable decor before dragging.")?);
        Ok(())
    }
    pub fn is_manipulating(&self) -> bool {
        self.mouse_start.is_some()
    }
    pub fn update_mouse_drag(&mut self, offset: f32) -> Result<(), String> {
        let start = self.mouse_start.ok_or("No mouse manipulation is active.")?;
        self.draft = Some(apply_mouse_offset(start, self.submode, self.axis, offset)?);
        Ok(())
    }
    pub fn end_mouse_drag(&mut self) {
        self.mouse_start = None;
    }
    pub fn rotation_axis(&self) -> RotationAxis {
        self.axis
    }
    /// C_HousingExpertMode.SelectNextRotationAxis: X -> Y -> Z -> X.
    pub fn next_axis(&mut self) {
        self.end_mouse_drag();
        self.axis = match self.axis {
            RotationAxis::X => RotationAxis::Y,
            RotationAxis::Y => RotationAxis::Z,
            RotationAxis::Z => RotationAxis::X,
        };
    }
    pub fn expert_transform(&self) -> Option<DecorTransform> {
        self.draft
    }
    pub fn translate_expert(&mut self, delta: [f32; 3]) -> Result<(), String> {
        let mut draft = self.draft.ok_or("Select editable decor in expert mode.")?;
        for (position, change) in draft.position.iter_mut().zip(delta) {
            *position += change;
        }
        if !draft.position.iter().all(|value| value.is_finite()) {
            return Err("Decor position must be finite.".into());
        }
        self.draft = Some(draft);
        Ok(())
    }
    pub fn increment_expert(&mut self, change: f32) -> Result<(), String> {
        let mut draft = self.draft.ok_or("Select editable decor in expert mode.")?;
        match self.submode {
            PrecisionSubmode::Translate => {
                return Err("Translation needs an axis direction.".into());
            }
            PrecisionSubmode::Rotate => {
                let angle = match self.axis {
                    RotationAxis::X => &mut draft.pitch,
                    RotationAxis::Y => &mut draft.yaw,
                    RotationAxis::Z => &mut draft.roll,
                };
                *angle += change;
                if !angle.is_finite() {
                    return Err("Decor rotation must be finite.".into());
                }
            }
            PrecisionSubmode::Scale => {
                if !self.scale_allowed {
                    return Err("Retail expert mode cannot scale WMO decor.".into());
                }
                draft.scale += change;
                if !draft.scale.is_finite() || draft.scale <= 0.0 {
                    return Err("Decor scale must be finite and positive.".into());
                }
            }
        }
        self.draft = Some(draft);
        Ok(())
    }
    pub fn commit_expert(&self, authority: &HousingAuthority) -> Result<HousingCommand, String> {
        let house_id = editable_layout(authority)?.house_id;
        Ok(HousingCommand::TransformDecor {
            house_id,
            instance_id: self
                .selected
                .ok_or("Select placed decor before committing.")?,
            transform: self.draft.ok_or("No expert transform is staged.")?,
        })
    }
    /// Basic DecorMoveOverlay commits on the next click, even when it hits itself.
    pub fn pointer_click(
        &mut self,
        authority: &HousingAuthority,
        picked: Option<u64>,
        ground: Option<[f32; 3]>,
    ) -> Result<Option<HousingCommand>, String> {
        let can_edit = layout(authority)?.can_edit;
        if self.mode == EditorMode::Cleanup {
            let Some(instance) = picked else {
                return Ok(None);
            };
            editable_layout(authority)?;
            self.select(authority, instance)?;
            return self.remove(authority).map(Some);
        }
        if self.mode == EditorMode::Expert {
            if let Some(instance) = picked {
                self.select(authority, instance)?;
                return Ok(None);
            }
            return if can_edit && self.draft.is_some() {
                self.commit_expert(authority).map(Some)
            } else {
                Ok(None)
            };
        }
        let choosing = self.placing.is_none() && self.selected.is_none();
        if choosing || !can_edit {
            if let Some(instance) = picked {
                self.select(authority, instance)?;
            }
            return Ok(None);
        }
        self.commit_at(
            authority,
            ground.ok_or("No authored ground surface is under the pointer.")?,
        )
        .map(Some)
    }
    pub fn begin_place(
        &mut self,
        authority: &HousingAuthority,
        decor: &Decor,
    ) -> Result<(), String> {
        editable_layout(authority)?;
        if !authority
            .quantity(decor.id)
            .is_some_and(|quantity| quantity > 0)
        {
            return Err("This decor is not available in your owned storage.".into());
        }
        let [pitch, yaw, roll] = decor.initial_rotation;
        *self = Self {
            placing: Some(decor.id),
            yaw,
            initial: Some(DecorTransform {
                position: [0.0; 3],
                yaw,
                pitch,
                roll,
                scale: decor.initial_scale,
            }),
            ..Default::default()
        };
        Ok(())
    }
    /// Retail CatalogEntry OnDragStart selects the owned source and arms GLOBAL_MOUSE_UP.
    pub fn begin_place_drag(
        &mut self,
        authority: &HousingAuthority,
        decor: &Decor,
    ) -> Result<(), String> {
        self.begin_place(authority, decor)?;
        self.commit_new_decor_on_mouse_up = true;
        Ok(())
    }

    /// An already-visible DecorMoveOverlay commits on release, never on this press.
    pub fn arm_move_overlay(&mut self) -> bool {
        self.move_overlay_pressed =
            self.mode == EditorMode::Basic && (self.placing.is_some() || self.selected.is_some());
        self.move_overlay_pressed
    }

    pub fn basic_pointer_release(
        &mut self,
        authority: &HousingAuthority,
        ground: Option<[f32; 3]>,
    ) -> Result<Option<HousingCommand>, String> {
        let commit = self.move_overlay_pressed
            || (self.placing.is_some() && self.commit_new_decor_on_mouse_up);
        self.move_overlay_pressed = false;
        if self.mode != EditorMode::Basic || !commit {
            // Source ignores the catalog click's original release; next click can place.
            self.commit_new_decor_on_mouse_up = self.placing.is_some();
            return Ok(None);
        }
        let point = ground.ok_or("No authored ground surface is under the pointer.")?;
        let command = self.commit_at(authority, point)?;
        self.commit_new_decor_on_mouse_up = false;
        Ok(Some(command))
    }

    pub fn select(&mut self, authority: &HousingAuthority, instance_id: u64) -> Result<(), String> {
        let layout = layout(authority)?;
        let decor = layout
            .decor
            .iter()
            .find(|decor| decor.instance_id == instance_id)
            .ok_or("Placed decor is not in the authoritative layout.")?;
        let draft = (self.mode == EditorMode::Expert && layout.can_edit).then_some(decor.transform);
        *self = Self {
            selected: Some(instance_id),
            yaw: decor.transform.yaw,
            mode: self.mode,
            submode: self.submode,
            draft,
            ..Default::default()
        };
        Ok(())
    }
    pub fn placement_transform(&self, position: [f32; 3]) -> Result<DecorTransform, String> {
        let initial = self
            .initial
            .ok_or("Decor has no authenticated initial transform.")?;
        Ok(DecorTransform {
            position,
            yaw: self.yaw,
            ..initial
        })
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
        if let Some(decor_id) = self.placing {
            let transform = self.placement_transform(position)?;
            self.placing = None;
            return Ok(HousingCommand::PlaceDecor {
                variant_id: None,
                house_id,
                decor_id,
                transform,
            });
        }
        Ok(HousingCommand::MoveDecor {
            house_id,
            instance_id: self.selected.ok_or("Select owned or placed decor first.")?,
            position,
        })
    }
    /// New owned decor has no persisted instance yet; rotation belongs to its preview.
    pub fn rotate_placement(
        &mut self,
        authority: &HousingAuthority,
        radians: f32,
    ) -> Result<(), String> {
        editable_layout(authority)?;
        if self.placing.is_none() {
            return Err("Select owned decor before rotating its placement preview.".into());
        }
        let yaw = self.yaw + radians;
        if !yaw.is_finite() {
            return Err("Decor rotation must be finite.".into());
        }
        self.yaw = yaw;
        Ok(())
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
        self.cancel();
        Ok(HousingCommand::RemoveDecor {
            house_id,
            instance_id,
        })
    }
}
