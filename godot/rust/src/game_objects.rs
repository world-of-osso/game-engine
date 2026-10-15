//! Real replicated usable game-object M2s, including stones and ritual portals,
//! resolved through build-pinned GameObjectDisplayInfo.
use crate::{
    assets::{
        M2_BOUNDS_META, build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
    },
    lighting::TerrainLight,
    particles::{ModelParticles, ParticlePools, PlacedParticles, view_basis},
    world_models::bind_visual_light,
};
use game_engine_core::csv_util::parse_csv_line;
use game_engine_network::replica::Unit;
use game_engine_ui_model::wow_cursor_data::ActiveWowCursor;
use godot::{
    classes::{Node3D, PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D, StaticBody3D},
    prelude::*,
};
use shared::components::{Position, Rotation};
use shared::protocol::{
    GAMEOBJECT_TYPE_BATTLEGROUND_FLAG, GAMEOBJECT_TYPE_BUTTON, GAMEOBJECT_TYPE_CHAIR,
    GAMEOBJECT_TYPE_GENERIC, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_MAILBOX,
    GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_RITUAL, GAMEOBJECT_TYPE_SPELL_FOCUS,
    GameObjectAnimation, GameObjectCollision, GameObjectInfo, GameObjectVisible,
};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

mod transport_wmo;

/// Separate from baked scenery: only these live meshes follow GO collision state.
pub(crate) const GAME_OBJECT_LAYER: u32 = 1 << 5;
const COLLISION_BENCH_SAMPLES: usize = 1200;
const COLLISION_BENCH_WARMUP: usize = 120;
const COLLISION_BENCH_PLAYERS: usize = 6;
const COLLISION_BENCH_WALL_LENGTH: f32 = 7.0;

fn measure_live_queries(
    space: &Gd<PhysicsDirectSpaceState3D>,
    feet: glam::Vec3,
) -> (Vec<f64>, i64) {
    let mut samples = Vec::with_capacity(COLLISION_BENCH_SAMPLES);
    let mut hits = 0_i64;
    for frame in 0..(COLLISION_BENCH_WARMUP + COLLISION_BENCH_SAMPLES) {
        let started = std::time::Instant::now();
        for player in 0..COLLISION_BENCH_PLAYERS {
            let angle = ((frame + player * 60) as f32).to_radians();
            let direction = glam::Vec3::new(angle.cos(), 0.0, angle.sin());
            let point = feet + direction * (player as f32 * 0.25);
            hits += i64::from(floor_hit(space, point).is_some());
            hits += i64::from(
                crate::wmo::collision::wall_hit(
                    space,
                    point + glam::Vec3::Y,
                    direction,
                    COLLISION_BENCH_WALL_LENGTH,
                )
                .is_some(),
            );
        }
        if frame >= COLLISION_BENCH_WARMUP {
            samples.push(started.elapsed().as_secs_f64() * 1e6);
        }
    }
    samples.sort_by(f64::total_cmp);
    (samples, hits)
}

struct ObjectNode {
    /// The display its loaded visual shows.
    display_id: u32,
    node: Gd<Node3D>,
    animation: Option<GameObjectAnimation>,
    particles: Option<PlacedParticles>,
}
pub(crate) struct GameObjects {
    data_root: PathBuf,
    displays: Option<Result<HashMap<u32, u32>, String>>,
    objects: HashMap<u64, ObjectNode>,
    transports: HashMap<u64, crate::transport_motion::TransportView>,
    light: Option<TerrainLight>,
    particles: Option<ParticlePools>,
}

fn parse_displays(text: &str) -> Result<HashMap<u32, u32>, String> {
    let mut lines = text.lines();
    let headers = parse_csv_line(lines.next().ok_or("Empty GameObjectDisplayInfo.csv")?);
    let id = headers
        .iter()
        .position(|h| h == "ID")
        .ok_or("GameObjectDisplayInfo lacks ID")?;
    let fdid = headers
        .iter()
        .position(|h| h == "FileDataID")
        .ok_or("GameObjectDisplayInfo lacks FileDataID")?;
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row = parse_csv_line(line);
            let number = |index: usize| {
                row.get(index)
                    .ok_or_else(|| format!("Incomplete GameObjectDisplayInfo row: {line}"))?
                    .parse::<u32>()
                    .map_err(|e| format!("GameObjectDisplayInfo row {line}: {e}"))
            };
            Ok((number(id)?, number(fdid)?))
        })
        .collect()
}

