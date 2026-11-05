//! Host wiring of combat animations and spell visuals: server combat traffic to
//! `WorldUnits` clips (`world_combat`), `SpellEffects` kits and melee sounds, per-frame
//! cast holds, kit lifetimes, missiles and particles, and automation state.

use std::time::Instant;

use godot::prelude::*;

use crate::GameClient;
use crate::account::CombatMessage;
use crate::frame_error::FrameError;

impl GameClient {
    pub(super) fn receive_combat_message(
        &mut self,
        message: CombatMessage,
    ) -> Result<(), FrameError> {
        match message {
            CombatMessage::Event(event) => {
                let voiced = self.spell_effects.observe_melee(&event, &self.world);
                self.world.apply_combat_event(&event)?;
                Ok(voiced?)
            }
            CombatMessage::SpellGo(go) => {
                self.sync_spell_visual_specialization();
                if self.world.local_player_id() == Some(go.caster) {
                    self.player_movement.skyriding_spell(go.spell_id);
                }
                self.nameplates.casts.spell_go(go.caster, go.spell_id);
                self.cast_bars.spell_go(go.caster, go.spell_id);
                self.auto_attack_post_cast(&go)?;
                let started = Instant::now();
                let shown = self
                    .spell_effects
                    .spell_go(&go, &self.replica, &mut self.world);
                self.spell_effects.add_busy(started.elapsed());
                Ok(shown?)
            }
            CombatMessage::SpellFailure(failure) => {
                let interrupter = failure
                    .failed_by
                    .and_then(|unit| self.cast_interrupter(unit));
                for casts in [&mut self.nameplates.casts, &mut self.cast_bars] {
                    casts.spell_failure(
                        failure.caster,
                        failure.spell_id,
                        failure.reason,
                        interrupter.clone(),
                    );
                }
                Ok(())
            }
            CombatMessage::AttackStart(start) => {
                self.receive_attack_start(&start);
                Ok(())
            }
            CombatMessage::AttackStopped(stopped) => {
                self.receive_attack_stopped(&stopped);
                Ok(())
            }
        }
    }

    fn sync_spell_visual_specialization(&mut self) {
        let local_spec = self.world.local_player_id().zip(self.account.spells.spec());
        self.spell_effects.set_local_specialization(local_spec);
    }

    pub(super) fn update_spell_visuals(&mut self, delta: f32) -> Result<(), String> {
        self.sync_spell_visual_specialization();
        let started = Instant::now();
        let prefetched = match self.world.local_player_id() {
            Some(local) => self.spell_effects.prefetch(
                local,
                self.account.spells.known(),
                &self.replica,
                &self.world,
            ),
            None => Ok(()),
        };
        let held = self
            .spell_effects
            .sync_casts(&self.replica, &mut self.world);
        let camera = self.world_camera.transform();
        let sound = &self.client_options.sound;
        let gain = if sound.muted {
            0.0
        } else {
            sound.master_volume * sound.effects_volume
        };
        let advanced = self
            .spell_effects
            .advance(delta, camera, gain, &mut self.world);
        self.drape_ground_effects();
        self.spell_effects.end_frame(started.elapsed());
        prefetched.and(held).and(advanced)
    }

    /// Drape the flat ground batches of newly spawned spell models onto the visible render
    /// surface (carpets, stair treads), leaving vertical parts and particles alone.
    ///
    /// A model whose column has no render geometry yet is retried for a bounded number of
    /// frames while the surrounding terrain and world objects spawn.
    fn drape_ground_effects(&mut self) {
        if !self.client_options.graphics.ground_effect_draping_enabled {
            return;
        }
        let drape = game_engine_core::ground_drape::GroundDrape::default();
        let window = shared::ground::STEP_UP_HEIGHT;
        let frame = self.frame_id;
        let space = self
            .world
            .local_player_node()
            .and_then(|node| node.get_world_3d())
            .and_then(|world| world.get_direct_space_state());
        let mut pending = self.spell_effects.take_drape_pending();
        for mut candidate in pending.drain(..) {
            if !candidate.node.is_instance_valid() {
                continue;
            }
            let anchor = candidate.node.get_global_position();
            let anchor = glam::Vec3::new(anchor.x, anchor.y, anchor.z);
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
            let floor_at = |x: f32, z: f32| {
                self.terrain
                    .height_at(x, z)
                    .into_iter()
                    .chain(ground.collision_floor(glam::Vec3::new(x, anchor.y, z)))
                    .max_by(f32::total_cmp)
            };
            let ground_y = floor_at(anchor.x, anchor.z).unwrap_or(anchor.y);
            let mut retry = false;
            // A kit model node can be the mesh itself or wrap its batches.
            let mut instances: Vec<Gd<godot::classes::MeshInstance3D>> = candidate
                .node
                .clone()
                .try_cast::<godot::classes::MeshInstance3D>()
                .into_iter()
                .collect();
            instances.extend(
                candidate
                    .node
                    .find_children_ex("*")
                    .type_("MeshInstance3D")
                    .owned(false)
                    .recursive(true)
                    .done()
                    .iter_shared()
                    .filter_map(|node| node.try_cast::<godot::classes::MeshInstance3D>().ok()),
            );
            for mut instance in instances {
                let Some(mesh) = instance
                    .get_mesh()
                    .and_then(|mesh| mesh.try_cast::<godot::classes::ArrayMesh>().ok())
                else {
                    continue;
                };
                match drape_mesh(
                    &mut instance,
                    &mesh,
                    &drape,
                    ground_y,
                    window,
                    frame,
                    &mut self.world_objects,
                    &self.terrain,
                    &floor_at,
                ) {
                    DrapeOutcome::Draped | DrapeOutcome::NotGround => {}
                    DrapeOutcome::NoSurface => retry = true,
                }
            }
            if retry && candidate.retry() {
                self.spell_effects.requeue_drape(candidate);
            }
        }
    }

