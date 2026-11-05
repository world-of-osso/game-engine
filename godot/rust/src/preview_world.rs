//! Offline capture helpers for client-only visual features (foot IK, ground draping).
//!
//! `preview_world_map` loads a real map without authenticating; these helpers add a real
//! player model and a real spell kit on top of it so a GDScript fixture can capture the
//! drawn result. Nothing here runs in normal gameplay.

use game_engine_core::spell_visual::VisualKit;
use godot::classes::Node3D;
use godot::prelude::*;
use shared::components::Player;

use crate::GameClient;
use crate::animation::WowAnimationPlayer;
use crate::foot_ik::FootIkState;

/// A player model placed on a previewed map, with its foot IK blend state.
pub(crate) struct PreviewCharacter {
    pub node: Gd<Node3D>,
    pub foot_ik: FootIkState,
}

/// A spell kit shown through the production spell-effect path at a fixed anchor.
pub(crate) struct PreviewKit {
    pub anchor: Gd<Node3D>,
    pub visual: u32,
    pub kit: u32,
    pub spell: u32,
    pub started: bool,
}

impl GameClient {
    /// Moves a preview character to `position`, facing `yaw`, as movement would place it.
    pub(crate) fn move_preview_character(
        &mut self,
        index: usize,
        position: Vector3,
        yaw: f64,
    ) -> Result<(), String> {
        let character = self
            .preview_characters
            .get(index)
            .ok_or("preview character index out of range")?;
        let mut node = character.node.clone();
        node.set_position(position);
        node.set_rotation(Vector3::new(0.0, yaw as f32, 0.0));
        Ok(())
    }

    /// Turns ground-effect draping on or off for kits spawned afterwards.
    pub(crate) fn set_preview_drape(&mut self, enabled: bool) {
        self.client_options.graphics.ground_effect_draping_enabled = enabled;
    }

    /// Frees the previewed kits so the same spell can be spawned again.
    pub(crate) fn clear_preview_kits(&mut self) {
        for kit in self.preview_kits.drain(..) {
            self.spell_effects.census_clear(kit.anchor.instance_id());
            if kit.anchor.is_instance_valid() {
                kit.anchor.clone().free();
            }
        }
    }

    /// Gives previewed spell kits a world units root to hang their effects under.
    pub(crate) fn preview_ensure_units_root(&mut self) {
        if self.world.root().is_some() {
            return;
        }
        let mut root = Node3D::new_alloc();
        root.set_name("PreviewWorldUnits");
        self.base_mut().add_child(&root);
        self.world.set_root(root);
    }

    /// Adds a bare player model at `position` (engine axes), facing `yaw`.
    pub(crate) fn preview_add_character(
        &mut self,
        race: u8,
        class: u8,
        sex: u8,
        position: Vector3,
        yaw: f64,
    ) -> Result<usize, String> {
        let player = Player {
            name: String::new(),
            race,
            class,
            appearance: shared::components::CharacterAppearance {
                sex,
                ..Default::default()
            },
        };
        let mut model = crate::assets::player::load_player_model(
            &self.data_root,
            &player,
            &shared::components::EquipmentAppearance::default(),
        )?;
        model.set_name(&format!(
            "PreviewCharacter{}",
            self.preview_characters.len()
        ));
        model.set_position(position);
        model.set_rotation(Vector3::new(0.0, yaw as f32, 0.0));
        self.base_mut().add_child(&model);
        self.preview_characters.push(PreviewCharacter {
            node: model,
            foot_ik: FootIkState::default(),
        });
        Ok(self.preview_characters.len() - 1)
    }

    /// Plays `clip` on preview character `index` at `time_ms`, for stance and stride poses.
    pub(crate) fn preview_play_clip(
        &mut self,
        index: usize,
        clip: u16,
        time_ms: f64,
    ) -> Result<(), String> {
        let character = self
            .preview_characters
            .get(index)
            .ok_or("preview character index out of range")?;
        let mut animation = character
            .node
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            .ok_or("preview character has no animation")?;
        animation
            .bind_mut()
            .play_clip(clip, true)
            .map_err(|error| error.to_string())?;
        animation.bind_mut().advance_time_ms(time_ms);
        Ok(())
    }

