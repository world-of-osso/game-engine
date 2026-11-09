//! WMO group visibility through portals, ported from the original client's
//! `src/rendering/camera/culling.rs` (`cull_wmo_portal_visibility`), itself the
//! Retail/WebWowViewerCpp traversal: inside an interior group, draw what its portals
//! reach and the whole exterior once a portal opens onto it; otherwise draw every
//! exterior group and the interiors whose portals are in view.

use std::collections::VecDeque;

use game_engine_core::{
    asset::wmo_format::fog::{WmoFogBlend, WmoFogVolume},
    wmo,
};
use glam::{Affine3A, Vec3};

use super::assets::{NativeWmoAsset, NativeWmoGroup};

/// Distance from a portal's plane within which the portal counts as visible whatever
/// the frustum says (WebWowViewerCpp `dotepsilon`, 1.5², as the original client).
const PORTAL_NEAR_DISTANCE: f32 = 2.25;
/// Portal crossings a traversal follows from a start group (WebWowViewerCpp `localLevel > 8`).
const MAX_PORTAL_HOPS: u32 = 8;
const ROOT_ALWAYS_DRAW_MASK: u32 = 0x2001_0008;
pub(super) const ROOT_ANTIPORTAL: u32 = 0x0400_0000;
const SPLIT_GROUP_PARENT: u32 = 0x40;
const SPLIT_GROUP_CHILD: u32 = 0x80;

/// WMO-local file axes to engine axes, the conversion the shared mesh batches use.
fn engine_axes([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, z, -y)
}

/// Engine axes back to WMO-local file axes (Z up).
fn file_axes(local: Vec3) -> [f32; 3] {
    [local.x, -local.z, local.y]
}

/// A frustum plane; points with `normal.dot(p) + d >= 0` are inside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HalfSpace {
    pub normal: Vec3,
    pub d: f32,
}

struct PortalGroup {
    index: u16,
    bbox_min: Vec3,
    bbox_max: Vec3,
    exterior: bool,
    root_flags: u32,
    split_parent: Option<u16>,
    first_child: Option<u16>,
    next_child: Option<u16>,
    /// Render triangles of an interior group, to tell whether the camera stands inside
    /// the group or only inside its bounding box. Empty for exterior groups.
    floor: Vec<[Vec3; 3]>,
}

/// Visibility and exterior windows from the same camera and portal traversal.
pub(crate) struct WmoVisibility {
    pub groups: Vec<bool>,
    pub exterior_windows: Option<Vec<Vec<HalfSpace>>>,
}

/// Portal graph and group volumes of one WMO, in WMO-local engine axes.
pub(crate) struct WmoPortals {
    groups: Vec<PortalGroup>,
    /// Per group index: (portal index, destination group, MOPR side of the source group).
    adjacency: Vec<Vec<(usize, u16, i16)>>,
    portal_vertices: Vec<Vec<Vec3>>,
    /// Per portal: its MOPT plane through its vertices, WMO-local engine axes.
    portal_planes: Vec<HalfSpace>,
    /// The last local camera position and the interior group it lies in: the portal
    /// cull and the camera fog ask for the same position each frame, and a still camera
    /// asks again on every frame.
    camera_group: std::cell::Cell<Option<(Vec3, Option<u16>)>>,
}

impl WmoPortals {
    /// Drawable (non-antiportal) groups of `asset` with MOGI bounds and MOPR links.
    pub fn new(asset: &NativeWmoAsset) -> Self {
        let root = &asset.root;
        let groups = asset
            .groups
            .iter()
            .filter(|group| {
                let info = root
                    .group_infos
                    .get(group.index as usize)
                    .expect("loaded WMO group has no root MOGI record");
                !group.group.header.group_flags.antiportal && info.flags & ROOT_ANTIPORTAL == 0
            })
            .map(|group| build_portal_group(root, group))
            .collect();
        let adjacency = build_portal_adjacency(root);
        let portal_vertices: Vec<Vec<Vec3>> = root
            .portals
            .iter()
            .map(|portal| portal.vertices.iter().copied().map(engine_axes).collect())
            .collect();
        let portal_planes = root
            .portals
            .iter()
            .zip(&portal_vertices)
            .map(|(portal, vertices)| {
                let normal = engine_axes(portal.normal);
                let d = vertices.first().map_or(0.0, |vertex| -normal.dot(*vertex));
                HalfSpace { normal, d }
            })
            .collect();
        Self {
            groups,
            adjacency,
            portal_vertices,
            portal_planes,
            camera_group: std::cell::Cell::new(None),
        }
    }

