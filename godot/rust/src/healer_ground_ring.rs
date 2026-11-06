//! Healer ground rings: a draped health arc under every party member whose health is
//! below the hiding threshold, its pulsing dispel ring, and the screen-edge chevrons for
//! off-screen allies (PLAN USER 11-05).
//!
//! Health is arc length and the colour ramp is a secondary cue; both come from
//! [`game_engine_ui_model::healer_ground_ring`]. The ring mesh reuses the visible-surface
//! raycast that foot IK and ground draping use, so it follows stairs and carpets. The
//! only network input is the group state the client already receives: no server change.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use game_engine_core::client_options_data::HealerGroundRingOptions;
use game_engine_core::dispel_data::{DispelCatalog, DispelSchool, DispelSchools};
use game_engine_ui_model::healer_ground_ring::{
    ARC_STATIONS, CHEVRON_MARGIN, DISPEL_INNER_RADIUS, DISPEL_OUTER_RADIUS, HEALTH_INNER_RADIUS,
    HEALTH_OUTER_RADIUS, RING_LIFT, RingDraw, RingMember, RingScope, arc_stations,
    chevron_placement, dispel_pulse, ring_draws, ring_ray_hit, ring_scope, station_offset,
};
use godot::classes::{
    ArrayMesh, Camera3D, CanvasLayer, MeshInstance3D, Node3D, PhysicsDirectSpaceState3D, Polygon2D,
    StandardMaterial3D, Viewport, base_material_3d, mesh,
};
use godot::prelude::*;
use shared::death::DeathState;

use crate::GameClient;
use crate::terrain::objects::TerrainObjects;
use crate::terrain::streaming::StreamedTerrain;

/// The member's ground position may drift this far before the ring re-drapes.
const REDRAPE_DISTANCE: f32 = 0.35;
/// A ring re-drapes once more this soon after its first drape, so terrain and carpets
/// that arrive with the surrounding tile join it; after that only movement re-drapes it.
const REDRAPE_SETTLE_SECS: f32 = 2.0;
/// Angular stations of the thin dispel ring.
const DISPEL_STATIONS: usize = 36;
/// Reused chevron nodes; a party has at most five members.
const CHEVRON_NODES: usize = 8;
/// `StandardMaterial3D` render priority above the ground transparent pass.
const RING_RENDER_PRIORITY: i32 = 4;

const CHEVRON_NAME: &str = "HealerGroundRingChevrons";
/// A ">" chevron pointing along +X, in pixels.
const CHEVRON_POINTS: [(f32, f32); 4] = [(9.0, 0.0), (-7.0, -9.0), (-3.0, 0.0), (-7.0, 9.0)];

enum CatalogLoad {
    Idle,
    Loading(Receiver<Result<DispelCatalog, String>>),
    Ready(Box<DispelCatalog>),
    Failed,
}

/// Members fed by an offline fixture instead of the group state.
pub(crate) struct PreviewRingParty {
    pub spec: u32,
    pub known: Vec<u32>,
    pub members: Vec<PreviewRingMember>,
}

pub(crate) struct PreviewRingMember {
    pub name: String,
    pub health_fraction: f32,
    pub position: Vector3,
    pub debuff_types: Vec<u8>,
    pub dead: bool,
    pub offline: bool,
    pub entity: Option<u64>,
}

/// One member as the ring renderer sees it.
struct RingSource {
    name: String,
    health_fraction: f32,
    position: Vector3,
    debuff_types: Vec<u8>,
    dead: bool,
    offline: bool,
    target: Option<u64>,
}

/// A drawn ring and its persistent nodes.
struct MemberRing {
    node: Gd<Node3D>,
    mesh: Gd<MeshInstance3D>,
    material: Gd<StandardMaterial3D>,
    dispel_mesh: Gd<MeshInstance3D>,
    dispel_material: Gd<StandardMaterial3D>,
    /// Per-station `(inner, outer)` heights relative to `anchor.y`.
    heights: Option<Vec<[f32; 2]>>,
    dispel_heights: Option<Vec<[f32; 2]>>,
    /// Node origin: the ground under the member at drape time.
    anchor: Vector3,
    draped_at: Vector2,
    draped_for: f32,
    /// Drapes since the ring appeared: the first, the settle refresh, then movement.
    drapes: u32,
    built_arc: Option<(f32, [f32; 3])>,
    built_dis: Option<DispelSchool>,
    drawn: Option<RingDraw>,
    target: Option<u64>,
    /// World-space bounds of the last built health mesh, for capture proof.
    mesh_bounds: Option<(f32, f32)>,
    mesh_vertices: usize,
    /// Last applied presentation, so a still ring makes no Godot calls.
    applied_alpha: f32,
    applied_color: [f32; 3],
    shown: bool,
    dispel_shown: bool,
    node_at: Vector3,
    applied_dispel_alpha: f32,
}