    /// Recent kit starts and missile flights, and the kit models and missiles shown now.
    pub(super) fn spell_visuals_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let mut started = VarArray::new();
        for start in self.spell_effects.started() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(start.spell_id));
            entry.set("kit", i64::from(start.kit_id));
            entry.set("unit", start.unit as i64);
            entry.set("event", format!("{:?}", start.event));
            entry.set("anim", start.anim.map_or(-1, i64::from));
            let models: PackedInt64Array =
                start.models.iter().map(|&fdid| i64::from(fdid)).collect();
            entry.set("models", &models);
            started.push(&entry.to_variant());
        }
        state.set("started", &started);
        let mut active = VarArray::new();
        for (unit, spell, kit, model) in self.spell_effects.active_models() {
            let mut entry = VarDictionary::new();
            entry.set("unit", unit as i64);
            entry.set("spell", i64::from(spell));
            entry.set("kit", i64::from(kit));
            entry.set("model", i64::from(model));
            active.push(&entry.to_variant());
        }
        state.set("active", &active);
        state.set("missiles", self.spell_effects.missile_count() as i64);
        let mut flights = VarArray::new();
        for flight in self.spell_effects.flights() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(flight.spell_id));
            entry.set("caster", flight.caster as i64);
            entry.set("target", flight.target as i64);
            entry.set("release_delay", flight.release_delay);
            entry.set("released_at", flight.released_at);
            entry.set("distance", flight.distance);
            entry.set("speed", flight.speed);
            entry.set("flight_time", flight.flight_time.map_or(-1.0, f64::from));
            flights.push(&entry.to_variant());
        }
        state.set("flights", &flights);
        let mut sounds = VarArray::new();
        for start in self.spell_effects.sound_starts() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(start.spell_id));
            entry.set("kit", i64::from(start.kit_id));
            entry.set("unit", start.unit as i64);
            entry.set("sound_kit", i64::from(start.sound_kit_id));
            entry.set("fdid", i64::from(start.fdid));
            entry.set("looping", start.looping);
            entry.set("source", start.source.name());
            entry.set("at", start.at);
            entry.set("late", start.late);
            entry.set("stopped_at", start.stopped_at.map_or(-1.0, f64::from));
            sounds.push(&entry.to_variant());
        }
        state.set("sounds", &sounds);
        let mut melee = VarArray::new();
        for seen in self.spell_effects.melee_seen() {
            let mut entry = VarDictionary::new();
            entry.set("attacker", seen.attacker as i64);
            entry.set("target", seen.target as i64);
            entry.set("result", format!("{:?}", seen.result));
            entry.set("at", seen.at);
            melee.push(&entry.to_variant());
        }
        state.set("melee", &melee);
        state.set("clock", self.spell_effects.clock());
        state.set("assets_pending", self.spell_effects.assets_pending() as i64);
        state.set("frame_ms", self.spell_effects.frame_ms());
        let mut casts = VarArray::new();
        for seen in self.spell_effects.casts_seen() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(seen.spell_id));
            entry.set("unit", seen.unit as i64);
            entry.set("go", seen.go);
            entry.set("at", seen.at);
            entry.set("wall_ms", seen.wall_ms as i64);
            entry.set("elapsed", seen.elapsed);
            entry.set("duration", seen.duration);
            casts.push(&entry.to_variant());
        }
        state.set("casts", &casts);
        state
    }
}

/// What draping one of a spell model's meshes did.
enum DrapeOutcome {
    /// Not a flat ground batch (a vertical part, a particle plane, a plinth).
    NotGround,
    /// A ground batch, but the client has no visible render surface under it yet.
    NoSurface,
    /// The batch now follows the visible surface.
    Draped,
}

