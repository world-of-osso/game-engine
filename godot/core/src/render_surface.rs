//! Highest visible render surface under a point.
//!
//! Collision deliberately matches Retail: `shared-protocol/src/ground/wmo.rs` honours the
//! MOPY collision flag, so stairs commonly carry an invisible collision-only ramp over
//! render-only treads, and carpets are render-only geometry above the collision floor.
//! The client keeps that collision untouched and asks this module for the visible surface
//! instead, using the geometry it actually draws.
//!
//! Movement, the server and the navmesh never read this: it is a client-side visual query.

use glam::Vec3;

/// How far under its collision floor an authored floor may sit and still count as the
/// visible floor: collision faces are often a few centimetres above the drawn surface.
pub const FLOOR_TOLERANCE: f32 = 0.05;

/// Only faces a character could visually stand on: upward faces no more than about 45
/// degrees from horizontal. Render treads, carpets and ramps pass; risers, ceilings and
/// walls do not.
const MIN_UP_NORMAL_Y: f32 = 0.7;

/// Triangles per BVH leaf.
const LEAF_SIZE: usize = 4;
/// Split bound for the traversal stack.
const MAX_DEPTH: usize = 64;
/// Segment ends exclude a triangle that touches only the segment's start (a point resting
/// on a surface is not a hit above itself).
const END_EPSILON: f32 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// Leaf: first triangle. Interior: left child index (`start + 1` is the right child).
    start: u32,
    count: u32,
}

/// A triangle mesh with a BVH over it, for vertical render-surface queries.
#[derive(Debug, PartialEq)]
pub struct RenderSurface {
    vertices: Vec<Vec3>,
    /// In BVH leaf order.
    triangles: Vec<[u32; 3]>,
    nodes: Vec<Node>,
}