pub(crate) struct HealerGroundRings {
    catalog: CatalogLoad,
    members: HashMap<String, MemberRing>,
    preview: Option<PreviewRingParty>,
    layer: Option<Gd<CanvasLayer>>,
    chevrons: Vec<Gd<Polygon2D>>,
    elapsed: f32,
    last_scope: RingScope,
    last_schools: DispelSchools,
    last_spec: Option<u32>,
    /// Pick rings of the last frame: `(center, inner, outer, arc_start, target)`.
    pick: Vec<([f32; 3], f32, f32, f32, u64)>,
    /// Chevron placements of the last frame, for the fixture snapshot.
    chevron_state: HashMap<String, [f32; 3]>,
    /// Every member source of the last frame and its draw, including hidden ones.
    last_members: Vec<(String, Option<RingDraw>)>,
    /// Ground columns sampled by drapes, and the feature's own frame cost.
    drape_samples: u64,
    update_us: f64,
    update_frames: u64,
    /// Fixture-only pause that keeps the nodes and their draped heights.
    suspended: bool,
    /// Chevrons re-project at 10 Hz; the edge arrow does not need every frame.
    chevrons_at: f32,
}

impl Default for HealerGroundRings {
    fn default() -> Self {
        Self {
            catalog: CatalogLoad::Idle,
            members: HashMap::new(),
            preview: None,
            layer: None,
            chevrons: Vec::new(),
            elapsed: 0.0,
            last_scope: RingScope::Off,
            last_schools: DispelSchools::default(),
            last_spec: None,
            pick: Vec::new(),
            chevron_state: HashMap::new(),
            last_members: Vec::new(),
            drape_samples: 0,
            update_us: 0.0,
            update_frames: 0,
            suspended: false,
            chevrons_at: f32::MIN,
        }
    }
}

/// The ground sources a ring drape reads.
struct DrapeContext<'a> {
    terrain: &'a StreamedTerrain,
    objects: &'a mut TerrainObjects,
    space: Option<Gd<PhysicsDirectSpaceState3D>>,
    frame: u64,
    /// Columns sampled this frame, accumulated into the fixture snapshot.
    samples: u64,
}

impl HealerGroundRings {
    pub(crate) fn with_preview(preview: PreviewRingParty) -> Self {
        Self {
            preview: Some(preview),
            ..Self::default()
        }
    }

    fn catalog(&self) -> Option<&DispelCatalog> {
        match &self.catalog {
            CatalogLoad::Ready(catalog) => Some(catalog),
            _ => None,
        }
    }

    /// Start the DB2 read on a worker thread; the 600k-row `SpellEffect` scan never runs
    /// on the frame thread.
    fn poll_catalog(&mut self, data_root: &std::path::Path) {
        match &self.catalog {
            CatalogLoad::Idle => {
                let dir = data_root.join("db2/active");
                let (send, receive) = mpsc::channel();
                game_engine_core::worker_shutdown::track(std::thread::spawn(move || {
                    let _ = send.send(DispelCatalog::load(&dir));
                }));
                self.catalog = CatalogLoad::Loading(receive);
            }
            CatalogLoad::Loading(receive) => match receive.try_recv() {
                Ok(Ok(catalog)) => self.catalog = CatalogLoad::Ready(Box::new(catalog)),
                Ok(Err(error)) => {
                    godot_error!("Dispel catalog failed: {error}");
                    self.catalog = CatalogLoad::Failed;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    godot_error!("Dispel catalog loader stopped");
                    self.catalog = CatalogLoad::Failed;
                }
            },
            CatalogLoad::Ready(_) | CatalogLoad::Failed => {}
        }
    }

    fn catalog_state(&self) -> &'static str {
        match self.catalog {
            CatalogLoad::Idle => "idle",
            CatalogLoad::Loading(_) => "loading",
            CatalogLoad::Ready(_) => "ready",
            CatalogLoad::Failed => "failed",
        }
    }

    fn free_member(&mut self, name: &str) {
        if let Some(member) = self.members.remove(name)
            && member.node.is_instance_valid()
        {
            member.node.clone().free();
        }
    }

    fn chevron_node(&mut self, parent: &mut Gd<Node3D>, index: usize) -> Gd<Polygon2D> {
        while self.chevrons.len() <= index {
            let layer = self.layer.get_or_insert_with(|| {
                let mut layer = CanvasLayer::new_alloc();
                layer.set_name(CHEVRON_NAME);
                // With the nameplates, below the registry UI layers.
                layer.set_layer(0);
                parent.add_child(&layer);
                layer
            });
            let mut chevron = Polygon2D::new_alloc();
            chevron.set_name(&format!("HealerGroundRingChevron{}", self.chevrons.len()));
            let points: Vec<Vector2> = CHEVRON_POINTS
                .iter()
                .map(|&(x, y)| Vector2::new(x, y))
                .collect();
            chevron.set_polygon(&PackedVector2Array::from(points.as_slice()));
            chevron.set_visible(false);
            layer.add_child(&chevron);
            self.chevrons.push(chevron);
        }
        self.chevrons[index].clone()
    }
}