pub(crate) const GAMEOBJECT_TYPE_CHEST: u8 = 3;
pub(crate) const GAMEOBJECT_TYPE_GOOBER: u8 = 10;
/// Its use casts the template's spell on the user (TC GameObject::Use).
pub(crate) const GAMEOBJECT_TYPE_SPELLCASTER: u8 = 22;

/// The model's yaw inside its object node.
const MODEL_YAW: f32 = -std::f32::consts::FRAC_PI_2;

/// The cursor a shown game object takes under the pointer and as its soft target icon.
/// Retail sends the template's `IconName` (TrinityCore `GameObjectTemplate::IconName`,
/// GameObjectData.h:67); world.db leaves it empty for the Goldshire Mailbox 142075 and
/// its Wooden Chairs, so the client's type default applies: `interface/cursor/mail.blp`
/// over a mailbox and the gears of `interface/cursor/interact.blp` over any other usable
/// object, as the Bevy client maps them (`cursor_for_interaction`, wow_cursor.rs:141-178).
pub(crate) fn is_seaforium_bomb(info: &GameObjectInfo) -> bool {
    info.go_type == 6 && matches!(info.entry, 195235 | 195331)
}

/// Only the two authored scripted bombs admit manual trap interaction.
pub(crate) fn game_object_info_cursor(info: &GameObjectInfo) -> Option<ActiveWowCursor> {
    if is_seaforium_bomb(info) {
        Some(ActiveWowCursor::Interact)
    } else {
        game_object_cursor(info.go_type)
    }
}

pub(crate) fn game_object_cursor(go_type: u8) -> Option<ActiveWowCursor> {
    match go_type {
        GAMEOBJECT_TYPE_MAILBOX => Some(ActiveWowCursor::Mail),
        shared::protocol::GAMEOBJECT_TYPE_GATHERING_NODE => Some(ActiveWowCursor::Interact),
        GAMEOBJECT_TYPE_GUILD_BANK
        | GAMEOBJECT_TYPE_BUTTON
        | GAMEOBJECT_TYPE_CHAIR
        | GAMEOBJECT_TYPE_MEETINGSTONE
        | GAMEOBJECT_TYPE_RITUAL
        | shared::protocol::GAMEOBJECT_TYPE_KEYSTONE_RECEPTACLE
        | shared::protocol::GAMEOBJECT_TYPE_HOUSING_PLOT
        | GAMEOBJECT_TYPE_CHEST
        | GAMEOBJECT_TYPE_GOOBER
        | GAMEOBJECT_TYPE_SPELLCASTER
        | GAMEOBJECT_TYPE_BATTLEGROUND_FLAG
        | shared::protocol::GAMEOBJECT_TYPE_BATTLEGROUND_FLAG_DROP
        | shared::protocol::GAMEOBJECT_TYPE_FLAGSTAND
        | shared::protocol::GAMEOBJECT_TYPE_CAPTURE_POINT => Some(ActiveWowCursor::Interact),
        _ => None,
    }
}

fn game_object_is_renderable(go_type: u8) -> bool {
    matches!(
        go_type,
        0 | 6
            | transport_wmo::GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT
            | GAMEOBJECT_TYPE_GENERIC
            | GAMEOBJECT_TYPE_SPELL_FOCUS
            | shared::protocol::GAMEOBJECT_TYPE_TRAPDOOR
    ) || game_object_cursor(go_type).is_some()
}

/// The top centre of an M2's bounding box placed by `model` (the model node's global
/// transform).
fn bounds_top(model: Transform3D, bounds: Aabb) -> Vector3 {
    let top = bounds.center() + Vector3::new(0.0, bounds.size.y / 2.0, 0.0);
    model * top
}