impl RenderSurface {
    /// Builds the BVH over `triangles` (indices into `vertices`). Triangles with an
    /// out-of-range index or no area are dropped; an empty result has no surface.
    pub fn build(vertices: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Option<Self> {
        let mut kept = Vec::with_capacity(triangles.len());
        for triangle in triangles {
            if triangle
                .iter()
                .any(|&index| index as usize >= vertices.len())
            {
                continue;
            }
            let [a, b, c] = triangle.map(|index| vertices[index as usize]);
            if (b - a).cross(c - a).length_squared() <= f32::EPSILON {
                continue;
            }
            kept.push(triangle);
        }
        if kept.is_empty() {
            return None;
        }
        let (nodes, order) = build_bvh(&vertices, &kept);
        let triangles = order
            .iter()
            .map(|&index| kept[index as usize])
            .collect::<Vec<_>>();
        Some(Self {
            vertices,
            triangles,
            nodes,
        })
    }

    /// Number of triangles the surface holds.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Area of triangle `index`, in the mesh's own space.
    pub fn triangle_area(&self, index: usize) -> Option<f32> {
        let triangle = self.triangles.get(index)?;
        let corners = triangle.map(|index| self.vertices[index as usize]);
        Some(
            (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .length()
                * 0.5,
        )
    }

    /// Up component of triangle `index`'s normal (negative for a downward face).
    pub fn triangle_normal_y(&self, index: usize) -> Option<f32> {
        let triangle = self.triangles.get(index)?;
        let corners = triangle.map(|index| self.vertices[index as usize]);
        Some(
            (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .normalize_or_zero()
                .y,
        )
    }

    /// Centroid of triangle `index`, in the mesh's own space.
    pub fn triangle_centroid(&self, index: usize) -> Option<Vec3> {
        let triangle = self.triangles.get(index)?;
        let corners = triangle.map(|index| self.vertices[index as usize]);
        Some((corners[0] + corners[1] + corners[2]) / 3.0)
    }

    /// World bounds of the mesh; `None` at no triangles (never built).
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.nodes.first().map(|root| (root.min, root.max))
    }

    /// Height of the highest upward face on the vertical line through `(x, z)` inside
    /// `[floor, ceiling]`. `None` when no upward face crosses that column.
    pub fn highest_upward_below(&self, x: f32, z: f32, ceiling: f32, floor: f32) -> Option<f32> {
        if self.nodes.is_empty() || ceiling < floor {
            return None;
        }
        // Cast a hair past both ends so a face exactly at the floor or the ceiling is a
        // hit; the height filter below keeps the requested interval, widened by the floor
        // tolerance: an authored floor often sits a few centimetres under its collision face.
        const MARGIN: f32 = 1e-3;
        let bottom = floor - FLOOR_TOLERANCE;
        let top = Vec3::new(x, ceiling + MARGIN, z);
        let dir = Vec3::new(0.0, bottom - ceiling - 2.0 * MARGIN, 0.0);
        let mut found: Option<f32> = None;
        let mut stack = Vec::with_capacity(MAX_DEPTH);
        stack.push(0usize);
        while let Some(index) = stack.pop() {
            let node = self.nodes[index];
            if !column_hits(node.min, node.max, top, dir) {
                continue;
            }
            if node.count == 0 {
                stack.push(node.start as usize);
                stack.push(node.start as usize + 1);
                continue;
            }
            for triangle in &self.triangles[node.start as usize..(node.start + node.count) as usize]
            {
                let corners = triangle.map(|index| self.vertices[index as usize]);
                if !faces_up(corners) {
                    continue;
                }
                if let Some(t) = segment_triangle(top, dir, corners) {
                    let height = top.y + dir.y * t;
                    if (bottom..=ceiling).contains(&height) {
                        found = Some(found.map_or(height, |high| high.max(height)));
                    }
                }
            }
        }
        found
    }
}

/// Several render meshes queried together, each rejected by its bounds first. Parts are
/// the already-parsed group or model meshes a scene holds; the bounds are the spatial
/// acceleration between parts and the BVH inside each part.
#[derive(Default)]
pub struct SurfaceSet {
    parts: Vec<SurfacePart>,
}

pub struct SurfacePart {
    pub min: Vec3,
    pub max: Vec3,
    pub surface: RenderSurface,
}

impl SurfaceSet {
    pub fn push(&mut self, surface: RenderSurface) {
        let Some((min, max)) = surface.bounds() else {
            return;
        };
        self.parts.push(SurfacePart { min, max, surface });
    }

    pub fn clear(&mut self) {
        self.parts.clear();
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn part(&self, index: usize) -> Option<&RenderSurface> {
        self.parts.get(index).map(|part| &part.surface)
    }

    /// Highest upward face on the column through `(x, z)` inside `[floor, ceiling]`, with
    /// the part it belongs to. Parts whose bounds miss the column are skipped.
    pub fn highest_upward_below(
        &self,
        x: f32,
        z: f32,
        ceiling: f32,
        floor: f32,
    ) -> Option<(f32, usize)> {
        let mut found: Option<(f32, usize)> = None;
        for (index, part) in self.parts.iter().enumerate() {
            if x < part.min.x || x > part.max.x || z < part.min.z || z > part.max.z {
                continue;
            }
            if part.min.y > ceiling || part.max.y < floor {
                continue;
            }
            if let Some(height) = part.surface.highest_upward_below(x, z, ceiling, floor) {
                found = Some(found.map_or((height, index), |(high, part)| {
                    (high.max(height), if height > high { index } else { part })
                }));
            }
        }
        found
    }
}

fn faces_up(corners: [Vec3; 3]) -> bool {
    let [a, b, c] = corners;
    let normal = (b - a).cross(c - a).normalize_or_zero();
    // Upward only: a downward face (a ceiling or an overhang's underside) is never a
    // surface to stand on, however horizontal it is.
    normal.y >= MIN_UP_NORMAL_Y
}

/// Slab test for the vertical segment `origin`..`origin + dir`.
fn column_hits(min: Vec3, max: Vec3, origin: Vec3, dir: Vec3) -> bool {
    let safe = Vec3::select(dir.abs().cmplt(Vec3::splat(1e-30)), Vec3::splat(1e-30), dir);
    let inv = safe.recip();
    let mut low = (min.x - origin.x) * inv.x;
    let mut high = (max.x - origin.x) * inv.x;
    if low > high {
        std::mem::swap(&mut low, &mut high);
    }
    let z_low = (min.z - origin.z) * inv.z;
    let z_high = (max.z - origin.z) * inv.z;
    low = low.max(z_low.min(z_high));
    high = high.min(z_low.max(z_high));
    let y_low = (min.y - origin.y) * inv.y;
    let y_high = (max.y - origin.y) * inv.y;
    low = low.max(y_low.min(y_high));
    high = high.min(y_low.max(y_high));
    low <= high && high >= 0.0 && low <= 1.0
}

/// Two-sided Moeller-Trumbore against the full segment `origin`..`origin + dir`.
fn segment_triangle(origin: Vec3, dir: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let edge1 = b - a;
    let edge2 = c - a;
    let p = dir.cross(edge2);
    let det = edge1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = dir.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inv_det;
    (END_EPSILON..=1.0 - END_EPSILON).contains(&t).then_some(t)
}

/// Builds the BVH and the leaf triangle order.
///
/// Node layout follows the traversal contract: node 0 is the root; an interior node's
/// children are `start` and `start + 1`; a leaf covers `order[start..start + count]`.
fn build_bvh(vertices: &[Vec3], triangles: &[[u32; 3]]) -> (Vec<Node>, Vec<usize>) {
    let mut order: Vec<usize> = (0..triangles.len()).collect();
    let boxes: Vec<(Vec3, Vec3)> = triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = triangle.map(|index| vertices[index as usize]);
            (a.min(b).min(c), a.max(b).max(c))
        })
        .collect();
    let mut nodes = vec![Node {
        min: Vec3::ZERO,
        max: Vec3::ZERO,
        start: 0,
        count: 0,
    }];
    let mut builder = Builder {
        boxes: &boxes,
        order: &mut order,
        nodes: &mut nodes,
    };
    let end = triangles.len();
    builder.fill(0, 0, end, 0);
    (nodes, order)
}

struct Builder<'a> {
    boxes: &'a [(Vec3, Vec3)],
    order: &'a mut Vec<usize>,
    nodes: &'a mut Vec<Node>,
}