impl GameClient {
    /// Build the ring sources from the group state the client already has. Positions use
    /// the rendered unit when it is replicated and the group state's own position when it
    /// is outside interest range; both are feet positions in world space.
    fn healer_ring_sources(&self) -> (Vec<RingSource>, bool, bool) {
        let group = &self.account.group;
        let in_raid = group.is_raid && !group.members.is_empty();
        let in_party = !group.is_raid && !group.members.is_empty();
        let local = self.account.session.selected_character_name.as_deref();
        let mut sources = Vec::with_capacity(group.members.len());
        for member in &group.members {
            if local.is_some_and(|name| name.eq_ignore_ascii_case(&member.name)) {
                continue;
            }
            let live = group
                .live
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&member.name))
                .map(|(_, state)| state);
            let Some(live) = live else {
                sources.push(RingSource {
                    name: member.name.clone(),
                    health_fraction: 0.0,
                    position: Vector3::ZERO,
                    debuff_types: Vec::new(),
                    dead: false,
                    offline: true,
                    target: member.entity,
                });
                continue;
            };
            let health_fraction = if live.max_health == 0 {
                0.0
            } else {
                (live.health as f32 / live.max_health as f32).clamp(0.0, 1.0)
            };
            let node_position = member
                .entity
                .and_then(|entity| self.world.unit_node(entity))
                .map(|node| node.get_global_position());
            let position = node_position.unwrap_or(Vector3::new(
                live.position.x,
                live.position.y,
                live.position.z,
            ));
            sources.push(RingSource {
                name: member.name.clone(),
                health_fraction,
                position,
                debuff_types: live.debuffs.iter().map(|aura| aura.dispel_type).collect(),
                dead: matches!(live.death, DeathState::Dead | DeathState::Ghost),
                offline: !member.online,
                target: member.entity,
            });
        }
        (sources, in_party, in_raid)
    }

    pub(super) fn update_healer_ground_rings(&mut self, delta: f32) {
        let started = std::time::Instant::now();
        self.update_healer_ground_rings_inner(delta);
        if !self.healer_rings.suspended {
            self.healer_rings.update_us += started.elapsed().as_secs_f64() * 1.0e6;
            self.healer_rings.update_frames += 1;
        }
    }

    fn update_healer_ground_rings_inner(&mut self, delta: f32) {
        if self.healer_rings.suspended {
            self.healer_rings.hide_all();
            return;
        }
        // The feature is idle outside the world unless a fixture injected a party.
        let preview = self.healer_rings.preview.is_some();
        if !preview && self.account.session.screen != game_engine_session::SessionScreen::InWorld {
            self.healer_rings.hide_all();
            return;
        }
        self.healer_rings.poll_catalog(&self.data_root);
        let Some(catalog) = self.healer_rings.catalog() else {
            return;
        };
        let (sources, in_party, in_raid) = if preview {
            let party = self.healer_rings.preview.as_ref().expect("checked above");
            (
                party
                    .members
                    .iter()
                    .map(|member| RingSource {
                        name: member.name.clone(),
                        health_fraction: member.health_fraction,
                        position: member.position,
                        debuff_types: member.debuff_types.clone(),
                        dead: member.dead,
                        offline: member.offline,
                        target: member.entity,
                    })
                    .collect::<Vec<_>>(),
                true,
                false,
            )
        } else {
            self.healer_ring_sources()
        };
        let spec = if preview {
            self.healer_rings.preview.as_ref().map(|party| party.spec)
        } else {
            self.account.spells.spec()
        };
        let known: Vec<u32> = if preview {
            self.healer_rings
                .preview
                .as_ref()
                .map(|party| party.known.clone())
                .unwrap_or_default()
        } else {
            self.account.spells.known().to_vec()
        };
        let options: HealerGroundRingOptions = self.client_options.hud.healer_ground_rings;
        let schools = catalog.schools_for_spells(known.iter().copied());
        let spec_is_healer = spec.is_some_and(|spec| catalog.is_healer_spec(spec));
        let scope = ring_scope(&options, spec_is_healer, in_party, in_raid);
        let members: Vec<RingMember<'_>> = sources
            .iter()
            .map(|source| RingMember {
                name: &source.name,
                health_fraction: source.health_fraction,
                debuff_types: &source.debuff_types,
                dead: source.dead,
                offline: source.offline,
            })
            .collect();
        let draws = ring_draws(&members, scope, schools, options.colorblind_ramp);
        self.healer_rings.last_members = sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                (
                    source.name.clone(),
                    draws.iter().find(|draw| draw.index == index).copied(),
                )
            })
            .collect();

        let space = self
            .world
            .local_player_node()
            .or_else(|| self.world.root())
            .and_then(|node| node.get_world_3d())
            .and_then(|world| world.get_direct_space_state());
        let mut parent = self.to_gd().upcast::<Node3D>();
        let mut context = DrapeContext {
            terrain: &self.terrain,
            objects: &mut self.world_objects,
            space,
            frame: self.frame_id,
            samples: 0,
        };
        self.healer_rings.elapsed += delta;
        self.healer_rings.last_scope = scope;
        self.healer_rings.last_schools = schools;
        self.healer_rings.last_spec = spec;
        let elapsed = self.healer_rings.elapsed;

        // Drop rings for members who left the group or the scope.
        let drawn_names: Vec<String> = draws
            .iter()
            .filter_map(|draw| sources.get(draw.index).map(|source| source.name.clone()))
            .collect();
        let stale: Vec<String> = self
            .healer_rings
            .members
            .keys()
            .filter(|name| !drawn_names.contains(name))
            .cloned()
            .collect();
        for name in stale {
            self.healer_rings.free_member(&name);
        }

        self.healer_rings.pick.clear();
        for draw in &draws {
            let Some(source) = sources.get(draw.index) else {
                continue;
            };
            let name = source.name.clone();
            let member = self
                .healer_rings
                .members
                .entry(name.clone())
                .or_insert_with(|| spawn_member_ring(&mut parent, &name));
            sync_member(member, source, draw, &mut context, elapsed);
            let center = [source.position.x, member.anchor.y, source.position.z];
            let target = source.target.unwrap_or(u64::MAX);
            if draw.alpha > 0.0 && member.shown {
                self.healer_rings.pick.push((
                    center,
                    HEALTH_INNER_RADIUS,
                    HEALTH_OUTER_RADIUS,
                    draw.arc_start,
                    target,
                ));
            }
            if draw.dispel.is_some() && member.dispel_shown {
                self.healer_rings.pick.push((
                    center,
                    DISPEL_INNER_RADIUS,
                    DISPEL_OUTER_RADIUS,
                    0.0,
                    target,
                ));
            }
        }

        self.healer_rings.drape_samples += context.samples;
        let elapsed = self.healer_rings.elapsed;
        if elapsed - self.healer_rings.chevrons_at >= 0.1 {
            self.healer_rings.chevrons_at = elapsed;
            let viewport = self.base().get_viewport();
            self.healer_rings
                .update_chevrons(&mut parent, &draws, &sources, viewport.as_ref());
        }
    }

    /// The party ring under the pointer, for click-to-target.
    pub(super) fn healer_ring_under_pointer(
        &self,
        camera: &Gd<Camera3D>,
        pointer: Vector2,
    ) -> Option<u64> {
        let rings: Vec<([f32; 3], f32, f32, f32)> = self
            .healer_rings
            .pick
            .iter()
            .map(|(center, inner, outer, arc_start, _)| (*center, *inner, *outer, *arc_start))
            .collect();
        let origin = camera.project_ray_origin(pointer);
        let direction = camera.project_ray_normal(pointer);
        let (index, _) = ring_ray_hit(
            [origin.x, origin.y, origin.z],
            [direction.x, direction.y, direction.z],
            &rings,
        )?;
        let target = self.healer_rings.pick[index].4;
        (target != u64::MAX).then_some(target)
    }

    /// Offline fixture: install a party, specialization and known spells.
    pub(super) fn install_preview_ring_party(
        &mut self,
        spec: u32,
        known: Vec<u32>,
        members: Vec<PreviewRingMember>,
    ) {
        self.healer_rings = HealerGroundRings::with_preview(PreviewRingParty {
            spec,
            known,
            members,
        });
    }

    /// Offline fixture: the ring target under a viewport point, using the production
    /// camera ray and annulus test.
    pub(super) fn preview_ring_pick_at(&self, at: Vector2) -> Option<u64> {
        let viewport = self.base().get_viewport()?;
        let camera = viewport.get_camera_3d()?;
        self.healer_ring_under_pointer(&camera, at)
    }

    /// Offline fixture: turn the feature off for the no-ring frame.
    pub(super) fn set_preview_ring_enabled(&mut self, enabled: bool) {
        self.client_options.hud.healer_ground_rings.enabled = enabled;
    }

    /// Offline fixture: pause and resume the rings without freeing their nodes.
    pub(super) fn suspend_preview_rings(&mut self, suspended: bool) {
        self.healer_rings.suspended = suspended;
        if suspended {
            self.healer_rings.hide_all();
        }
    }

    /// Offline fixture: the rings the feature currently draws, for assertions.
    pub(super) fn preview_ring_state(&self) -> VarDictionary {
        let rings = &self.healer_rings;
        let mut state = VarDictionary::new();
        state.set("catalog", rings.catalog_state());
        state.set("spec", rings.last_spec.map_or(-1, |spec| spec as i64));
        state.set(
            "healer",
            rings
                .last_spec
                .and_then(|spec| rings.catalog().map(|catalog| catalog.is_healer_spec(spec)))
                .unwrap_or(false),
        );
        state.set("dispel_schools", i64::from(rings.last_schools.bits()));
        state.set("scope", format!("{:?}", rings.last_scope));
        state.set("drape_samples", rings.drape_samples as i64);
        state.set("update_frames", rings.update_frames as i64);
        state.set(
            "update_us_mean",
            if rings.update_frames == 0 {
                0.0
            } else {
                rings.update_us / rings.update_frames as f64
            },
        );
        let mut members = VarArray::new();
        for (name, draw) in &rings.last_members {
            let node = rings.members.get(name);
            let mut entry = VarDictionary::new();
            entry.set("name", name.as_str());
            if let Some(draw) = draw {
                entry.set("alpha", draw.alpha);
                entry.set("arc_start_deg", draw.arc_start.to_degrees());
                entry.set("arc_end_deg", draw.arc_end.to_degrees());
                entry.set("color", draw.color.as_slice());
                entry.set("dispel", draw.dispel.map_or("", DispelSchool::name));
                entry.set(
                    "dispel_visible",
                    node.is_some_and(|member| member.dispel_mesh.is_visible()),
                );
            } else {
                entry.set("alpha", 0.0);
                entry.set("dispel", "");
                entry.set("dispel_visible", false);
            }
            entry.set(
                "ring_visible",
                node.is_some_and(|member| member.mesh.is_visible()),
            );
            entry.set(
                "mesh_vertices",
                node.map_or(0, |member| member.mesh_vertices) as i64,
            );
            if let Some((low, high)) = node.and_then(|member| member.mesh_bounds) {
                entry.set("mesh_min_y", low);
                entry.set("mesh_max_y", high);
            }
            if let Some(member) = node {
                entry.set("anchor", member.anchor);
                entry.set("node_global", member.node.get_global_position());
                entry.set("mesh_global", member.mesh.get_global_position());
                entry.set("mesh_aabb", member.mesh.get_aabb());
                entry.set(
                    "mesh_surfaces",
                    member
                        .mesh
                        .get_mesh()
                        .map_or(0, |mesh| mesh.get_surface_count()) as i64,
                );
                let albedo = member.material.get_albedo();
                entry.set(
                    "albedo",
                    [albedo.r, albedo.g, albedo.b, albedo.a].as_slice(),
                );
            }
            if let Some(&[x, y, angle]) = rings.chevron_state.get(name) {
                let mut chevron = VarDictionary::new();
                chevron.set("x", x);
                chevron.set("y", y);
                chevron.set("angle", angle);
                entry.set("chevron", &chevron);
            }
            members.push(&entry.to_variant());
        }
        state.set("members", &members);
        let mut pick = VarArray::new();
        for (center, inner, outer, arc_start, target) in &rings.pick {
            let mut entry = VarDictionary::new();
            entry.set("center", Vector3::new(center[0], center[1], center[2]));
            entry.set("inner", *inner);
            entry.set("outer", *outer);
            entry.set("arc_start", *arc_start);
            entry.set("target", *target as i64);
            pick.push(&entry.to_variant());
        }
        state.set("pick", &pick);
        state
    }
}