    /// Group visibility and exterior windows for one camera and frustum.
    pub fn visibility(
        &self,
        world_from_local: Affine3A,
        frustum: &[HalfSpace],
        camera: Vec3,
    ) -> WmoVisibility {
        let local_camera = world_from_local.inverse().transform_point3(camera);
        let interior = self.camera_interior_group(local_camera);
        let exterior_windows = interior.map(|group| {
            self.exterior_frusta_from_group(
                self.split_parent(group),
                world_from_local,
                frustum,
                camera,
            )
        });
        let exterior: Vec<u16> = self
            .groups
            .iter()
            .filter(|group| group.exterior)
            .map(|group| group.index)
            .collect();
        let traverse = |start: &[u16]| self.traverse(start, world_from_local, frustum, camera);
        let mut visible = match interior {
            Some(group) => {
                // The camera can stand inside an interior group's volume without a floor
                // triangle below it (over lava, water or a floor owned by another group).
                // Retail's camera query returns the groups the camera is inside, so start
                // the traversal from each of them as well; otherwise a chamber entered
                // through an unportalled opening is culled and its walls and ceiling show
                // sky. Neltharus' Magmatusk chamber (groups 2 and 3) is such a case.
                let mut starts = vec![self.split_parent(group)];
                for candidate in &self.groups {
                    if candidate.exterior
                        || local_camera.cmplt(candidate.bbox_min).any()
                        || local_camera.cmpgt(candidate.bbox_max).any()
                        || !group_intersects_frustum(candidate, world_from_local, frustum)
                    {
                        continue;
                    }
                    let parent = self.split_parent(candidate.index);
                    if !starts.contains(&parent) {
                        starts.push(parent);
                    }
                }
                let mut visible = traverse(&starts);
                if exterior_windows
                    .as_ref()
                    .is_some_and(|windows| !windows.is_empty())
                {
                    for (drawn, outside) in visible.iter_mut().zip(traverse(&exterior)) {
                        *drawn |= outside;
                    }
                }
                visible
            }
            None => traverse(&exterior),
        };
        for group in &self.groups {
            if visible.get(group.index as usize).copied().unwrap_or(false) {
                self.show_split_children(group.index, &mut visible);
            }
        }
        for group in &self.groups {
            if group.root_flags & ROOT_ALWAYS_DRAW_MASK != 0
                && group_intersects_frustum(group, world_from_local, frustum)
                && let Some(drawn) = visible.get_mut(group.index as usize)
            {
                *drawn = true;
            }
        }
        WmoVisibility {
            groups: visible,
            exterior_windows,
        }
    }

    /// A camera in a split child traverses the parent and draws all of its children
    /// (WebWowViewerCpp `startTraversingWMOGroup` / `addSplitChildWMOsToView`).
    fn split_parent(&self, index: u16) -> u16 {
        self.groups
            .iter()
            .find(|group| group.index == index)
            .and_then(|group| group.split_parent)
            .unwrap_or(index)
    }

    fn show_split_children(&self, parent: u16, visible: &mut [bool]) {
        let mut child = self
            .groups
            .iter()
            .find(|group| group.index == parent)
            .and_then(|group| group.first_child);
        for _ in 0..self.groups.len() {
            let Some(group) =
                child.and_then(|index| self.groups.iter().find(|group| group.index == index))
            else {
                break;
            };
            if let Some(drawn) = visible.get_mut(group.index as usize) {
                *drawn = true;
            }
            child = group.next_child;
        }
    }

    /// Clip every exterior path from the camera's interior group through its portals.
    /// A camera close to a portal keeps its current frustum to avoid doorway pop-in.
    fn exterior_frusta_from_group(
        &self,
        group: u16,
        world_from_local: Affine3A,
        frustum: &[HalfSpace],
        camera: Vec3,
    ) -> Vec<Vec<HalfSpace>> {
        let mut windows = Vec::new();
        let mut pending = vec![(group, frustum.to_vec(), vec![group])];
        while let Some((current, planes, path)) = pending.pop() {
            for &(portal, destination, _) in
                self.adjacency.get(current as usize).into_iter().flatten()
            {
                if path.contains(&destination) {
                    continue;
                }
                let Some(vertices) = self.portal_vertices.get(portal) else {
                    continue;
                };
                let polygon: Vec<_> = vertices
                    .iter()
                    .map(|vertex| world_from_local.transform_point3(*vertex))
                    .collect();
                let Some(clipped) = portal_window(&polygon, &planes, camera) else {
                    continue;
                };
                let Some(group) = self.groups.iter().find(|group| group.index == destination)
                else {
                    continue;
                };
                if group.exterior {
                    windows.push(clipped);
                } else {
                    let mut next_path = path.clone();
                    next_path.push(destination);
                    pending.push((destination, clipped, next_path));
                }
            }
        }
        windows
    }

    /// Conservative world bounds of the interior groups of this WMO.
    pub(crate) fn interior_bounds(&self, world_from_local: Affine3A) -> Vec<(Vec3, Vec3)> {
        self.groups
            .iter()
            .filter(|group| !group.exterior)
            .map(|group| {
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                for bits in 0..8 {
                    let corner = Vec3::new(
                        if bits & 1 == 0 {
                            group.bbox_min.x
                        } else {
                            group.bbox_max.x
                        },
                        if bits & 2 == 0 {
                            group.bbox_min.y
                        } else {
                            group.bbox_max.y
                        },
                        if bits & 4 == 0 {
                            group.bbox_min.z
                        } else {
                            group.bbox_max.z
                        },
                    );
                    let point = world_from_local.transform_point3(corner);
                    min = min.min(point);
                    max = max.max(point);
                }
                (min, max)
            })
            .collect()
    }

    /// The MFOG fog at world `camera`: WebWowViewerCpp picks the group the camera stands
    /// in (`map.cpp:487-512`) and asks it for `checkFog` (`DayNightLightHolder.cpp:390-396`).
    /// The group comes from `camera_interior_group`, not the reference's BSP query.
    pub fn camera_fog(
        &self,
        fog: &WmoFogVolume,
        world_from_local: Affine3A,
        camera: Vec3,
    ) -> Option<WmoFogBlend> {
        let local_camera = world_from_local.inverse().transform_point3(camera);
        let group = self.camera_interior_group(local_camera)?;
        fog.camera_fog(group, file_axes(local_camera))
    }