impl Builder<'_> {
    /// Fills a node slot already allocated for `order[start..end]`; interior nodes append
    /// their two adjacent child slots at the end, the layout traversal expects.
    fn fill(&mut self, index: usize, start: usize, end: usize, depth: usize) {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        let mut centroid_min = Vec3::splat(f32::INFINITY);
        let mut centroid_max = Vec3::splat(f32::NEG_INFINITY);
        for &triangle in &self.order[start..end] {
            let (low, high) = self.boxes[triangle];
            min = min.min(low);
            max = max.max(high);
            let centroid = (low + high) * 0.5;
            centroid_min = centroid_min.min(centroid);
            centroid_max = centroid_max.max(centroid);
        }
        let count = end - start;
        let extent = centroid_max - centroid_min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        if count <= LEAF_SIZE || depth >= MAX_DEPTH || extent[axis] <= f32::EPSILON {
            self.nodes[index] = Node {
                min,
                max,
                start: start as u32,
                count: count as u32,
            };
            return;
        }
        // A median split keeps both halves equal, so coincident triangles cannot nest
        // deeper than the depth bound.
        self.order[start..end].sort_by(|&a, &b| {
            let left = (self.boxes[a].0 + self.boxes[a].1)[axis];
            let right = (self.boxes[b].0 + self.boxes[b].1)[axis];
            left.total_cmp(&right)
        });
        let middle = start + count / 2;
        // Both children are adjacent, so traversal reaches them at `start` and `start + 1`.
        let left = self.nodes.len();
        let right = left + 1;
        self.nodes.push(Node {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            start: 0,
            count: 0,
        });
        self.nodes.push(Node {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            start: 0,
            count: 0,
        });
        self.nodes[index] = Node {
            min,
            max,
            start: left as u32,
            count: 0,
        };
        self.fill(left, start, middle, depth + 1);
        self.fill(right, middle, end, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A horizontal quad at `y` spanning `[x0, x1] x [z0, z1]`, wound upward (the engine's
    /// counter-clockwise front faces, as every render mesh is drawn).
    fn quad(y: f32, x0: f32, x1: f32, z0: f32, z1: f32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let vertices = vec![
            Vec3::new(x0, y, z0),
            Vec3::new(x1, y, z0),
            Vec3::new(x1, y, z1),
            Vec3::new(x0, y, z1),
        ];
        (vertices, vec![[0, 2, 1], [0, 3, 2]])
    }

    fn plane(y: f32) -> RenderSurface {
        let (vertices, triangles) = quad(y, -10.0, 10.0, -10.0, 10.0);
        RenderSurface::build(vertices, triangles).expect("plane")
    }

    #[test]
    fn an_authored_floor_just_under_its_collision_face_is_found() {
        // Collision faces sit a couple of centimetres above the drawn floor in authored data.
        let surface = plane(-0.02);
        let height = surface
            .highest_upward_below(0.0, 0.0, 1.6, 0.0)
            .expect("floor just under collision");
        assert!((height + 0.02).abs() < 1e-3, "height {height}");
    }

    #[test]
    fn a_floor_well_under_its_collision_face_is_not_the_visible_floor() {
        let surface = plane(-0.5);
        assert_eq!(surface.highest_upward_below(0.0, 0.0, 1.6, 0.0), None);
    }

    #[test]
    fn returns_the_highest_upward_face_in_the_window() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for y in [0.0, 0.4, 0.9] {
            let (mut points, mut faces) = quad(y, -2.0, 2.0, -2.0, 2.0);
            let base = vertices.len() as u32;
            vertices.append(&mut points);
            triangles.extend(faces.drain(..).map(|face| face.map(|index| index + base)));
        }
        let surface = RenderSurface::build(vertices, triangles).expect("stack");
        assert_eq!(surface.highest_upward_below(0.5, -0.5, 1.6, 0.0), Some(0.9));
        assert_eq!(surface.highest_upward_below(0.5, -0.5, 0.7, 0.0), Some(0.4));
        assert_eq!(surface.highest_upward_below(5.0, 0.0, 1.6, 0.0), None);
    }

    #[test]
    fn ignores_faces_above_the_window_and_vertical_faces() {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let (mut points, mut faces) = quad(0.0, -2.0, 2.0, -2.0, 2.0);
        vertices.append(&mut points);
        triangles.append(&mut faces);
        // A face above the window is a ceiling or a higher floor, not this surface.
        let base = vertices.len() as u32;
        let (mut points, mut faces) = quad(5.0, -2.0, 2.0, -2.0, 2.0);
        vertices.append(&mut points);
        triangles.extend(faces.drain(..).map(|face| face.map(|index| index + base)));
        // A vertical wall crossing the column is not ground.
        let base = vertices.len() as u32;
        vertices.extend([
            Vec3::new(0.0, 0.0, -2.0),
            Vec3::new(0.0, 3.0, -2.0),
            Vec3::new(0.0, 3.0, 2.0),
            Vec3::new(0.0, 0.0, 2.0),
        ]);
        triangles.push([base, base + 1, base + 2]);
        triangles.push([base, base + 2, base + 3]);
        let surface = RenderSurface::build(vertices, triangles).expect("surface");
        assert_eq!(surface.highest_upward_below(0.0, 0.0, 1.6, 0.0), Some(0.0));
    }

    #[test]
    fn collision_ramp_and_render_steps_pick_the_step_tread() {
        // The collision ramp rises 1.0 over 4.0; the render stairs are four 1.0-wide
        // treads at 0.25, 0.5, 0.75 and 1.0 with vertical risers between them.
        let ramp_floor = |x: f32| 0.25 * (x + 2.0);
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        // Steps.
        for step in 0..4 {
            let x0 = -2.0 + step as f32;
            let y = 0.25 * (step as f32 + 1.0);
            let (mut points, mut faces) = quad(y, x0, x0 + 1.0, -2.0, 2.0);
            let base = vertices.len() as u32;
            vertices.append(&mut points);
            triangles.extend(faces.drain(..).map(|face| face.map(|index| index + base)));
        }
        let surface = RenderSurface::build(vertices, triangles).expect("steps");
        for step in 0..4 {
            let x = -2.0 + step as f32 + 0.5;
            let tread = 0.25 * (step as f32 + 1.0);
            let collision = ramp_floor(x);
            let height = surface
                .highest_upward_below(x, 0.0, collision + 1.6, collision)
                .expect("visible tread");
            assert!(
                (height - tread).abs() < 1e-4,
                "step {step}: tread {tread} != {height}"
            );
            assert!(height >= collision);
        }
    }

    #[test]
    fn bvh_query_matches_brute_force_over_scattered_triangles() {
        let mut state = 0x1234_5678u64;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as f32 / (u32::MAX >> 1) as f32) * 20.0 - 10.0
        };
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for _ in 0..400 {
            let base = vertices.len() as u32;
            let center = Vec3::new(next(), next().abs(), next());
            vertices.push(center);
            vertices.push(center + Vec3::new(0.8, 0.1, 0.0));
            vertices.push(center + Vec3::new(0.0, 0.1, 0.9));
            triangles.push([base, base + 2, base + 1]);
        }
        let surface = RenderSurface::build(vertices.clone(), triangles.clone()).expect("mesh");
        for _ in 0..200 {
            let x = next();
            let z = next();
            let ceiling = 12.0;
            let floor = 0.0;
            let mut expected: Option<f32> = None;
            for triangle in &triangles {
                let corners = triangle.map(|index| vertices[index as usize]);
                if !faces_up(corners) {
                    continue;
                }
                let top = Vec3::new(x, ceiling, z);
                let dir = Vec3::new(0.0, floor - ceiling, 0.0);
                if let Some(t) = segment_triangle(top, dir, corners) {
                    let height = top.y + dir.y * t;
                    expected = Some(expected.map_or(height, |high| high.max(height)));
                }
            }
            match (surface.highest_upward_below(x, z, ceiling, floor), expected) {
                (Some(found), Some(height)) => assert!(
                    (found - height).abs() < 1e-3,
                    "query at ({x}, {z}): {found} != {height}"
                ),
                (None, None) => {}
                (found, expected) => panic!("query at ({x}, {z}): {found:?} != {expected:?}"),
            }
        }
    }

    #[test]
    fn a_surface_set_skips_parts_whose_bounds_miss_the_column() {
        let (vertices, triangles) = quad(0.5, -1.0, 1.0, -1.0, 1.0);
        let (other_vertices, other_triangles) = quad(0.8, 4.0, 6.0, 4.0, 6.0);
        let mut set = SurfaceSet::default();
        set.push(RenderSurface::build(vertices, triangles).expect("first"));
        set.push(RenderSurface::build(other_vertices, other_triangles).expect("second"));
        assert_eq!(set.len(), 2);
        let near = |value: Option<(f32, usize)>, height: f32, part: usize| {
            value.is_some_and(|(found, index)| (found - height).abs() < 1e-3 && index == part)
        };
        assert!(near(set.highest_upward_below(0.0, 0.0, 1.6, 0.0), 0.5, 0));
        assert!(near(set.highest_upward_below(5.0, 5.0, 1.6, 0.0), 0.8, 1));
        assert_eq!(set.highest_upward_below(3.0, 3.0, 1.6, 0.0), None);
    }

    /// A horizontal face wound downward is a ceiling or an overhang's underside, never a
    /// surface a character stands on, even when it is inside the step window, and neither is
    /// a face steeper than about 45 degrees.
    #[test]
    fn a_downward_or_steep_face_is_not_a_visible_floor() {
        let (vertices, _) = quad(0.0, -2.0, 2.0, -2.0, 2.0);
        // Reverse the upward winding of both triangles.
        let down = RenderSurface::build(vertices, vec![[0, 1, 2], [0, 2, 3]]).expect("ceiling");
        assert_eq!(down.triangle_normal_y(0), Some(-1.0));
        assert_eq!(down.highest_upward_below(0.0, 0.0, 1.6, 0.0), None);
        // 60 degrees off horizontal: a roof face, not a floor.
        let steep = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.866, 0.5),
            Vec3::new(1.0, 0.0, 0.0),
        ];
        assert!(!faces_up(steep));
    }

    #[test]
    fn an_empty_or_degenerate_mesh_has_no_surface() {
        assert!(RenderSurface::build(Vec::new(), Vec::new()).is_none());
        assert!(
            RenderSurface::build(vec![Vec3::ZERO, Vec3::X, Vec3::Z], vec![[0, 1, 9]]).is_none()
        );
    }
}