impl HealerGroundRings {
    fn hide_all(&mut self) {
        for member in self.members.values_mut() {
            member.mesh.set_visible(false);
            member.dispel_mesh.set_visible(false);
            member.shown = false;
            member.dispel_shown = false;
            member.drawn = None;
        }
        for chevron in &mut self.chevrons {
            chevron.set_visible(false);
        }
        self.pick.clear();
        self.chevron_state.clear();
        self.last_members.clear();
    }

    fn update_chevrons(
        &mut self,
        parent: &mut Gd<Node3D>,
        draws: &[RingDraw],
        sources: &[RingSource],
        viewport: Option<&Gd<Viewport>>,
    ) {
        self.chevron_state.clear();
        let Some(viewport) = viewport else {
            for chevron in &mut self.chevrons {
                chevron.set_visible(false);
            }
            return;
        };
        let size = viewport.get_visible_rect().size;
        let Some(camera) = viewport.get_camera_3d() else {
            for chevron in &mut self.chevrons {
                chevron.set_visible(false);
            }
            return;
        };
        let mut used = 0;
        for draw in draws {
            // Only an ally below the health threshold is marked, as the plan asks.
            if draw.alpha <= 0.0 {
                continue;
            }
            let Some(source) = sources.get(draw.index) else {
                continue;
            };
            let world = source.position + Vector3::UP * 1.0;
            let behind = camera.is_position_behind(world);
            let screen = camera.unproject_position(world);
            let clip = [screen.x / size.x * 2.0 - 1.0, 1.0 - screen.y / size.y * 2.0];
            let Some(placement) = chevron_placement(clip, behind, [size.x, size.y], CHEVRON_MARGIN)
            else {
                continue;
            };
            if used >= CHEVRON_NODES {
                break;
            }
            let mut chevron = self.chevron_node(&mut *parent, used);
            chevron.set_visible(true);
            chevron.set_position(Vector2::new(placement.position[0], placement.position[1]));
            chevron.set_rotation(placement.angle);
            chevron.set_color(Color::from_rgba(
                draw.color[0],
                draw.color[1],
                draw.color[2],
                0.9,
            ));
            self.chevron_state.insert(
                source.name.clone(),
                [
                    placement.position[0],
                    placement.position[1],
                    placement.angle,
                ],
            );
            used += 1;
        }
        for chevron in self.chevrons.iter_mut().skip(used) {
            chevron.set_visible(false);
        }
    }
}