    /// Breadth-first from `start` (each seen through `frustum`) through the portals in
    /// view; per group index, whether it was reached. Each crossing narrows the view to
    /// the portal's window, so a portal counts only when it is seen through every portal
    /// before it, and only from the side of the group it leaves (WebWowViewerCpp
    /// `drainPortalTraversal`: MOPR side, portal planes per hop, at most 8 hops, an almost
    /// equal frustum crosses a portal once).
    fn traverse(
        &self,
        start: &[u16],
        world_from_local: Affine3A,
        frustum: &[HalfSpace],
        camera: Vec3,
    ) -> Vec<bool> {
        let mut reached = vec![false; self.adjacency.len()];
        let mut queue = VecDeque::with_capacity(start.len());
        for &group in start {
            if let Some(slot) = reached.get_mut(group as usize) {
                *slot = true;
                queue.push_back((group, frustum.to_vec(), 0));
            }
        }
        let local_camera = world_from_local.inverse().transform_point3(camera);
        let mut crossed: Vec<Vec<Vec<HalfSpace>>> = vec![Vec::new(); self.portal_vertices.len()];
        while let Some((current, planes, hops)) = queue.pop_front() {
            if hops == MAX_PORTAL_HOPS {
                continue;
            }
            for &(portal, destination, side) in &self.adjacency[current as usize] {
                if reached.get(destination as usize).is_none()
                    || !self.camera_on_side(portal, side, local_camera)
                {
                    continue;
                }
                let Some(window) = self.portal_window(portal, world_from_local, &planes, camera)
                else {
                    continue;
                };
                if crossed[portal]
                    .iter()
                    .any(|seen| frusta_almost_equal(seen, &window))
                {
                    continue;
                }
                crossed[portal].push(window.clone());
                reached[destination as usize] = true;
                queue.push_back((destination, window, hops + 1));
            }
        }
        reached
    }

    /// Whether the local camera is on the `side` (MOPR) of `portal`'s plane, or within
    /// `PORTAL_NEAR_DISTANCE` of it (WebWowViewerCpp `isInsidePortalThis` and `dotepsilon`).
    fn camera_on_side(&self, portal: usize, side: i16, local_camera: Vec3) -> bool {
        let Some(plane) = self.portal_planes.get(portal) else {
            return true;
        };
        let distance = plane.normal.dot(local_camera) + plane.d;
        distance.abs() <= PORTAL_NEAR_DISTANCE
            || if side < 0 {
                distance <= 0.0
            } else {
                distance >= 0.0
            }
    }

    /// The view through `portal` from inside `frustum`, if any of it is in view.
    fn portal_window(
        &self,
        portal: usize,
        world_from_local: Affine3A,
        frustum: &[HalfSpace],
        camera: Vec3,
    ) -> Option<Vec<HalfSpace>> {
        let polygon: Vec<Vec3> = self
            .portal_vertices
            .get(portal)?
            .iter()
            .map(|vertex| world_from_local.transform_point3(*vertex))
            .collect();
        portal_window(&polygon, frustum, camera)
    }

    /// The interior group the camera stands in: its bounding box contains the camera
    /// and one of its triangles lies below it; the highest such floor wins
    /// (WebWowViewerCpp `getGroupWmoThatCameraIsInside`).
    fn camera_interior_group(&self, local_camera: Vec3) -> Option<u16> {
        if let Some((camera, group)) = self.camera_group.get()
            && camera == local_camera
        {
            return group;
        }
        let group = self.find_camera_interior_group(local_camera);
        self.camera_group.set(Some((local_camera, group)));
        group
    }