    /// Enables or disables foot IK on the preview characters (for before/after captures).
    pub(crate) fn preview_set_foot_ik(&mut self, enabled: bool) {
        self.preview_foot_ik = enabled;
    }

    /// Object queue and loader counters for offline fixture diagnostics.
    pub(crate) fn preview_debug_objects(&self) -> GString {
        GString::from(self.world_objects.debug_state().to_string().as_str())
    }

    /// The collision floor and highest visible render surface at a previewed column.
    pub(crate) fn preview_surface_probe(&mut self, x: f32, z: f32, near: f32) -> VarDictionary {
        let space = self
            .base()
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state());
        let ground = crate::ground::TerrainGround {
            terrain: &self.terrain,
            walls: &|_, _, _| None,
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &|feet| {
                space
                    .as_ref()
                    .and_then(|space| crate::game_objects::floor_hit(space, feet))
            },
        };
        let mut state = VarDictionary::new();
        state.set("terrain", self.terrain.height_at(x, z).unwrap_or(f32::NAN));
        let floor = ground.collision_floor(glam::Vec3::new(x, near, z));
        state.set("floor", floor.unwrap_or(f32::NAN));
        let surface = floor.and_then(|floor| {
            self.world_objects.visible_surface(
                self.terrain.height_at(x, z),
                x,
                z,
                floor,
                shared::ground::STEP_UP_HEIGHT,
                self.frame_id,
            )
        });
        state.set("surface", surface.unwrap_or(f32::NAN));
        state
    }

    /// Per-character foot IK state, for fixture assertions.
    pub(crate) fn preview_foot_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("enabled", self.preview_foot_ik);
        let mut characters = VarArray::new();
        for (index, character) in self.preview_characters.iter().enumerate() {
            let mut entry = VarDictionary::new();
            entry.set("index", index as i64);
            entry.set("position", character.node.get_global_transform().origin);
            if let Some(animation) = character
                .node
                .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            {
                let player = animation.bind();
                entry.set("has_rig", player.leg_rig().is_some());
                entry.set("clip", i64::from(player.action_id()));
            }
            let last = character.foot_ik.last;
            entry.set("weights", &PackedFloat32Array::from(last.weights));
            entry.set("targets", &PackedFloat32Array::from(last.targets));
            entry.set("animated", &PackedFloat32Array::from(last.animated));
            entry.set("floors", &PackedFloat32Array::from(last.floors));
            entry.set("pelvis_drop", last.pelvis_drop);
            characters.push(&entry.to_variant());
        }
        state.set("characters", &characters);
        state
    }

    /// Shows kit `kit` of `visual` at `position` through the production spell-effect path.
    /// Returns `true` once the kit's models are loaded and it has started.
    pub(crate) fn preview_show_kit(
        &mut self,
        visual: u32,
        kit: u32,
        spell: u32,
        position: Vector3,
    ) -> Result<bool, String> {
        let index = match self.preview_kits.iter().position(|preview| {
            preview.visual == visual && preview.kit == kit && preview.spell == spell
        }) {
            Some(index) => index,
            None => {
                let mut anchor = Node3D::new_alloc();
                anchor.set_name(&format!("PreviewKit{visual}_{kit}"));
                anchor.set_position(position);
                // Ground kits are authored against unit attachment ids. A detached preview
                // owner has no rig, so every attachment sits at the anchor origin instead.
                let mut model = Node3D::new_alloc();
                model.set_name("NpcModel");
                let mut skeleton = Node3D::new_alloc();
                skeleton.set_name("Skeleton3D");
                model.add_child(&skeleton);
                for attachment in 0..=60u8 {
                    let mut bone = Node3D::new_alloc();
                    bone.set_name(&format!("AttachmentBone{attachment}"));
                    let mut point = Node3D::new_alloc();
                    point.set_name(&format!("Attachment{attachment}"));
                    bone.add_child(&point);
                    skeleton.add_child(&bone);
                }
                let mut ground = Node3D::new_alloc();
                ground.set_name("AttachmentGround");
                skeleton.add_child(&ground);
                anchor.add_child(&model);
                self.base_mut().add_child(&anchor);
                self.preview_kits.push(PreviewKit {
                    anchor,
                    visual,
                    kit,
                    spell,
                    started: false,
                });
                self.preview_kits.len() - 1
            }
        };
        if self.preview_kits[index].started {
            return Ok(true);
        }
        let Some(prepared) = self.spell_effects.census_prepare(visual, kit)? else {
            return Ok(false);
        };
        start_kit(
            &mut self.spell_effects,
            &prepared,
            &self.preview_kits[index],
        );
        self.preview_kits[index].started = true;
        Ok(true)
    }

    /// Advances preview-character foot IK after the animation players wrote their frame.
    pub(crate) fn update_preview_characters(&mut self, delta: f32) {
        if self.preview_characters.is_empty() {
            return;
        }
        let enabled = self.preview_foot_ik;
        let frame = self.frame_id;
        let space = self
            .base()
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state());
        let ground = crate::ground::TerrainGround {
            terrain: &self.terrain,
            walls: &|_, _, _| None,
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &|feet| {
                space
                    .as_ref()
                    .and_then(|space| crate::game_objects::floor_hit(space, feet))
            },
        };
        for character in &mut self.preview_characters {
            let Some(animation) = character
                .node
                .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            else {
                continue;
            };
            let (rig, skeleton) = {
                let player = animation.bind();
                let (Some(rig), Some(skeleton)) = (player.leg_rig(), player.skeleton()) else {
                    continue;
                };
                (rig, skeleton)
            };
            let anchor = character.node.get_global_position();
            let floor_at = |x: f32, z: f32| {
                self.terrain
                    .height_at(x, z)
                    .into_iter()
                    .chain(ground.collision_floor(glam::Vec3::new(x, anchor.y, z)))
                    .max_by(f32::total_cmp)
            };
            let mut surface_at = |x: f32, z: f32, ceiling: f32, floor: f32| {
                self.world_objects.visible_surface(
                    self.terrain.height_at(x, z),
                    x,
                    z,
                    floor,
                    ceiling - floor,
                    frame,
                )
            };
            let mut skeleton = skeleton;
            let scale = skeleton.get_global_transform().basis.get_scale().y;
            character.foot_ik.apply(
                &mut skeleton,
                rig,
                scale,
                enabled,
                delta,
                &floor_at,
                &mut surface_at,
            );
        }
    }

    /// Holds or resumes preview character `index`'s animation, for repeatable captures.
    pub(crate) fn preview_hold_character(
        &mut self,
        index: usize,
        paused: bool,
    ) -> Result<(), String> {
        let character = self
            .preview_characters
            .get(index)
            .ok_or("preview character index out of range")?;
        let mut animation = character
            .node
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            .ok_or("preview character has no animation")?;
        animation.call("set_paused", &[paused.to_variant()]);
        Ok(())
    }

    /// Removes only the preview characters, leaving the map and kits in place.
    pub(crate) fn preview_clear_characters(&mut self) {
        for character in self.preview_characters.drain(..) {
            if character.node.is_instance_valid() {
                character.node.free();
            }
        }
    }

    /// Frees preview characters and kits (map reload, fixture teardown).
    pub(crate) fn clear_previews(&mut self) {
        for character in self.preview_characters.drain(..) {
            if character.node.is_instance_valid() {
                character.node.free();
            }
        }
        for preview in self.preview_kits.drain(..) {
            self.spell_effects
                .census_clear(preview.anchor.instance_id());
            if preview.anchor.is_instance_valid() {
                preview.anchor.free();
            }
        }
    }
}

fn start_kit(
    effects: &mut crate::spell_effects::SpellEffects,
    kit: &VisualKit,
    preview: &PreviewKit,
) {
    effects.census_start(kit, preview.anchor.instance_id(), preview.spell);
}