fn ring_material() -> Gd<StandardMaterial3D> {
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(base_material_3d::ShadingMode::UNSHADED);
    material.set_transparency(base_material_3d::Transparency::ALPHA);
    material.set_cull_mode(base_material_3d::CullMode::DISABLED);
    material.set_render_priority(RING_RENDER_PRIORITY);
    material
}

fn spawn_member_ring(parent: &mut Gd<Node3D>, name: &str) -> MemberRing {
    let mut node = Node3D::new_alloc();
    node.set_name(&format!("HealerGroundRing:{name}"));
    let mut mesh = MeshInstance3D::new_alloc();
    mesh.set_name("HealthRing");
    let material = ring_material();
    mesh.set_material_override(&material);
    mesh.set_visible(false);
    node.add_child(&mesh);
    let mut dispel_mesh = MeshInstance3D::new_alloc();
    dispel_mesh.set_name("DispelRing");
    let dispel_material = ring_material();
    dispel_mesh.set_material_override(&dispel_material);
    dispel_mesh.set_visible(false);
    node.add_child(&dispel_mesh);
    parent.add_child(&node);
    MemberRing {
        node,
        mesh,
        material,
        dispel_mesh,
        dispel_material,
        heights: None,
        dispel_heights: None,
        anchor: Vector3::ZERO,
        draped_at: Vector2::new(f32::MAX, f32::MAX),
        draped_for: f32::MIN,
        drapes: 0,
        built_arc: None,
        built_dis: None,
        drawn: None,
        target: None,
        mesh_bounds: None,
        mesh_vertices: 0,
        applied_alpha: f32::NAN,
        applied_color: [f32::NAN; 3],
        shown: false,
        dispel_shown: false,
        node_at: Vector3::ZERO,
        applied_dispel_alpha: f32::NAN,
    }
}