    fn find_camera_interior_group(&self, local_camera: Vec3) -> Option<u16> {
        self.groups
            .iter()
            .filter(|group| {
                !group.exterior
                    && local_camera.cmpge(group.bbox_min).all()
                    && local_camera.cmple(group.bbox_max).all()
            })
            .filter_map(|group| {
                Some((group.index, floor_height_below(&group.floor, local_camera)?))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}

fn build_portal_group(root: &wmo::WmoRootData, group: &NativeWmoGroup) -> PortalGroup {
    let (bbox_min, bbox_max) = root
        .group_infos
        .get(group.index as usize)
        .map(|info| {
            let (a, b) = (engine_axes(info.bbox_min), engine_axes(info.bbox_max));
            (a.min(b), a.max(b))
        })
        .unwrap_or((Vec3::splat(f32::MIN), Vec3::splat(f32::MAX)));
    let exterior = group.group.header.group_flags.exterior;
    let floor = if exterior {
        Vec::new()
    } else {
        collect_group_floor(group)
    };
    PortalGroup {
        index: group.index as u16,
        bbox_min,
        bbox_max,
        exterior,
        root_flags: root.group_infos[group.index as usize].flags,
        split_parent: (group.group.header.flags2 & SPLIT_GROUP_CHILD != 0)
            .then(|| u16::try_from(group.group.header.parent_split_group_index).ok())
            .flatten(),
        first_child: (group.group.header.flags2 & SPLIT_GROUP_PARENT != 0)
            .then(|| u16::try_from(group.group.header.parent_split_group_index).ok())
            .flatten(),
        next_child: u16::try_from(group.group.header.next_split_child_group_index).ok(),
        floor,
    }
}

fn group_intersects_frustum(
    group: &PortalGroup,
    world_from_local: Affine3A,
    frustum: &[HalfSpace],
) -> bool {
    let corners: [Vec3; 8] = std::array::from_fn(|bits| {
        let corner = Vec3::new(
            if bits & 1 == 0 {
                group.bbox_min.x
            } else {
                group.bbox_max.x
            },
            if bits & 2 == 0 {
                group.bbox_min.y
            } else {
                group.bbox_max.y
            },
            if bits & 4 == 0 {
                group.bbox_min.z
            } else {
                group.bbox_max.z
            },
        );
        world_from_local.transform_point3(corner)
    });
    frustum.iter().all(|plane| {
        corners
            .iter()
            .any(|corner| plane.normal.dot(*corner) + plane.d >= 0.0)
    })
}

fn collect_group_floor(group: &NativeWmoGroup) -> Vec<[Vec3; 3]> {
    group
        .batches
        .iter()
        .flat_map(|batch| {
            batch.indices.chunks_exact(3).map(|triangle| {
                [0, 1, 2].map(|corner| Vec3::from(batch.positions[triangle[corner] as usize]))
            })
        })
        .collect()
}

fn build_portal_adjacency(root: &wmo::WmoRootData) -> Vec<Vec<(usize, u16, i16)>> {
    let mut groups_by_portal = vec![Vec::new(); root.portals.len()];
    for portal_ref in &root.portal_refs {
        if let Some(groups) = groups_by_portal.get_mut(portal_ref.portal_index as usize) {
            groups.push((portal_ref.group_index, portal_ref.side));
        }
    }
    let mut adjacency = vec![Vec::new(); root.n_groups as usize];
    for (portal, groups) in groups_by_portal.iter().enumerate() {
        for &(source, _) in groups {
            let Some(neighbors) = adjacency.get_mut(source as usize) else {
                continue;
            };
            // A group's MOGP portal range lists its portals by the group across them, with
            // the side of the listing group: the entry naming the destination is the source's.
            neighbors.extend(
                groups
                    .iter()
                    .filter(|&&(destination, _)| destination != source)
                    .map(|&(destination, side)| (portal, destination, side)),
            );
        }
    }
    adjacency
}

/// Equal plane for plane within 1e-3 of the larger magnitude (WebWowViewerCpp
/// `frustumPlanesAlmostEqual`).
fn frusta_almost_equal(a: &[HalfSpace], b: &[HalfSpace]) -> bool {
    let close = |x: f32, y: f32| (x - y).abs() <= 1e-3 * x.abs().max(y.abs()).max(1.0);
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            close(a.normal.x, b.normal.x)
                && close(a.normal.y, b.normal.y)
                && close(a.normal.z, b.normal.z)
                && close(a.d, b.d)
        })
}

fn camera_near_plane(polygon: &[Vec3], camera: Vec3) -> bool {
    let normal = polygon_normal(polygon);
    normal != Vec3::ZERO && normal.dot(camera - polygon[0]).abs() <= PORTAL_NEAR_DISTANCE
}

/// Clip a portal to the current view, then add camera-to-edge planes.
fn portal_window(polygon: &[Vec3], frustum: &[HalfSpace], camera: Vec3) -> Option<Vec<HalfSpace>> {
    if polygon.len() < 3 {
        return None;
    }
    if camera_near_plane(polygon, camera) {
        return Some(frustum.to_vec());
    }
    let mut clipped = polygon.to_vec();
    for plane in frustum {
        clipped = clip_polygon(&clipped, |point| plane.normal.dot(point) + plane.d);
        if clipped.len() < 3 {
            return None;
        }
    }
    let center = clipped.iter().copied().sum::<Vec3>() / clipped.len() as f32;
    let mut planes = frustum.to_vec();
    for (&a, &b) in clipped.iter().zip(clipped.iter().cycle().skip(1)) {
        let mut normal = (a - camera).cross(b - camera).normalize_or_zero();
        if normal == Vec3::ZERO {
            continue;
        }
        if normal.dot(center - camera) < 0.0 {
            normal = -normal;
        }
        planes.push(HalfSpace {
            normal,
            d: -normal.dot(camera),
        });
    }
    Some(planes)
}

/// Unit normal of a planar polygon (Newell's method), zero when degenerate.
fn polygon_normal(polygon: &[Vec3]) -> Vec3 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .fold(Vec3::ZERO, |normal, (a, b)| {
            normal
                + Vec3::new(
                    (a.y - b.y) * (a.z + b.z),
                    (a.z - b.z) * (a.x + b.x),
                    (a.x - b.x) * (a.y + b.y),
                )
        })
        .normalize_or_zero()
}

fn clip_polygon(polygon: &[Vec3], signed_distance: impl Fn(Vec3) -> f32) -> Vec<Vec3> {
    let mut clipped = Vec::with_capacity(polygon.len() + 2);
    for (&a, &b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        let (da, db) = (signed_distance(a), signed_distance(b));
        if da >= 0.0 {
            clipped.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            clipped.push(a.lerp(b, da / (da - db)));
        }
    }
    clipped
}

/// Height of the highest triangle directly below `point` (Y up).
fn floor_height_below(floor: &[[Vec3; 3]], point: Vec3) -> Option<f32> {
    floor
        .iter()
        .filter_map(|triangle| triangle_height_at(triangle, point.x, point.z))
        .filter(|height| *height <= point.y)
        .reduce(f32::max)
}