/// Drapes one `MeshInstance3D` of a spell model, keeping its UVs, second UVs, colours and
/// material. A skinned batch is measured in its authored bind pose; once draped it draws
/// as the static ground decal it now is, with the kit's own transform and material.
#[allow(clippy::too_many_arguments)]
fn drape_mesh(
    instance: &mut Gd<godot::classes::MeshInstance3D>,
    mesh: &Gd<godot::classes::ArrayMesh>,
    drape: &game_engine_core::ground_drape::GroundDrape,
    ground_y: f32,
    window: f32,
    frame: u64,
    surfaces: &mut crate::terrain::objects::TerrainObjects,
    terrain: &crate::terrain::streaming::StreamedTerrain,
    floor_at: &dyn Fn(f32, f32) -> Option<f32>,
) -> DrapeOutcome {
    use game_engine_core::ground_drape::BatchMesh;
    if mesh.get_surface_count() != 1 {
        return DrapeOutcome::NotGround;
    }
    let arrays = mesh.surface_get_arrays(0);
    let array = |kind: godot::classes::mesh::ArrayType| arrays.get(kind.ord() as usize);
    let Some(positions) = array(godot::classes::mesh::ArrayType::VERTEX)
        .and_then(|value| value.try_to::<PackedVector3Array>().ok())
    else {
        return DrapeOutcome::NotGround;
    };
    let Some(uvs) = array(godot::classes::mesh::ArrayType::TEX_UV)
        .and_then(|value| value.try_to::<PackedVector2Array>().ok())
    else {
        return DrapeOutcome::NotGround;
    };
    let positions: Vec<[f32; 3]> = positions
        .as_slice()
        .iter()
        .map(|point| [point.x, point.y, point.z])
        .collect();
    let uvs: Vec<[f32; 2]> = uvs.as_slice().iter().map(|uv| [uv.x, uv.y]).collect();
    let uvs2: Option<Vec<[f32; 2]>> = array(godot::classes::mesh::ArrayType::TEX_UV2)
        .and_then(|value| value.try_to::<PackedVector2Array>().ok())
        .map(|uvs| uvs.as_slice().iter().map(|uv| [uv.x, uv.y]).collect());
    let colors: Option<Vec<[f32; 4]>> = array(godot::classes::mesh::ArrayType::COLOR)
        .and_then(|value| value.try_to::<PackedColorArray>().ok())
        .map(|colors| {
            colors
                .as_slice()
                .iter()
                .map(|color| [color.r, color.g, color.b, color.a])
                .collect()
        });
    let indices: Vec<u32> = array(godot::classes::mesh::ArrayType::INDEX)
        .and_then(|value| value.try_to::<PackedInt32Array>().ok())
        .filter(|indices| !indices.is_empty())
        .map(|indices| {
            indices
                .as_slice()
                .iter()
                .filter_map(|index| (*index >= 0).then_some(*index as u32))
                .collect()
        })
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    let batch = BatchMesh {
        positions: &positions,
        uvs: &uvs,
        uvs2: uvs2.as_deref(),
        colors: colors.as_deref(),
        indices: &indices,
    };
    let normals: Option<Vec<[f32; 3]>> = array(godot::classes::mesh::ArrayType::NORMAL)
        .and_then(|value| value.try_to::<PackedVector3Array>().ok())
        .map(|normals| normals.as_slice().iter().map(|n| [n.x, n.y, n.z]).collect());
    let placement = crate::render_surface::affine(instance.get_global_transform());
    let mut surface = |x: f32, z: f32, ceiling: f32, floor: f32| {
        surfaces.visible_surface(terrain.height_at(x, z), x, z, floor, ceiling - floor, frame)
    };
    let Some(draped) =
        drape.drape_batch(&batch, placement, ground_y, window, floor_at, &mut surface)
    else {
        return DrapeOutcome::NotGround;
    };
    if draped.dropped == 0 {
        return DrapeOutcome::NoSurface;
    }
    log_draped_batch(&instance, placement, &draped);
    let skinned = !instance.get_skeleton_path().is_empty();
    let flip_normals = winding_normals_are_inward(&positions, &indices, normals.as_deref());
    instance.set_mesh(&draped_mesh(&draped, flip_normals));
    if skinned {
        // The draped geometry is measured in the instance's space, not the animated pose.
        instance.set_skeleton_path(&NodePath::from(""));
    }
    DrapeOutcome::Draped
}