impl GameObjects {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            displays: None,
            objects: HashMap::new(),
            transports: HashMap::new(),
            light: None,
            particles: None,
        }
    }
    pub fn enable_particles(&mut self, density: f32) {
        self.particles = Some(ParticlePools::new(density));
    }
    pub fn set_particle_density(&mut self, density: f32) {
        if let Some(pools) = &mut self.particles {
            pools.set_density(density);
        }
    }
    /// Measure actual Godot GO-floor and WMO/live-GO wall rays in the loaded arena.
    /// Synchronous query microbenchmark: six nearby simulated positions, not full frame time.
    pub(crate) fn benchmark_live_queries(
        &self,
        space: &Gd<PhysicsDirectSpaceState3D>,
        feet: glam::Vec3,
    ) -> VarDictionary {
        let (samples, hits) = measure_live_queries(space, feet);
        let mut result = VarDictionary::new();
        result.set("query_microbenchmark", true);
        result.set("warmup_discarded_samples", COLLISION_BENCH_WARMUP as i64);
        result.set("samples", COLLISION_BENCH_SAMPLES as i64);
        result.set("players_per_sample", COLLISION_BENCH_PLAYERS as i64);
        result.set("floor_queries_per_sample", COLLISION_BENCH_PLAYERS as i64);
        result.set("wall_queries_per_sample", COLLISION_BENCH_PLAYERS as i64);
        result.set("loaded_objects", self.objects.len() as i64);
        let bodies = self
            .objects
            .values()
            .filter_map(|object| object.node.get_node_or_null("GameObjectModel/M2Collision"));
        let active = bodies
            .filter_map(|body| body.try_cast::<StaticBody3D>().ok())
            .filter(|body| body.get_collision_layer() & GAME_OBJECT_LAYER != 0)
            .count();
        result.set("active_hulls", active as i64);
        result.set("observed_hits", hits);
        for (name, index) in [
            ("p50_us", 600),
            ("p95_us", 1140),
            ("p99_us", 1188),
            ("max_us", 1199),
        ] {
            result.set(name, samples[index]);
        }
        result
    }

    /// Whether `id` is a shown usable game object.
    pub fn contains(&self, id: u64) -> bool {
        self.objects.contains_key(&id)
    }
    pub fn position(&self, id: u64) -> Option<Vector3> {
        Some(self.objects.get(&id)?.node.get_global_position())
    }
    pub fn transport_pose(&self, id: u64) -> Option<shared::transport_route::TransportPose> {
        Some(self.transports.get(&id)?.pose())
    }
    pub fn advance_transports(&mut self, delta: f32) {
        for (id, view) in &mut self.transports {
            view.advance(delta);
            let pose = view.pose();
            let object = self
                .objects
                .get_mut(id)
                .expect("transport view has a rendered object");
            object.node.set_position(Vector3::new(
                pose.position[0],
                pose.position[2],
                -pose.position[1],
            ));
            object.node.set_rotation(Vector3::new(
                0.0,
                pose.orientation + std::f32::consts::FRAC_PI_2,
                0.0,
            ));
            // Node3D transform notifications are deferred. Input queries this deck in
            // this same frame, so publish its actual child-body transform now.
            let mut body = object
                .node
                .get_node_as::<StaticBody3D>("GameObjectModel/M2Collision");
            body.force_update_transform();
        }
    }
    /// The top centre of the object's model bounds, where its soft target icon sits.
    pub fn icon_anchor(&self, id: u64) -> Option<Vector3> {
        let model = self
            .objects
            .get(&id)?
            .node
            .get_node_or_null("GameObjectModel")?
            .try_cast::<Node3D>()
            .ok()?;
        let bounds = model.get_meta(M2_BOUNDS_META).try_to::<Aabb>().ok()?;
        Some(bounds_top(model.get_global_transform(), bounds))
    }
    pub fn upsert(
        &mut self,
        parent: &mut Gd<Node3D>,
        unit: Unit,
        info: &GameObjectInfo,
    ) -> Result<(), String> {
        let id = unit.server_id;
        if unit
            .get::<GameObjectVisible>()
            .is_some_and(|visible| !visible.0)
        {
            self.remove(id);
            return Ok(());
        }
        if !game_object_is_renderable(info.go_type) {
            self.remove(id);
            return Ok(());
        }
        let Some(position) = unit.get::<Position>() else {
            return Ok(());
        };
        // Initial component fragments may arrive separately; never infer collision from art.
        let Some(collision) = unit.get::<GameObjectCollision>() else {
            return Ok(());
        };
        if !info.scale.is_finite() || info.scale <= 0.0 {
            return Err(format!(
                "Game object {id} has invalid replicated scale {}",
                info.scale
            ));
        }
        let changed = self
            .objects
            .get(&id)
            .is_none_or(|old| old.display_id != info.display_id);
        if changed {
            let (visual, particle_model) = self.load_visual(info)?;
            if game_object_info_cursor(info).is_some()
                && let Err(error) = crate::targeting::attach_pick_area(&visual, id)
            {
                visual.free();
                return Err(error);
            }
            self.remove(id);
            let mut node = Node3D::new_alloc();
            node.set_name(&format!("Mailbox_{id}"));
            node.set_meta("game_object_server_id", &(id as i64).to_variant());
            node.set_meta("game_object_name", &info.name.to_variant());
            node.set_meta("game_object_entry", &(info.entry as i64).to_variant());
            node.set_meta(
                "game_object_display_id",
                &(info.display_id as i64).to_variant(),
            );
            node.add_child(&visual);
            parent.add_child(&node);
            let particles = match self.place_particles(parent, &visual, id, particle_model) {
                Ok(particles) => particles,
                Err(error) => {
                    node.free();
                    return Err(error);
                }
            };
            self.objects.insert(
                id,
                ObjectNode {
                    display_id: info.display_id,
                    node,
                    animation: None,
                    particles,
                },
            );
        }
        let object = self
            .objects
            .get_mut(&id)
            .ok_or("Mailbox visual missing after load")?;
        object
            .node
            .set_position(Vector3::new(position.x, position.y, position.z));
        object.node.set_rotation(Vector3::new(
            0.0,
            unit.get::<Rotation>().map_or(0.0, |r| r.y),
            0.0,
        ));
        object.node.set_scale(Vector3::ONE * info.scale);
        apply_object_collision(object, collision.0);
        if let Some(animation) = unit.get::<GameObjectAnimation>() {
            apply_object_animation(object, *animation)?;
        }
        if let Some(state) = unit.get::<shared::components::TransportState>() {
            match self.transports.get_mut(&id) {
                Some(view) => view.observe(state)?,
                None => {
                    self.transports
                        .insert(id, crate::transport_motion::TransportView::new(state)?);
                }
            }
        }
        Ok(())
    }
    fn place_particles(
        &mut self,
        parent: &mut Gd<Node3D>,
        model: &Gd<Node3D>,
        id: u64,
        source: Option<Rc<ModelParticles>>,
    ) -> Result<Option<PlacedParticles>, String> {
        let Some((pools, source)) = self.particles.as_mut().zip(source) else {
            return Ok(None);
        };
        pools.attach(parent);
        let (placed, errors) =
            pools.place(&source, model, id as u32, &self.data_root.join("textures"));
        if errors.is_empty() {
            Ok(Some(placed))
        } else {
            Err(format!("Game object {id} particles: {}", errors.join("; ")))
        }
    }
    fn load_visual(
        &mut self,
        info: &GameObjectInfo,
    ) -> Result<(Gd<Node3D>, Option<Rc<ModelParticles>>), String> {
        let path = self.data_root.join("db2/active/GameObjectDisplayInfo.csv");
        let displays = self
            .displays
            .get_or_insert_with(|| {
                std::fs::read_to_string(&path)
                    .map_err(|e| format!("Read {}: {e}", path.display()))
                    .and_then(|text| parse_displays(&text))
            })
            .as_ref()
            .map_err(Clone::clone)?;
        let fdid = displays
            .get(&info.display_id)
            .copied()
            .filter(|id| *id != 0)
            .ok_or_else(|| {
                format!(
                    "Game object entry {} has no GameObjectDisplayInfo {} model FileDataID",
                    info.entry, info.display_id
                )
            })?;
        let resolver = local_resolver(&self.data_root);
        if info.go_type == transport_wmo::GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT {
            return self.load_transport(&resolver, info, fdid);
        }
        let path = cache_model_files(&resolver, &self.data_root, fdid).map_err(|e| {
            format!(
                "Game object entry {} display {} model {fdid}: {e}",
                info.entry, info.display_id
            )
        })?;
        let path = GString::from(path.to_string_lossy().as_ref());
        let parsed = read_model(&path)?;
        cache_model_textures(&resolver, &self.data_root, &[0; 3], &parsed)?;
        let (mut model, missing) = build_model(&parsed, &path, &[0; 3], None)?;
        if !missing.is_empty() {
            model.free();
            return Err(format!(
                "Game object entry {} model {fdid} missing texture FDIDs: {missing:?}",
                info.entry
            ));
        }
        if let Some(shape) =
            crate::terrain::doodad_collision::collision_shape(parsed.collision.as_ref())
        {
            crate::terrain::doodad_collision::attach_collision(&mut model, &shape);
        }
        model.set_name("GameObjectModel");
        model.set_meta("model_file_data_id", &(fdid as i64).to_variant());
        model.set_rotation(Vector3::new(0.0, MODEL_YAW, 0.0));
        bind_visual_light(&model, self.light.as_ref());
        Ok((model, ModelParticles::from_model(fdid, &parsed)))
    }
    fn load_transport(
        &self,
        resolver: &osso_asset_resolver::CascListfileResolver,
        info: &GameObjectInfo,
        fdid: u32,
    ) -> Result<(Gd<Node3D>, Option<Rc<ModelParticles>>), String> {
        let mut model =
            transport_wmo::load_transport_wmo(resolver, &self.data_root, fdid, self.light.as_ref())
                .map_err(|e| {
                    format!(
                        "Game object entry {} display {}: {e}",
                        info.entry, info.display_id
                    )
                })?;
        model.set_name("GameObjectModel");
        model.set_meta("model_file_data_id", &(fdid as i64).to_variant());
        // WMO and M2 local axes match (ADT MODF and MDDF share `placement_rotation`).
        model.set_rotation(Vector3::new(0.0, MODEL_YAW, 0.0));
        Ok((model, None))
    }
    /// Advance the source emitters of each replicated, loaded game-object model.
    pub fn update_particles(&mut self, camera: Transform3D, delta: f32) {
        let Some(pools) = &mut self.particles else {
            return;
        };
        let view = view_basis(camera);
        pools.begin_frame();
        for object in self.objects.values_mut() {
            let Some(placed) = &mut object.particles else {
                continue;
            };
            let model = object.node.get_node_as::<Node3D>("GameObjectModel");
            placed.update_and_draw(&model, delta, 1.0, &view, pools);
            object.node.set_meta(
                "game_object_particle_emitters",
                &(placed.emitter_count() as i64).to_variant(),
            );
            object.node.set_meta(
                "game_object_particle_quads",
                &(placed.drawn() as i64).to_variant(),
            );
        }
        pools.end_frame();
    }
    pub fn particle_state(&self) -> Option<VarDictionary> {
        let pools = self.particles.as_ref()?;
        let mut state = VarDictionary::new();
        state.set("drawn", pools.drawn() as i64);
        state.set("pools", pools.pool_count() as i64);
        Some(state)
    }
    /// Models read the scene light (`TerrainLight::bind_scene`), so only its arrival or
    /// removal rebinds them.
    pub fn update_lighting(&mut self, light: Option<TerrainLight>) {
        if self.light.is_some() != light.is_some() {
            for object in self.objects.values() {
                bind_visual_light(&object.node, light.as_ref());
            }
        }
        if let Some((pools, light)) = self.particles.as_mut().zip(light.as_ref()) {
            pools.update_lighting(light);
        }
        self.light = light;
    }
    pub fn remove(&mut self, id: u64) {
        self.transports.remove(&id);
        if let Some(object) = self.objects.remove(&id) {
            object.node.free();
        }
    }
    pub fn reset(&mut self) {
        for (_, object) in self.objects.drain() {
            object.node.free();
        }
        self.transports.clear();
        if let Some(pools) = &mut self.particles {
            pools.reset();
        }
        self.light = None;
    }
}