fn triangle_height_at(triangle: &[Vec3; 3], x: f32, z: f32) -> Option<f32> {
    let [a, b, c] = *triangle;
    let det = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
    if det.abs() <= f32::EPSILON {
        return None;
    }
    let wa = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / det;
    let wb = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / det;
    let wc = 1.0 - wa - wb;
    (wa >= 0.0 && wb >= 0.0 && wc >= 0.0).then_some(wa * a.y + wb * b.y + wc * c.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    impl WmoPortals {
        fn visible_groups(
            &self,
            world_from_local: Affine3A,
            frustum: &[HalfSpace],
            camera: Vec3,
        ) -> Vec<bool> {
            self.visibility(world_from_local, frustum, camera).groups
        }

        fn exterior_frusta(
            &self,
            world_from_local: Affine3A,
            frustum: &[HalfSpace],
            camera: Vec3,
        ) -> Option<Vec<Vec<HalfSpace>>> {
            self.visibility(world_from_local, frustum, camera)
                .exterior_windows
        }
    }

    /// Exterior group 0 (a courtyard) and interior group 1 (a 10x4x10 room at
    /// x,z in 0..10), joined by a 2x2 doorway portal in the plane z = 0.
    fn courtyard_and_room() -> WmoPortals {
        let floor = vec![
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 10.0),
            ],
            [
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(0.0, 0.0, 10.0),
            ],
        ];
        WmoPortals {
            groups: vec![
                PortalGroup {
                    index: 0,
                    bbox_min: Vec3::new(-50.0, -1.0, -50.0),
                    bbox_max: Vec3::new(50.0, 20.0, 0.0),
                    exterior: true,
                    root_flags: 0,
                    split_parent: None,
                    first_child: None,
                    next_child: None,
                    floor: Vec::new(),
                },
                PortalGroup {
                    index: 1,
                    bbox_min: Vec3::new(0.0, -1.0, 0.0),
                    bbox_max: Vec3::new(10.0, 4.0, 10.0),
                    exterior: false,
                    root_flags: 0,
                    split_parent: None,
                    first_child: None,
                    next_child: None,
                    floor,
                },
            ],
            adjacency: vec![vec![(0, 1, 0)], vec![(0, 0, 0)]],
            portal_vertices: vec![vec![
                Vec3::new(4.0, 0.0, 0.0),
                Vec3::new(6.0, 0.0, 0.0),
                Vec3::new(6.0, 2.0, 0.0),
                Vec3::new(4.0, 2.0, 0.0),
            ]],
            portal_planes: Vec::new(),
            camera_group: std::cell::Cell::new(None),
        }
    }

    /// Four side planes of a 90 degree view from `eye` looking along `forward` (XZ).
    fn view(eye: Vec3, forward: Vec3) -> Vec<HalfSpace> {
        let right = forward.cross(Vec3::Y);
        [
            (forward + right).normalize(),
            (forward - right).normalize(),
            (forward + Vec3::Y).normalize(),
            (forward - Vec3::Y).normalize(),
        ]
        .map(|normal| HalfSpace {
            normal,
            d: -normal.dot(eye),
        })
        .to_vec()
    }

    fn drawn_indices(drawn: Vec<bool>) -> Vec<u16> {
        (0..drawn.len() as u16)
            .filter(|&group| drawn[group as usize])
            .collect()
    }

    fn visible(eye: Vec3, forward: Vec3) -> Vec<u16> {
        drawn_indices(courtyard_and_room().visible_groups(
            Affine3A::IDENTITY,
            &view(eye, forward),
            eye,
        ))
    }

    /// `courtyard_and_room` plus interior group 2 (x,z in 0..10 x 10..20) behind room 1,
    /// joined to it by a 1x2 doorway at x 0..1 in the plane z = 10.
    fn courtyard_and_two_rooms() -> WmoPortals {
        let mut portals = courtyard_and_room();
        portals.groups.push(PortalGroup {
            index: 2,
            bbox_min: Vec3::new(0.0, -1.0, 10.0),
            bbox_max: Vec3::new(10.0, 4.0, 20.0),
            exterior: false,
            root_flags: 0,
            split_parent: None,
            first_child: None,
            next_child: None,
            floor: Vec::new(),
        });
        portals.adjacency = vec![vec![(0, 1, 0)], vec![(0, 0, 0), (1, 2, 0)], vec![(1, 1, 0)]];
        portals.portal_vertices.push(vec![
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(1.0, 0.0, 10.0),
            Vec3::new(1.0, 2.0, 10.0),
            Vec3::new(0.0, 2.0, 10.0),
        ]);
        portals
    }

    #[test]
    fn a_room_whose_doorway_is_outside_the_first_doorway_window_is_not_drawn() {
        let portals = courtyard_and_two_rooms();
        let visible = |eye: Vec3| {
            drawn_indices(portals.visible_groups(Affine3A::IDENTITY, &view(eye, Vec3::Z), eye))
        };
        // From 20 yards out the front doorway's window spans x 3.5..6.5 at z = 10: the
        // back doorway at x 0..1 is in the view frustum but not through the front door.
        assert_eq!(visible(Vec3::new(5.0, 1.0, -20.0)), [0, 1]);
        // From the right the window spans x -2.5..3.5 at z = 10: the doorways line up.
        assert_eq!(visible(Vec3::new(7.25, 1.0, -5.0)), [0, 1, 2]);
    }

    /// `courtyard_and_two_rooms` plus a back door from room 2 to the courtyard (portal 2,
    /// x 4..6 in the plane z = 20), with MOPR sides: each portal's plane faces +z, the
    /// group in front of the camera's approach on its -z side.
    fn building_with_back_door() -> WmoPortals {
        let mut portals = courtyard_and_two_rooms();
        portals.adjacency = vec![
            vec![(0, 1, -1), (2, 2, 1)],
            vec![(0, 0, 1), (1, 2, -1)],
            vec![(1, 1, 1), (2, 0, -1)],
        ];
        portals.portal_vertices.push(vec![
            Vec3::new(4.0, 0.0, 20.0),
            Vec3::new(6.0, 0.0, 20.0),
            Vec3::new(6.0, 2.0, 20.0),
            Vec3::new(4.0, 2.0, 20.0),
        ]);
        portals.portal_planes = [0.0, -10.0, -20.0]
            .map(|d| HalfSpace { normal: Vec3::Z, d })
            .to_vec();
        portals
    }

    #[test]
    fn a_door_seen_from_behind_does_not_open_its_room() {
        let portals = building_with_back_door();
        let visible = |eye: Vec3, forward: Vec3| {
            drawn_indices(portals.visible_groups(Affine3A::IDENTITY, &view(eye, forward), eye))
        };
        // From the front the back door is in view but faces away: room 2 stays hidden.
        assert_eq!(visible(Vec3::new(5.0, 1.0, -20.0), Vec3::Z), [0, 1]);
        // From behind, the front door faces away: room 1 stays hidden.
        assert_eq!(visible(Vec3::new(5.0, 1.0, 40.0), -Vec3::Z), [0, 2]);
    }

    #[test]
    fn inside_camera_returns_matching_groups_and_exterior_windows() {
        let portals = courtyard_and_room();
        let eye = Vec3::new(5.0, 1.0, 8.0);
        let open = portals.visibility(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye);
        assert_eq!(drawn_indices(open.groups), [0, 1]);
        assert_eq!(open.exterior_windows.unwrap().len(), 1);

        let closed = portals.visibility(Affine3A::IDENTITY, &view(eye, Vec3::Z), eye);
        assert_eq!(drawn_indices(closed.groups), [1]);
        assert!(closed.exterior_windows.unwrap().is_empty());
    }

    #[test]
    fn outdoor_windows_clip_to_the_door_and_close_when_facing_away() {
        let portals = courtyard_and_room();
        let eye = Vec3::new(5.0, 1.0, 8.0);
        let windows = portals
            .exterior_frusta(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye)
            .unwrap();
        assert_eq!(windows.len(), 1);
        let inside = Vec3::new(5.0, 1.0, -8.0);
        let outside = Vec3::new(12.0, 1.0, -8.0);
        assert!(windows[0].iter().all(|p| p.normal.dot(inside) + p.d >= 0.0));
        assert!(windows[0].iter().any(|p| p.normal.dot(outside) + p.d < 0.0));
        assert!(
            portals
                .exterior_frusta(Affine3A::IDENTITY, &view(eye, Vec3::Z), eye)
                .unwrap()
                .is_empty()
        );
        let outside_eye = Vec3::new(5.0, 1.0, -20.0);
        assert!(
            portals
                .exterior_frusta(Affine3A::IDENTITY, &view(outside_eye, Vec3::Z), outside_eye)
                .is_none()
        );
    }

    #[test]
    fn split_child_camera_uses_parent_exterior_portal() {
        let mut portals = courtyard_and_room();
        let parent = &mut portals.groups[1];
        let child = PortalGroup {
            index: 2,
            bbox_min: parent.bbox_min,
            bbox_max: parent.bbox_max,
            exterior: false,
            root_flags: 0,
            split_parent: Some(1),
            first_child: None,
            next_child: None,
            floor: std::mem::take(&mut parent.floor),
        };
        parent.first_child = Some(2);
        portals.groups.push(child);
        portals.adjacency.push(Vec::new());
        let eye = Vec3::new(5.0, 1.0, 8.0);
        let windows = portals
            .exterior_frusta(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye)
            .unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(
            drawn_indices(portals.visible_groups(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye)),
            [0, 1, 2]
        );
    }

    #[test]
    fn portal_reached_split_parent_draws_its_children() {
        let mut portals = courtyard_and_room();
        portals.groups[1].first_child = Some(2);
        portals.groups.push(PortalGroup {
            index: 2,
            bbox_min: portals.groups[1].bbox_min,
            bbox_max: portals.groups[1].bbox_max,
            exterior: false,
            root_flags: 0,
            split_parent: Some(1),
            first_child: None,
            next_child: None,
            floor: Vec::new(),
        });
        portals.adjacency.push(Vec::new());
        let eye = Vec3::new(5.0, 1.0, -20.0);
        assert_eq!(
            drawn_indices(portals.visible_groups(Affine3A::IDENTITY, &view(eye, Vec3::Z), eye)),
            [0, 1, 2]
        );
        assert_eq!(
            drawn_indices(portals.visible_groups(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye)),
            [0]
        );
    }

    #[test]
    fn outdoor_window_rejects_a_second_door_outside_the_first_window() {
        let mut portals = courtyard_and_room();
        let bbox_min = portals.groups[1].bbox_min;
        let bbox_max = portals.groups[1].bbox_max;
        portals.groups.push(PortalGroup {
            index: 2,
            bbox_min,
            bbox_max,
            exterior: false,
            root_flags: 0,
            split_parent: None,
            first_child: None,
            next_child: None,
            floor: Vec::new(),
        });
        portals.adjacency = vec![vec![(1, 2, 0)], vec![(0, 2, 0)], vec![(0, 1, 0), (1, 0, 0)]];
        portals.portal_vertices[0] = vec![
            Vec3::new(4.0, 0.0, 4.0),
            Vec3::new(6.0, 0.0, 4.0),
            Vec3::new(6.0, 2.0, 4.0),
            Vec3::new(4.0, 2.0, 4.0),
        ];
        portals.portal_vertices.push(vec![
            Vec3::new(9.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 2.0, 0.0),
            Vec3::new(9.0, 2.0, 0.0),
        ]);
        let eye = Vec3::new(5.0, 1.0, 8.0);
        assert!(
            portals
                .exterior_frusta(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye)
                .unwrap()
                .is_empty()
        );
        for vertex in &mut portals.portal_vertices[1] {
            vertex.x -= 4.0;
        }
        assert_eq!(
            portals
                .exterior_frusta(Affine3A::IDENTITY, &view(eye, -Vec3::Z), eye)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn outside_facing_the_doorway_draws_the_room() {
        assert_eq!(visible(Vec3::new(5.0, 1.0, -20.0), Vec3::Z), [0, 1]);
    }

    #[test]
    fn outside_facing_away_hides_the_room() {
        assert_eq!(visible(Vec3::new(5.0, 1.0, -20.0), -Vec3::Z), [0]);
    }

    #[test]
    fn inside_facing_the_doorway_draws_the_courtyard() {
        assert_eq!(visible(Vec3::new(5.0, 1.0, 8.0), -Vec3::Z), [0, 1]);
    }

    #[test]
    fn inside_facing_the_back_wall_hides_the_courtyard() {
        assert_eq!(visible(Vec3::new(5.0, 1.0, 8.0), Vec3::Z), [1]);
    }

    #[test]
    fn standing_in_the_doorway_sees_both_sides_whatever_the_view() {
        assert_eq!(visible(Vec3::new(5.0, 1.0, 1.0), Vec3::Z), [0, 1]);
    }

    #[test]
    fn root_admission_draws_only_eligible_in_frustum_groups() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let bytes = std::fs::read(data_root.join("terrain/4334889.adt")).unwrap();
        let objects = game_engine_core::adt::parse_obj(&bytes).unwrap();
        let placement = &objects.wmos[0];
        let mut asset =
            crate::wmo::assets::read_placement(&resolver, &data_root, placement).unwrap();
        asset.root.portal_refs.clear();
        asset.root.group_infos[0].flags = 0x8;
        asset.root.group_infos[1].flags = 0x10000;
        asset.root.group_infos[2].flags = 0x20000000;
        asset.root.group_infos[3].flags = 0x04000008;
        asset.root.group_infos[4].flags = 0;
        let portals = WmoPortals::new(&asset);
        let transform = crate::wmo::placement::adt_world_from_local(placement, (33, 32));
        let camera = Vec3::new(-61.1, 75.3, 873.4);

        let visible = portals.visible_groups(transform, &[], camera);
        assert!(visible[0] && visible[1] && visible[2]);
        assert!(!visible[3] && !visible[4]);

        let away = [HalfSpace {
            normal: Vec3::X,
            d: -1_000_000.0,
        }];
        let visible = portals.visible_groups(transform, &away, camera);
        assert!(!visible[0] && !visible[1] && !visible[2]);
    }

    /// Neltharus' central chamber has parent group 12 and split children 15, 14 and 13.
    /// The recorded camera stands in child 13, which has no portals of its own.
    #[test]
    fn split_child_camera_draws_parent_and_siblings() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let bytes = std::fs::read(data_root.join("terrain/4334889.adt")).unwrap();
        let objects = game_engine_core::adt::parse_obj(&bytes).unwrap();
        let placement = &objects.wmos[0];
        let asset = crate::wmo::assets::read_placement(&resolver, &data_root, placement).unwrap();
        assert_eq!(asset.root_fdid, 4_329_056);
        let portals = WmoPortals::new(&asset);
        let transform = crate::wmo::placement::adt_world_from_local(placement, (33, 32));
        let camera = Vec3::new(-61.1, 75.3, 873.4);
        assert_eq!(
            portals.camera_interior_group(transform.inverse().transform_point3(camera)),
            Some(13)
        );
        let visibility = portals.visibility(transform, &[], camera);
        assert!(
            visibility.exterior_windows.is_some(),
            "camera must be inside the WMO"
        );
        for group in [12, 13, 14, 15] {
            assert!(
                visibility.groups[group],
                "central chamber group {group} was hidden"
            );
        }

        let entrance_camera = Vec3::new(70.6, 79.5, 793.2);
        let entrance_drawn = portals.visible_groups(transform, &[], entrance_camera);
        assert!(entrance_drawn[0], "military ward parent was hidden");
        assert!(entrance_drawn[1], "military ward child was hidden");
    }

    /// Neltharus' Magmatusk chamber (groups 2 and 3) is entered from the overlook
    /// corridor (group 11) through an opening without a portal. A camera on the
    /// overlook stands inside the chamber groups' volumes but over the lava, so no floor
    /// triangle puts it in those groups; they must still be drawn or the chamber's walls
    /// and ceiling show sky. Recorded Forever pose: camera (-160.0, 107.0977, 916.9087).
    #[test]
    fn camera_volume_over_lava_draws_the_chamber() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let bytes = std::fs::read(data_root.join("terrain/4334889.adt")).unwrap();
        let objects = game_engine_core::adt::parse_obj(&bytes).unwrap();
        let placement = &objects.wmos[0];
        let asset = crate::wmo::assets::read_placement(&resolver, &data_root, placement).unwrap();
        assert_eq!(asset.root_fdid, 4_329_056);
        let portals = WmoPortals::new(&asset);
        let transform = crate::wmo::placement::adt_world_from_local(placement, (33, 32));
        let camera = Vec3::new(-160.0, 107.0977, 916.9087);
        assert_eq!(
            portals.camera_interior_group(transform.inverse().transform_point3(camera)),
            Some(11)
        );
        let visibility = portals.visibility(transform, &[], camera);
        for group in [2, 3] {
            assert!(
                visibility.groups[group],
                "Magmatusk chamber group {group} was culled"
            );
        }

        let boss_camera = Vec3::new(-211.1, 106.2798, 946.7906);
        let boss = portals.visibility(transform, &[], boss_camera);
        for group in [2, 3] {
            assert!(
                boss.groups[group],
                "Magmatusk chamber group {group} was culled from the boss camera"
            );
        }
    }

    /// Cultists' Quay (WarbandScene 5, tile 2837_27_31): the scene's camera and its
    /// character slot stand in portal-less interior cave groups of WMO 5356285, so its
    /// MFOG record 0 applies at full weight.
    #[test]
    fn cultists_quay_camera_and_character_take_the_cave_fog() {
        use game_engine_core::asset::wmo_format::fog::WmoFogData;

        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let bytes = std::fs::read(data_root.join("terrain/6252664.adt")).unwrap();
        let objects = game_engine_core::adt::parse_obj(&bytes).unwrap();
        let placement = &objects.wmos[0];
        let asset = crate::wmo::assets::read_placement(&resolver, &data_root, placement).unwrap();
        assert_eq!(asset.root_fdid, 5_356_285);
        let portals = WmoPortals::new(&asset);
        let fog = crate::wmo::assets::wmo_fog_volume(&asset);
        let world_from_local = crate::wmo::placement::adt_world_from_local(placement, (27, 31));
        let cave = WmoFogData {
            end: 578.0,
            start_scalar: 0.129,
            color: [21, 80, 99],
        };
        // WarbandScene 5 Position (the authored camera) and WarbandScenePlacement 40.
        for point in [
            Vec3::new(181.911_45, 94.236_43, -2500.392_3),
            Vec3::new(194.243, 91.256_2, -2500.98),
        ] {
            let blend = portals
                .camera_fog(&fog, world_from_local, point)
                .expect("inside an interior cave group");
            assert_eq!(blend.fog, cave);
            assert_eq!(blend.weight(), 1.0);
        }
        // 50 yd above the cave's MOHD box: exterior fog only.
        assert_eq!(
            portals.camera_fog(&fog, world_from_local, Vec3::new(194.0, 460.0, -2500.0)),
            None
        );
    }

    /// Stormwind Stockade (WDT 791060 places WMO 108631 at the origin): its MFOG record 0
    /// is a plain record, record 1 is F_IEBLEND, so the cells take record 0's blue fog.
    #[test]
    fn stockade_cell_block_takes_its_blue_mfog() {
        use game_engine_core::{adt::WmoPlacement, asset::wmo_format::fog::WmoFogData};

        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let placement = WmoPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0; 3],
            rotation: [0.0; 3],
            extents_min: [0.0; 3],
            extents_max: [0.0; 3],
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid: Some(108_631),
            path: None,
        };
        let asset = crate::wmo::assets::read_placement(&resolver, &data_root, &placement).unwrap();
        let fog = crate::wmo::assets::wmo_fog_volume(&asset);
        let portals = WmoPortals::new(&asset);
        let placed = crate::wmo::placement::PlacedWmo::new(placement, asset, None);
        // The owned global-WMO fixture's spawn (native_transfer_fixture STOCKADE), 2 yd up.
        let camera = Vec3::new(103.0, -32.5, -76.0);
        let blend = portals
            .camera_fog(&fog, placed.world_from_local, camera)
            .expect("inside a Stockade interior group");
        // 27.1 yd from the group's nearest portal: past the 25 yd full-weight depth.
        assert!((blend.dist_to_exit - 27.11).abs() < 0.01, "{blend:?}");
        assert_eq!(blend.weight(), 1.0);
        assert_eq!(
            blend.fog,
            WmoFogData {
                end: 133.333_33,
                start_scalar: 0.1,
                color: [49, 91, 143],
            }
        );
    }

    #[test]
    fn placement_transform_moves_the_room() {
        let portals = courtyard_and_room();
        let placed = Affine3A::from_translation(Vec3::new(100.0, 0.0, 0.0));
        let eye = Vec3::new(105.0, 1.0, 8.0);
        let groups = drawn_indices(portals.visible_groups(placed, &view(eye, Vec3::Z), eye));
        assert_eq!(groups, [1]);
    }

    /// Neltharus' WMO: a camera above the first floor triangle of each interior group it
    /// stands in is on that group's MOPR side of the group's portals.
    #[test]
    fn a_camera_in_a_room_is_on_its_side_of_the_room_portals() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let bytes = std::fs::read(data_root.join("terrain/4334889.adt")).unwrap();
        let objects = game_engine_core::adt::parse_obj(&bytes).unwrap();
        let asset =
            crate::wmo::assets::read_placement(&resolver, &data_root, &objects.wmos[0]).unwrap();
        let portals = WmoPortals::new(&asset);
        let (mut on_side, mut off_side) = (0, 0);
        for group in portals.groups.iter().filter(|group| !group.exterior) {
            let Some(floor) = group.floor.first() else {
                continue;
            };
            let camera = (floor[0] + floor[1] + floor[2]) / 3.0 + Vec3::Y;
            if portals.find_camera_interior_group(camera) != Some(group.index) {
                continue;
            }
            for &(portal, _, side) in &portals.adjacency[group.index as usize] {
                if portals.camera_on_side(portal, side, camera) {
                    on_side += 1;
                } else {
                    off_side += 1;
                }
            }
        }
        // With the sides inverted the counts swap: a camera in a room would be behind its
        // own doorways and the traversal would close them. The two remaining edges belong
        // to concave rooms whose floor centre lies behind one of their portal planes.
        assert_eq!((on_side, off_side), (44, 2));
    }
}