/// Highest collision floor or terrain height under a ring vertex, near the member.
fn floor_under(context: &DrapeContext<'_>, x: f32, z: f32, near: f32) -> Option<f32> {
    let ground = crate::ground::TerrainGround {
        terrain: context.terrain,
        walls: &|_, _, _| None,
        step_wall_blocked: &|_, _, _| false,
        game_object_floor: &|feet| {
            context
                .space
                .as_ref()
                .and_then(|space| crate::game_objects::floor_hit(space, feet))
        },
    };
    context
        .terrain
        .height_at(x, z)
        .into_iter()
        .chain(ground.collision_floor(glam::Vec3::new(x, near, z)))
        .max_by(f32::total_cmp)
}

/// The highest visible render surface under a column, falling back to its collision floor.
fn surface_under(context: &mut DrapeContext<'_>, x: f32, z: f32, near: f32) -> Option<f32> {
    let floor = floor_under(context, x, z, near)?;
    context
        .objects
        .visible_surface(
            context.terrain.height_at(x, z),
            x,
            z,
            floor,
            game_engine_core::ground_drape::MAX_SURFACE_RISE,
            context.frame,
        )
        .or(Some(floor))
}

/// Drapes one annulus around `position` onto the visible surface. Returns the node origin
/// and per-station `(inner, outer)` heights relative to it.
fn drape_ring(
    context: &mut DrapeContext<'_>,
    position: Vector3,
    stations: usize,
    inner: f32,
    outer: f32,
) -> (Vector3, Vec<[f32; 2]>) {
    let anchor = surface_under(context, position.x, position.z, position.y).unwrap_or(position.y);
    let mut heights = Vec::with_capacity(stations);
    for station in 0..stations {
        let angle = TAU * station as f32 / stations as f32;
        let (inner_x, inner_z) = station_offset(angle, inner);
        let (outer_x, outer_z) = station_offset(angle, outer);
        let inner_y = surface_under(
            context,
            position.x + inner_x,
            position.z + inner_z,
            position.y,
        )
        .unwrap_or(anchor);
        let outer_y = surface_under(
            context,
            position.x + outer_x,
            position.z + outer_z,
            position.y,
        )
        .unwrap_or(anchor);
        heights.push([inner_y - anchor + RING_LIFT, outer_y - anchor + RING_LIFT]);
    }
    (
        Vector3::new(position.x, anchor + RING_LIFT, position.z),
        heights,
    )
}