/// Dedicated authored GO collision, using the same step/search/normal rules as M2 floors.
pub(crate) fn floor_hit(space: &Gd<PhysicsDirectSpaceState3D>, feet: glam::Vec3) -> Option<f32> {
    Some(
        walkable_floor_hit(space, feet)?
            .get("position")?
            .to::<Vector3>()
            .y,
    )
}

pub(crate) fn floor_object(space: &Gd<PhysicsDirectSpaceState3D>, feet: glam::Vec3) -> Option<u64> {
    let hit = walkable_floor_hit(space, feet)?;
    let mut node = hit
        .get("collider")?
        .try_to::<Gd<godot::classes::Node>>()
        .ok()?;
    loop {
        if node.has_meta("game_object_server_id") {
            return Some(node.get_meta("game_object_server_id").to::<i64>() as u64);
        }
        node = node.get_parent()?;
    }
}

fn walkable_floor_hit(
    space: &Gd<PhysicsDirectSpaceState3D>,
    feet: glam::Vec3,
) -> Option<VarDictionary> {
    let from =
        Vector3::from_array((feet + glam::Vec3::Y * shared::ground::STEP_UP_HEIGHT).to_array());
    let to =
        Vector3::from_array((feet - glam::Vec3::Y * shared::ground::FLOOR_SEARCH_DEPTH).to_array());
    let mut query = PhysicsRayQueryParameters3D::create(from, to)
        .expect("Godot could not allocate game-object floor ray parameters");
    query.set_collision_mask(GAME_OBJECT_LAYER);
    let hit = space.clone().intersect_ray(&query);
    let normal = hit.get("normal")?.to::<Vector3>();
    if normal.y < shared::ground::min_walkable_normal_y() {
        return None;
    }
    Some(hit)
}