/// Logs one draped batch's real geometry, for capture-run proof.
fn log_draped_batch(
    instance: &Gd<godot::classes::MeshInstance3D>,
    placement: glam::Affine3A,
    draped: &game_engine_core::ground_drape::DrapedBatch,
) {
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for point in &draped.positions {
        let world = placement.transform_point3(glam::Vec3::from_array(*point));
        min = min.min(world);
        max = max.max(world);
    }
    godot_print!(
        "drape batch {}: vertices={} triangles={} hits={} discarded={} max_edge_span={:.3} x={:.2}..{:.2} z={:.2}..{:.2} y={:.2}..{:.2}",
        instance.get_name(),
        draped.positions.len(),
        draped.triangle_count(),
        draped.dropped,
        draped.discarded,
        draped.max_edge_span,
        min.x,
        max.x,
        min.z,
        max.z,
        min.y,
        max.y,
    );
}

/// Rebuilds the authored surface with the draped geometry and smooth normals; the UV,
/// second UV and vertex colours interpolate from the source.
fn draped_mesh(
    draped: &game_engine_core::ground_drape::DrapedBatch,
    flip_normals: bool,
) -> Gd<godot::classes::ArrayMesh> {
    use godot::classes::mesh::{ArrayType, PrimitiveType};
    let vertices: Vec<Vector3> = draped
        .positions
        .iter()
        .map(|point| Vector3::from_array(*point))
        .collect();
    let uvs: Vec<Vector2> = draped
        .uvs
        .iter()
        .map(|uv| Vector2::from_array(*uv))
        .collect();
    let normals = draped_normals(&draped.positions, &draped.indices, flip_normals);
    let indices: Vec<i32> = draped.indices.iter().map(|index| *index as i32).collect();
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        ArrayType::VERTEX.ord() as usize,
        &PackedVector3Array::from(vertices.as_slice()).to_variant(),
    );
    arrays.set(
        ArrayType::NORMAL.ord() as usize,
        &PackedVector3Array::from(normals.as_slice()).to_variant(),
    );
    arrays.set(
        ArrayType::TEX_UV.ord() as usize,
        &PackedVector2Array::from(uvs.as_slice()).to_variant(),
    );
    if let Some(uvs2) = &draped.uvs2 {
        let values: Vec<Vector2> = uvs2.iter().map(|uv| Vector2::from_array(*uv)).collect();
        arrays.set(
            ArrayType::TEX_UV2.ord() as usize,
            &PackedVector2Array::from(values.as_slice()).to_variant(),
        );
    }
    if let Some(colors) = &draped.colors {
        let values: Vec<Color> = colors
            .iter()
            .map(|color| Color::from_rgba(color[0], color[1], color[2], color[3]))
            .collect();
        arrays.set(
            ArrayType::COLOR.ord() as usize,
            &PackedColorArray::from(values.as_slice()).to_variant(),
        );
    }
    arrays.set(
        ArrayType::INDEX.ord() as usize,
        &PackedInt32Array::from(indices.as_slice()).to_variant(),
    );
    let mut mesh = godot::classes::ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh
}

/// Area-weighted vertex normals, so the draped surface keeps a lit, unbroken shading.
/// Whether the mesh's winding normals point into the surface. The drawn meshes carry
/// Godot's front-face winding, so a rebuilt mesh must negate them to keep the authored
/// lighting; the mesh's own normal stream decides, so a source that already agrees stays.
fn winding_normals_are_inward(
    positions: &[[f32; 3]],
    indices: &[u32],
    normals: Option<&[[f32; 3]]>,
) -> bool {
    let Some(normals) = normals else {
        return true;
    };
    let mut agreement = 0.0f32;
    for triangle in indices.chunks_exact(3).take(64) {
        let corners = [
            glam::Vec3::from_array(positions[triangle[0] as usize]),
            glam::Vec3::from_array(positions[triangle[1] as usize]),
            glam::Vec3::from_array(positions[triangle[2] as usize]),
        ];
        let winding = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
        let authored = triangle
            .iter()
            .map(|&index| glam::Vec3::from_array(normals[index as usize]))
            .sum::<glam::Vec3>();
        agreement += winding.dot(authored);
    }
    agreement < 0.0
}

fn draped_normals(positions: &[[f32; 3]], indices: &[u32], flip: bool) -> Vec<Vector3> {
    let mut normals = vec![glam::Vec3::ZERO; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let corners = [
            glam::Vec3::from_array(positions[triangle[0] as usize]),
            glam::Vec3::from_array(positions[triangle[1] as usize]),
            glam::Vec3::from_array(positions[triangle[2] as usize]),
        ];
        let normal = (corners[1] - corners[0]).cross(corners[2] - corners[0])
            * if flip { -1.0 } else { 1.0 };
        for index in triangle {
            normals[*index as usize] += normal;
        }
    }
    normals
        .into_iter()
        .map(|normal| {
            let normal = normal.normalize_or(glam::Vec3::Y);
            Vector3::new(normal.x, normal.y, normal.z)
        })
        .collect()
}

#[cfg(test)]
#[path = "combat_visuals_ground_batch_tests.rs"]
mod ground_batch_tests;