/// Per-frame state of one drawn member: re-drape when the ground changed, then rebuild the
/// arc mesh and the pulsing outer ring as their inputs change.
fn sync_member(
    member: &mut MemberRing,
    source: &RingSource,
    draw: &RingDraw,
    context: &mut DrapeContext<'_>,
    elapsed: f32,
) {
    member.target = source.target;
    let position = source.position;
    let moved = member
        .draped_at
        .distance_to(Vector2::new(position.x, position.z));
    let health_visible = draw.alpha > 0.0;
    let settle_refresh = member.drapes == 1 && elapsed - member.draped_for >= REDRAPE_SETTLE_SECS;
    let needs_drape = member.heights.is_none() || moved > REDRAPE_DISTANCE || settle_refresh;
    if needs_drape {
        context.samples += (2 * ARC_STATIONS) as u64;
        let (anchor, heights) = drape_ring(
            context,
            position,
            ARC_STATIONS,
            HEALTH_INNER_RADIUS,
            HEALTH_OUTER_RADIUS,
        );
        member.anchor = anchor;
        member.heights = Some(heights);
        member.built_arc = None;
        if draw.dispel.is_some() || member.dispel_heights.is_none() {
            context.samples += (2 * DISPEL_STATIONS) as u64;
            let (_, heights) = drape_ring(
                context,
                position,
                DISPEL_STATIONS,
                DISPEL_INNER_RADIUS,
                DISPEL_OUTER_RADIUS,
            );
            member.dispel_heights = Some(heights);
            member.built_dis = None;
        }
        member.draped_at = Vector2::new(position.x, position.z);
        member.draped_for = elapsed;
        member.drapes += 1;
    }
    if health_visible {
        update_health_mesh(member, draw);
    }
    let shown = health_visible && member.heights.is_some();
    if member.shown != shown {
        member.mesh.set_visible(shown);
        member.shown = shown;
    }
    update_dispel_ring(member, draw, elapsed);
    member.drawn = Some(*draw);
    let at = Vector3::new(position.x, member.anchor.y, position.z);
    if member.node_at != at {
        member.node.set_position(at);
        member.node_at = at;
    }
}

/// Height of a station pair, interpolated between the draped stations.
fn station_heights(heights: &[[f32; 2]], angle: f32) -> [f32; 2] {
    let stations = heights.len();
    let step = TAU / stations as f32;
    let position = (angle.rem_euclid(TAU) / step).rem_euclid(stations as f32);
    let low = position.floor() as usize % stations;
    let high = (low + 1) % stations;
    let blend = position - position.floor();
    [
        heights[low][0] + (heights[high][0] - heights[low][0]) * blend,
        heights[low][1] + (heights[high][1] - heights[low][1]) * blend,
    ]
}

fn update_health_mesh(member: &mut MemberRing, draw: &RingDraw) {
    let Some(heights) = &member.heights else {
        return;
    };
    let rebuilt = member.built_arc.is_none_or(|(start, color)| {
        (start - draw.arc_start).abs() > 1e-3
            || color
                .iter()
                .zip(draw.color.iter())
                .any(|(left, right)| (left - right).abs() > 0.01)
    });
    if member.applied_color != draw.color || member.applied_alpha != draw.alpha {
        member.material.set_albedo(Color::from_rgba(
            draw.color[0],
            draw.color[1],
            draw.color[2],
            draw.alpha,
        ));
        member.applied_alpha = draw.alpha;
        member.applied_color = draw.color;
    }
    if !rebuilt {
        return;
    }
    let angles = arc_stations(draw.arc_start, draw.arc_end);
    let mut vertices: Vec<Vector3> = Vec::with_capacity(angles.len() * 2);
    for &angle in &angles {
        let [inner_y, outer_y] = station_heights(heights, angle);
        let (inner_x, inner_z) = station_offset(angle, HEALTH_INNER_RADIUS);
        let (outer_x, outer_z) = station_offset(angle, HEALTH_OUTER_RADIUS);
        vertices.push(Vector3::new(inner_x, inner_y, inner_z));
        vertices.push(Vector3::new(outer_x, outer_y, outer_z));
    }
    let mut indices: Vec<i32> = Vec::with_capacity(angles.len().saturating_sub(1) * 6);
    for station in 0..angles.len().saturating_sub(1) {
        let inner = (station * 2) as i32;
        let outer = inner + 1;
        indices.extend([inner, outer, inner + 2, inner, outer + 1, inner + 2]);
    }
    let normals = vec![Vector3::UP; vertices.len()];
    let mut mesh = ArrayMesh::new_gd();
    add_surface(&mut mesh, &vertices, &normals, &indices);
    member.mesh.set_mesh(&mesh);
    member.built_arc = Some((draw.arc_start, draw.color));
    member.mesh_vertices = vertices.len();
    let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
    for vertex in &vertices {
        let world = member.anchor.y + vertex.y;
        low = low.min(world);
        high = high.max(world);
    }
    member.mesh_bounds = Some((low, high));
}