fn apply_object_collision(object: &mut ObjectNode, enabled: bool) {
    if let Some(mut collision) = object
        .node
        .get_node_or_null("GameObjectModel/M2Collision")
        .and_then(|node| node.try_cast::<godot::classes::StaticBody3D>().ok())
    {
        let layer = if enabled {
            crate::terrain::doodad_collision::DOODAD_LAYER | GAME_OBJECT_LAYER
        } else {
            0
        };
        collision.set_collision_layer(layer);
    }
}

fn apply_object_animation(
    object: &mut ObjectNode,
    animation: GameObjectAnimation,
) -> Result<(), String> {
    if object.animation == Some(animation) {
        return Ok(());
    }
    let mut player = object
        .node
        .get_node_or_null("GameObjectModel/M2Animation")
        .and_then(|node| node.try_cast::<crate::animation::WowAnimationPlayer>().ok())
        .ok_or_else(|| {
            format!(
                "Gameobject display {} has no M2 animation player",
                object.display_id
            )
        })?;
    player
        .bind_mut()
        .play_clip(animation.animation_id, animation.looping)
        .map_err(|error| format!("Gameobject display {}: {error}", object.display_id))?;
    object.animation = Some(animation);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use godot::builtin::EulerOrder;

    /// The model node's global transform as `upsert` places it: the object node at the
    /// replicated position, yaw (orientation + pi/2, game-server `game_object_bundle`) and
    /// scale, and the model rotated by -pi/2 inside it.
    fn placed_model(position: Vector3, orientation: f32, scale: f32) -> Transform3D {
        let yaw = orientation + std::f32::consts::FRAC_PI_2;
        let node = Transform3D::new(
            Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, yaw, 0.0))
                .scaled(Vector3::ONE * scale),
            position,
        );
        node * Transform3D::new(
            Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, MODEL_YAW, 0.0)),
            Vector3::ZERO,
        )
    }

    /// An M2 header bounding box (WoW model space, Z up) as `m2_bounds` stores it.
    fn m2_box(min: [f32; 3], max: [f32; 3]) -> Aabb {
        let low = Vector3::new(min[0], min[2], -max[1]);
        let high = Vector3::new(max[0], max[2], -min[1]);
        Aabb::new(low, high - low)
    }

    /// WoW `(x, y, z)` in engine coordinates (game-server `wow_to_bevy`).
    fn engine(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3::new(x, z, -y)
    }

    #[test]
    fn isle_bombs_offer_normal_disarm_requests_but_arena_traps_do_not() {
        for (entry, name) in [(195235, "Seaforium Bomb"), (195331, "Huge Seaforium Bomb")] {
            let bomb = GameObjectInfo {
                entry,
                go_type: 6,
                display_id: 9061,
                name: name.into(),
                scale: 1.0,
            };
            assert_eq!(
                crate::meeting_stones::spell_or_quest_object_request(77, &bomb),
                Some(shared::protocol::UseGameObject { object: 77 }),
                "normal source bomb interaction must reach the disarm request"
            );
            assert_eq!(
                game_object_info_cursor(&bomb),
                Some(ActiveWowCursor::Interact)
            );
        }
        let shadow_sight = GameObjectInfo {
            entry: 184663,
            go_type: 6,
            display_id: 5932,
            name: "Shadow Sight".into(),
            scale: 1.0,
        };
        assert!(crate::meeting_stones::spell_or_quest_object_request(77, &shadow_sight).is_none());
        assert!(game_object_info_cursor(&shadow_sight).is_none());
    }

    #[test]
    fn arena_doors_and_shadow_sight_render_without_manual_use_cursor() {
        for kind in [0, 6, shared::protocol::GAMEOBJECT_TYPE_TRAPDOOR] {
            assert!(game_object_is_renderable(kind));
            assert!(game_object_cursor(kind).is_none());
        }
    }

    #[test]
    fn battleground_flags_render_and_offer_object_use() {
        let flag = shared::protocol::GAMEOBJECT_TYPE_BATTLEGROUND_FLAG;
        assert!(game_object_is_renderable(flag));
        assert_eq!(game_object_cursor(flag), Some(ActiveWowCursor::Interact));
    }

    #[test]
    fn alterac_source_buttons_and_neutral_goober_render_and_offer_object_use() {
        for (entry, kind, display, name) in [
            (178946, 1, 5652, "Horde Banner"),
            (180418, 10, 6211, "Snowfall Banner"),
        ] {
            let banner = GameObjectInfo {
                entry,
                go_type: kind,
                display_id: display,
                name: name.into(),
                scale: 1.0,
            };
            assert!(banner.is_usable());
            assert!(game_object_is_renderable(kind));
            assert_eq!(game_object_cursor(kind), Some(ActiveWowCursor::Interact));
        }
    }

    #[test]
    fn deephaul_dropped_crystal_renders_and_offers_object_use() {
        let dropped = shared::protocol::GAMEOBJECT_TYPE_BATTLEGROUND_FLAG_DROP;
        assert!(game_object_is_renderable(dropped));
        assert_eq!(game_object_cursor(dropped), Some(ActiveWowCursor::Interact));
    }

    #[test]
    fn kotmogu_orbs_render_and_offer_object_use() {
        let orb = shared::protocol::GAMEOBJECT_TYPE_FLAGSTAND;
        assert!(game_object_is_renderable(orb));
        assert_eq!(game_object_cursor(orb), Some(ActiveWowCursor::Interact));
    }

    #[test]
    fn node_battleground_capture_points_render_and_offer_object_use() {
        let point = shared::protocol::GAMEOBJECT_TYPE_CAPTURE_POINT;
        assert!(game_object_is_renderable(point));
        assert_eq!(game_object_cursor(point), Some(ActiveWowCursor::Interact));
    }

    #[test]
    fn mailbox_and_chair_cursors() {
        assert_eq!(
            game_object_cursor(GAMEOBJECT_TYPE_MAILBOX),
            Some(ActiveWowCursor::Mail)
        );
        assert_eq!(
            game_object_cursor(GAMEOBJECT_TYPE_CHAIR),
            Some(ActiveWowCursor::Interact)
        );
        assert_eq!(
            game_object_cursor(GAMEOBJECT_TYPE_GUILD_BANK),
            Some(ActiveWowCursor::Interact)
        );
        // GAMEOBJECT_TYPE_GENERIC decoration is not shown as usable.
        assert_eq!(game_object_cursor(5), None);
    }

    #[test]
    fn mythic_font_renders_and_has_interaction_cursor() {
        assert_eq!(game_object_cursor(49), Some(ActiveWowCursor::Interact));
        assert!(game_object_is_renderable(49));
    }

    #[test]
    fn toyfx4_decorations_render_without_becoming_interactive() {
        // Authentic effect50 Stormwind banner45011 / entry194274 / display10483.
        assert!(game_object_is_renderable(GAMEOBJECT_TYPE_GENERIC));
        assert_eq!(game_object_cursor(GAMEOBJECT_TYPE_GENERIC), None);
        // Fishing Chair33223 and MOLL-E40768 retain their existing interaction.
        assert!(game_object_is_renderable(GAMEOBJECT_TYPE_CHAIR));
        assert!(game_object_is_renderable(GAMEOBJECT_TYPE_MAILBOX));
    }

    #[test]
    fn quest_objects_show_interact_cursor_and_render_without_changing_decoration_cursor() {
        // Retail Netherwing Egg (chest) and Nerub'ar Egg Sac (goober).
        for kind in [3, 10] {
            assert_eq!(game_object_cursor(kind), Some(ActiveWowCursor::Interact));
            assert!(game_object_is_renderable(kind));
        }
        let decoration = GameObjectInfo {
            entry: 194274,
            go_type: GAMEOBJECT_TYPE_GENERIC,
            display_id: 10483,
            name: "Stormwind banner".into(),
            scale: 1.0,
        };
        assert_eq!(game_object_cursor(decoration.go_type), None);
    }

    #[test]
    fn toyfx18_spellcaster_objects_render_and_send_use() {
        // Foam Sword Rack 194342/display8614: TC GameObject::Use casts its spell on the user.
        let rack = GameObjectInfo {
            entry: 194342,
            go_type: 22,
            display_id: 8614,
            name: "Foam Sword Rack".into(),
            scale: 1.0,
        };
        assert!(game_object_is_renderable(rack.go_type));
        assert_eq!(
            game_object_cursor(rack.go_type),
            Some(ActiveWowCursor::Interact)
        );
        let request = crate::meeting_stones::spell_or_quest_object_request(7, &rack);
        assert_eq!(request.map(|use_object| use_object.object), Some(7));
    }

    #[test]
    fn meetingstones_stone_and_portal_have_interaction_cursor() {
        for kind in [
            shared::protocol::GAMEOBJECT_TYPE_MEETINGSTONE,
            shared::protocol::GAMEOBJECT_TYPE_RITUAL,
        ] {
            assert_eq!(game_object_cursor(kind), Some(ActiveWowCursor::Interact));
        }
    }

    #[test]
    fn icon_anchor_is_the_top_of_the_model_box() {
        // Goldshire Mailbox (gameobject guid 26784, display 1907, model 199999.m2 box
        // -0.516,-0.702,-0.005 .. 0.517,0.721,3.080).
        let (x, y, z, o) = (-9455.99, 45.8229, 56.4395, 1.40499_f32);
        let top = bounds_top(
            placed_model(engine(x, y, z), o, 1.0),
            m2_box([-0.516, -0.702, -0.005], [0.517, 0.721, 3.080]),
        );
        // Box centre (0.0005, 0.0095) turned by the orientation.
        let (cx, cy) = (
            0.0005 * o.cos() - 0.0095 * o.sin(),
            0.0005 * o.sin() + 0.0095 * o.cos(),
        );
        let expected = engine(x + cx, y + cy, z + 3.080);
        assert!(top.distance_to(expected) < 0.001, "{top:?} vs {expected:?}");

        // Goldshire Wooden Chair (guid 26246, orientation 3.77864, display 39, model
        // 198115.m2 box -0.999,-0.553,-0.007 .. 0.179,0.553,1.328): the box centre sits
        // 0.41 yd behind the origin along the chair's facing.
        let (x, y, z, o) = (-9458.4, 16.0574, 56.9748, 3.77864_f32);
        let top = bounds_top(
            placed_model(engine(x, y, z), o, 1.0),
            m2_box([-0.999, -0.553, -0.007], [0.179, 0.553, 1.328]),
        );
        let back = -0.41;
        let expected = engine(x + back * o.cos(), y + back * o.sin(), z + 1.328);
        assert!(top.distance_to(expected) < 0.001, "{top:?} vs {expected:?}");
    }
    #[test]
    fn native_mailbox_display_metadata_uses_file_data_id_not_entry_or_display_id() {
        let rows =
            parse_displays("FileDataID,ID,GeoBox_0\n198004,1727,-0.8\n199999,1907,-0.5\n").unwrap();
        assert_eq!(rows.get(&1727), Some(&198004));
        assert_eq!(rows.get(&1907), Some(&199999));
        assert_eq!(rows.get(&140907), None);
        assert!(parse_displays("ID,Model\n1727,198004\n").is_err());
        assert!(parse_displays("ID,FileDataID\n1727,bad\n").is_err());
    }
}