fn update_dispel_ring(member: &mut MemberRing, draw: &RingDraw, elapsed: f32) {
    let Some(school) = draw.dispel else {
        if member.dispel_shown {
            member.dispel_mesh.set_visible(false);
            member.dispel_shown = false;
        }
        member.built_dis = None;
        return;
    };
    if member.built_dis != Some(school) || member.dispel_heights.is_none() {
        let Some(heights) = &member.dispel_heights else {
            member.dispel_mesh.set_visible(false);
            return;
        };
        let mut vertices: Vec<Vector3> = Vec::with_capacity(DISPEL_STATIONS * 2);
        for (station, height) in heights.iter().enumerate() {
            let angle = TAU * station as f32 / DISPEL_STATIONS as f32;
            let (inner_x, inner_z) = station_offset(angle, DISPEL_INNER_RADIUS);
            let (outer_x, outer_z) = station_offset(angle, DISPEL_OUTER_RADIUS);
            vertices.push(Vector3::new(inner_x, height[0], inner_z));
            vertices.push(Vector3::new(outer_x, height[1], outer_z));
        }
        let mut indices: Vec<i32> = Vec::with_capacity(DISPEL_STATIONS * 6);
        for station in 0..DISPEL_STATIONS {
            let next = (station + 1) % DISPEL_STATIONS;
            let inner = (station * 2) as i32;
            let outer = inner + 1;
            let next_inner = (next * 2) as i32;
            let next_outer = next_inner + 1;
            indices.extend([inner, outer, next_outer, inner, next_outer, next_inner]);
        }
        let normals = vec![Vector3::UP; vertices.len()];
        let mut mesh = ArrayMesh::new_gd();
        add_surface(&mut mesh, &vertices, &normals, &indices);
        member.dispel_mesh.set_mesh(&mesh);
        member.built_dis = Some(school);
    }
    let tint = dispel_color(school);
    // Quantized to 32 steps: the pulse is identical to the eye and the material only
    // changes a few times per second.
    let alpha = (dispel_pulse(elapsed) * 32.0).round() / 32.0;
    if member.applied_dispel_alpha != alpha {
        member
            .dispel_material
            .set_albedo(Color::from_rgba(tint[0], tint[1], tint[2], alpha));
        member.applied_dispel_alpha = alpha;
    }
    if !member.dispel_shown {
        member.dispel_mesh.set_visible(true);
        member.dispel_shown = true;
    }
}

fn add_surface(
    mesh: &mut Gd<ArrayMesh>,
    vertices: &[Vector3],
    normals: &[Vector3],
    indices: &[i32],
) {
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &PackedVector3Array::from(vertices).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::NORMAL.ord() as usize,
        &PackedVector3Array::from(normals).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::INDEX.ord() as usize,
        &PackedInt32Array::from(indices).to_variant(),
    );
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
}

fn dispel_color(school: DispelSchool) -> [f32; 3] {
    let debuff = game_engine_ui_model::aura_display_data::DebuffType::from_dispel_id(school.id());
    parse_rgb(debuff.border_color())
}

fn parse_rgb(text: &str) -> [f32; 3] {
    let mut parts = text
        .split(',')
        .filter_map(|part| part.trim().parse::<f32>().ok());
    [
        parts.next().unwrap_or(1.0),
        parts.next().unwrap_or(1.0),
        parts.next().unwrap_or(1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispel_colors_are_the_debuff_type_borders() {
        assert_eq!(dispel_color(DispelSchool::Magic), [0.2, 0.6, 1.0]);
        assert_eq!(dispel_color(DispelSchool::Poison), [0.0, 0.6, 0.0]);
    }

    #[test]
    fn station_heights_interpolate_and_wrap() {
        let heights = vec![[0.0, 0.1], [0.2, 0.3], [0.4, 0.5], [0.6, 0.7]];
        assert_eq!(station_heights(&heights, 0.0), [0.0, 0.1]);
        let quarter = TAU / 4.0;
        assert_eq!(station_heights(&heights, quarter), [0.2, 0.3]);
        let eighth = TAU / 8.0;
        let middle = station_heights(&heights, eighth);
        assert!((middle[0] - 0.1).abs() < 1e-5);
        assert!((middle[1] - 0.2).abs() < 1e-5);
        assert_eq!(station_heights(&heights, TAU), [0.0, 0.1]);
    }

    #[test]
    fn parse_rgb_reads_the_retail_color_strings() {
        assert_eq!(parse_rgb("1.0,0.55,0.0,1.0"), [1.0, 0.55, 0.0]);
        assert_eq!(parse_rgb("broken"), [1.0, 1.0, 1.0]);
    }
}
