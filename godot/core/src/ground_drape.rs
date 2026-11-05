//! Drapery for flat, ground-aligned M2 batches (ground spell effects).
//!
//! A ground spell's M2 sits at the collision floor, so a carpet drawn above that floor hides
//! it. Shadowlands-clients fix this by conforming the flat plan of the effect to the visible
//! surface. This module subdivides such a batch into a small grid, moves every vertex onto
//! the highest visible render surface under it (see [`crate::render_surface`]) and leaves the
//! authored texture coordinates, colours, blend and vertical parts untouched.
//!
//! Nothing here is used by movement, the server or the navmesh.

use glam::{Affine3A, Vec3};

/// Target world edge length of the subdivided grid, in yards.
pub const DEFAULT_GRID_EDGE: f32 = 0.25;
/// Longest edge a batch may have before it is not treated as a ground batch.
pub const MAX_GROUND_BATCH_EXTENT: f32 = 40.0;
/// A flat batch's world height span; anything taller is a vertical effect, not a plane.
pub const MAX_GROUND_BATCH_THICKNESS: f32 = 0.75;
/// Highest a ground batch may hover above its placement's floor. Kits lift their ground
/// model to avoid z-fighting (Death and Decay reaches 1.72 yd), so this is not the step
/// window the surface search uses.
pub const MAX_GROUND_HOVER: f32 = 2.0;
/// Fraction of the batch's area that must be near-horizontal.
pub const MIN_GROUND_AREA_FRACTION: f32 = 0.9;
/// |normal.y| for a face to count as near-horizontal (up to 25 degrees off level).
pub const MIN_GROUND_NORMAL_Y: f32 = 0.9;
/// Bounded lift of draped vertices, in yards, to keep them off the surface they follow.
pub const DRAPE_BIAS: f32 = 0.02;
/// Hard bound on the generated grid so one effect cannot stall a frame.
pub const MAX_SUBDIVISION: u32 = 32;

/// A batch as the model stores it, in model space.
pub struct BatchMesh<'a> {
    pub positions: &'a [[f32; 3]],
    pub uvs: &'a [[f32; 2]],
    /// Second UV set (masks), interpolated like the first when present.
    pub uvs2: Option<&'a [[f32; 2]]>,
    /// Per-vertex colours (RGBA), interpolated like the UVs when present.
    pub colors: Option<&'a [[f32; 4]]>,
    pub indices: &'a [u32],
}

/// The same batch after conforming; model-space positions, interpolated UVs and triangles.
pub struct DrapedBatch {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub uvs2: Option<Vec<[f32; 2]>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub indices: Vec<u32>,
    /// Vertices that found a visible surface inside the step window.
    pub dropped: usize,
    /// Triangles removed because a draped edge spans more than the step height.
    pub discarded: usize,
}

impl DrapedBatch {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GroundDrape {
    pub grid_edge: f32,
    pub bias: f32,
}

impl Default for GroundDrape {
    fn default() -> Self {
        Self {
            grid_edge: DEFAULT_GRID_EDGE,
            bias: DRAPE_BIAS,
        }
    }
}

impl GroundDrape {
    /// Whether this batch is a flat, ground-aligned plane: most of its area is horizontal,
    /// it has negligible world height, and it lies at or near the placement's ground height.
    pub fn is_ground_batch(
        &self,
        mesh: &BatchMesh<'_>,
        placement: Affine3A,
        ground_y: f32,
    ) -> bool {
        let Some(analysis) = analyze(mesh, placement) else {
            return false;
        };
        // Authored ground effects hover above their placement to avoid z-fighting; only a
        // batch higher than that allowance is a different part of the model.
        analysis.horizontal_area >= MIN_GROUND_AREA_FRACTION * analysis.total_area
            && analysis.min.y >= ground_y - MAX_GROUND_BATCH_THICKNESS
            && analysis.max.y <= ground_y + MAX_GROUND_HOVER
            && analysis.max.y - analysis.min.y <= MAX_GROUND_BATCH_THICKNESS
            && analysis.max.x - analysis.min.x <= MAX_GROUND_BATCH_EXTENT
            && analysis.max.z - analysis.min.z <= MAX_GROUND_BATCH_EXTENT
    }

    /// Subdivide and conform a flat batch.
    ///
    /// `floor_at` gives the collision floor under a world point (the lower bound of the
    /// search) and `surface_at(x, z, ceiling, floor)` the highest visible render surface
    /// inside that interval. Vertices without a surface keep their authored height, so an
    /// effect stays put where the client has no render geometry yet.
    pub fn drape_batch(
        &self,
        mesh: &BatchMesh<'_>,
        placement: Affine3A,
        ground_y: f32,
        window: f32,
        floor_at: impl Fn(f32, f32) -> Option<f32>,
        mut surface_at: impl FnMut(f32, f32, f32, f32) -> Option<f32>,
    ) -> Option<DrapedBatch> {
        if !self.is_ground_batch(mesh, placement, ground_y) {
            return None;
        }
        let model_from_world = placement.inverse();
        let triangles = subdivide(mesh, placement, self.grid_edge)?;
        let mut positions = Vec::with_capacity(triangles.positions.len());
        let mut heights = Vec::with_capacity(triangles.positions.len());
        let mut dropped = 0;
        for world in &triangles.positions {
            let floor = floor_at(world.x, world.z).unwrap_or(ground_y);
            let ceiling = floor + window;
            let draped = surface_at(world.x, world.z, ceiling, floor)
                .map(|surface| surface + self.bias.clamp(0.0, 0.1));
            if draped.is_some() {
                dropped += 1;
            }
            let height = draped.unwrap_or(world.y);
            heights.push(height);
            let local = model_from_world.transform_point3(Vec3::new(world.x, height, world.z));
            positions.push(local.to_array());
        }
        // A triangle whose vertices landed on surfaces more than a step apart (or that
        // bridges a missing hit, which keeps its authored height) would draw as a shard.
        let mut indices = Vec::with_capacity(triangles.indices.len());
        let mut discarded = 0;
        for triangle in triangles.indices.chunks_exact(3) {
            if height_span(triangle, &heights) > window {
                discarded += 1;
                continue;
            }
            indices.extend_from_slice(triangle);
        }
        Some(DrapedBatch {
            positions,
            uvs: triangles.uvs,
            uvs2: triangles.uvs2,
            colors: triangles.colors,
            indices,
            dropped,
            discarded,
        })
    }
}

/// Vertical span of one triangle's draped vertices; infinite when an index is missing.
fn height_span(triangle: &[u32], heights: &[f32]) -> f32 {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for &index in triangle {
        let Some(&height) = heights.get(index as usize) else {
            return f32::INFINITY;
        };
        low = low.min(height);
        high = high.max(height);
    }
    high - low
}

struct Analysis {
    min: Vec3,
    max: Vec3,
    total_area: f32,
    horizontal_area: f32,
}

fn analyze(mesh: &BatchMesh<'_>, placement: Affine3A) -> Option<Analysis> {
    if mesh.indices.len() < 3 || mesh.indices.len() % 3 != 0 {
        return None;
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut total_area = 0.0;
    let mut horizontal_area = 0.0;
    for triangle in mesh.indices.chunks_exact(3) {
        let corners = [
            world_point(mesh, placement, triangle[0])?,
            world_point(mesh, placement, triangle[1])?,
            world_point(mesh, placement, triangle[2])?,
        ];
        for corner in corners {
            min = min.min(corner);
            max = max.max(corner);
        }
        let area = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .length()
            * 0.5;
        if !area.is_finite() {
            continue;
        }
        total_area += area;
        let normal_y = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .normalize_or_zero()
            .y
            .abs();
        if normal_y >= MIN_GROUND_NORMAL_Y {
            horizontal_area += area;
        }
    }
    (total_area > 0.0).then_some(Analysis {
        min,
        max,
        total_area,
        horizontal_area,
    })
}

fn world_point(mesh: &BatchMesh<'_>, placement: Affine3A, index: u32) -> Option<Vec3> {
    let position = mesh.positions.get(index as usize)?;
    Some(placement.transform_point3(Vec3::from_array(*position)))
}

/// The tessellated batch, still in world space.
struct Tessellated {
    positions: Vec<Vec3>,
    uvs: Vec<[f32; 2]>,
    uvs2: Option<Vec<[f32; 2]>>,
    colors: Option<Vec<[f32; 4]>>,
    indices: Vec<u32>,
}

/// Splits every triangle into a barycentric grid whose world edges are at most `edge`.
///
/// A triangle is split into `n * n` sub-triangles per edge, so a ground plane's two large
/// triangles become a regular grid. UVs interpolate linearly, keeping the authored mapping.
fn subdivide(mesh: &BatchMesh<'_>, placement: Affine3A, edge: f32) -> Option<Tessellated> {
    let edge = edge.max(0.02);
    // A stream that does not cover every position cannot interpolate every triangle.
    let vertices = mesh.positions.len();
    let uvs2 = mesh.uvs2.filter(|uvs| uvs.len() == vertices);
    let colors = mesh.colors.filter(|colors| colors.len() == vertices);
    let mut tessellated = Tessellated {
        positions: Vec::new(),
        uvs: Vec::new(),
        uvs2: uvs2.map(|_| Vec::new()),
        colors: colors.map(|_| Vec::new()),
        indices: Vec::new(),
    };
    for triangle in mesh.indices.chunks_exact(3) {
        let corners = [
            world_point(mesh, placement, triangle[0])?,
            world_point(mesh, placement, triangle[1])?,
            world_point(mesh, placement, triangle[2])?,
        ];
        let uv = [
            *mesh.uvs.get(triangle[0] as usize)?,
            *mesh.uvs.get(triangle[1] as usize)?,
            *mesh.uvs.get(triangle[2] as usize)?,
        ];
        let uv2 = match uvs2 {
            Some(uvs) => Some([
                *uvs.get(triangle[0] as usize)?,
                *uvs.get(triangle[1] as usize)?,
                *uvs.get(triangle[2] as usize)?,
            ]),
            None => None,
        };
        let color = match colors {
            Some(colors) => Some([
                *colors.get(triangle[0] as usize)?,
                *colors.get(triangle[1] as usize)?,
                *colors.get(triangle[2] as usize)?,
            ]),
            None => None,
        };
        let longest = (corners[1] - corners[0])
            .length()
            .max((corners[2] - corners[1]).length())
            .max((corners[0] - corners[2]).length());
        let steps = (longest / edge).ceil().clamp(1.0, MAX_SUBDIVISION as f32) as u32;
        emit_grid(&mut tessellated, corners, uv, uv2, color, steps);
        if tessellated.positions.len() > 64 * 1024 {
            return None;
        }
    }
    Some(tessellated)
}

/// Appends one triangle's barycentric grid, deduplicating with a local index grid.
fn emit_grid(
    out: &mut Tessellated,
    corners: [Vec3; 3],
    uv: [[f32; 2]; 3],
    uv2: Option<[[f32; 2]; 3]>,
    color: Option<[[f32; 4]; 3]>,
    steps: u32,
) {
    let base = out.positions.len() as u32;
    let factor = 1.0 / steps as f32;
    for row in 0..=steps {
        for column in 0..=(steps - row) {
            let b = row as f32 * factor;
            let c = column as f32 * factor;
            let a = 1.0 - b - c;
            out.positions
                .push(corners[0] * a + corners[1] * b + corners[2] * c);
            out.uvs.push([
                uv[0][0] * a + uv[1][0] * b + uv[2][0] * c,
                uv[0][1] * a + uv[1][1] * b + uv[2][1] * c,
            ]);
            if let (Some(uv2), Some(target)) = (uv2, out.uvs2.as_mut()) {
                target.push([
                    uv2[0][0] * a + uv2[1][0] * b + uv2[2][0] * c,
                    uv2[0][1] * a + uv2[1][1] * b + uv2[2][1] * c,
                ]);
            }
            if let (Some(color), Some(target)) = (color, out.colors.as_mut()) {
                let mut value = [0.0; 4];
                for channel in 0..4 {
                    value[channel] =
                        color[0][channel] * a + color[1][channel] * b + color[2][channel] * c;
                }
                target.push(value);
            }
        }
    }
    // Vertex at (row, column) lives at the start of its row.
    let row_start = |row: u32| {
        let mut start = 0;
        for previous in 0..row {
            start += steps - previous + 1;
        }
        start
    };
    for row in 0..steps {
        for column in 0..(steps - row) {
            let top = row_start(row) + column;
            let below = row_start(row + 1) + column;
            let is_last = column + 1 == steps - row;
            out.indices
                .extend_from_slice(&[base + top, base + top + 1, base + below]);
            if !is_last {
                out.indices
                    .extend_from_slice(&[base + top + 1, base + below + 1, base + below]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch<'a>(
        positions: &'a [[f32; 3]],
        uvs: &'a [[f32; 2]],
        indices: &'a [u32],
    ) -> BatchMesh<'a> {
        BatchMesh {
            positions,
            uvs,
            uvs2: None,
            colors: None,
            indices,
        }
    }

    /// A 6x6 yd horizontal quad centered at the origin.
    fn ground_quad() -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
        (
            vec![
                [-3.0, 0.0, -3.0],
                [3.0, 0.0, -3.0],
                [3.0, 0.0, 3.0],
                [-3.0, 0.0, 3.0],
            ],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![0, 1, 2, 0, 2, 3],
        )
    }

    /// Render steps rising with x, 0.25 yd per 1 yd, starting at x = -3.
    fn stair_surface(x: f32, z: f32, ceiling: f32, floor: f32) -> Option<f32> {
        if !(-3.0..=3.0).contains(&x) || !(-3.0..=3.0).contains(&z) {
            return None;
        }
        let tread = 0.25
            * (0..8)
                .map(|step| (-3.0 + step as f32, 0.25 * (step + 1) as f32))
                .find(|&(start, _)| x >= start && x < start + 1.0)
                .map(|(_, height)| height)?;
        (floor..=ceiling).contains(&tread).then_some(tread)
    }

    #[test]
    fn a_flat_ground_quad_follows_the_render_steps_and_keeps_its_uvs() {
        let (positions, uvs, indices) = ground_quad();
        let mesh = batch(&positions, &uvs, &indices);
        let drape = GroundDrape::default();
        assert!(drape.is_ground_batch(&mesh, Affine3A::IDENTITY, 0.0));
        let draped = drape
            .drape_batch(
                &mesh,
                Affine3A::IDENTITY,
                0.0,
                1.6,
                |_, _| Some(0.0),
                stair_surface,
            )
            .expect("draped");
        assert_eq!(draped.dropped, draped.positions.len());
        for (vertex, uv) in draped.positions.iter().zip(&draped.uvs) {
            let expected = stair_surface(vertex[0], vertex[2], 1.6, 0.0).expect("step");
            assert!(
                (vertex[1] - (expected + DRAPE_BIAS)).abs() < 1e-3,
                "vertex {vertex:?} should sit on tread {expected}"
            );
            assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
        }
        // A 6 yd edge at 0.25 yd spacing keeps the grid close to the requirement.
        for triangle in draped.indices.chunks_exact(3) {
            let corners = [
                Vec3::from_array(draped.positions[triangle[0] as usize]),
                Vec3::from_array(draped.positions[triangle[1] as usize]),
                Vec3::from_array(draped.positions[triangle[2] as usize]),
            ];
            for (a, b) in [
                (corners[0], corners[1]),
                (corners[1], corners[2]),
                (corners[2], corners[0]),
            ] {
                assert!(
                    (a.x - b.x).abs() <= 0.26 && (a.z - b.z).abs() <= 0.26,
                    "edge {a:?} -> {b:?} is longer than the grid"
                );
            }
        }
    }

    #[test]
    fn a_carpet_edge_adheres_on_both_sides_and_nothing_floats() {
        let (positions, uvs, indices) = ground_quad();
        let mesh = batch(&positions, &uvs, &indices);
        let drape = GroundDrape::default();
        // The carpet is the half x > 0, 0.3 yd above the collision floor.
        let surface = |x: f32, _z: f32, ceiling: f32, floor: f32| {
            let height = if x >= 0.0 { 0.3 } else { 0.0 };
            (floor..=ceiling).contains(&height).then_some(height)
        };
        let draped = drape
            .drape_batch(
                &mesh,
                Affine3A::IDENTITY,
                0.0,
                1.6,
                |_, _| Some(0.0),
                surface,
            )
            .expect("draped");
        let mut below = 0;
        let mut above = 0;
        for vertex in &draped.positions {
            let expected = if vertex[0] >= 0.0 { 0.3 } else { 0.0 } + DRAPE_BIAS;
            assert!((vertex[1] - expected).abs() < 1e-3, "vertex {vertex:?}");
            if vertex[0] > 0.0 {
                above += 1;
            } else {
                below += 1;
            }
        }
        assert!(below > 0 && above > 0);
    }

    #[test]
    fn vertical_and_thick_batches_are_not_ground_batches() {
        let drape = GroundDrape::default();
        let uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        // A vertical quad: a wall effect, not a floor.
        let wall = [
            [0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [3.0, 3.0, 0.0],
            [3.0, 0.0, 0.0],
        ];
        let indices = [0, 1, 2, 0, 2, 3];
        let mesh = batch(&wall, &uvs, &indices);
        assert!(!drape.is_ground_batch(&mesh, Affine3A::IDENTITY, 0.0));
        // A ground quad lifted far above its placement is not aligned to the ground.
        let (positions, uvs, indices) = ground_quad();
        let lifted: Vec<_> = positions
            .iter()
            .map(|[x, y, z]| [*x, y + 4.0, *z])
            .collect();
        let lifted_mesh = batch(&lifted, &uvs, &indices);
        assert!(!drape.is_ground_batch(&lifted_mesh, Affine3A::IDENTITY, 0.0));
    }

    #[test]
    fn a_hovering_ground_quad_drapes_onto_the_surface() {
        let (positions, uvs, indices) = ground_quad();
        let hovering: Vec<_> = positions
            .iter()
            .map(|[x, y, z]| [*x, y + 1.0, *z])
            .collect();
        let mesh = batch(&hovering, &uvs, &indices);
        let drape = GroundDrape::default();
        let draped = drape
            .drape_batch(
                &mesh,
                Affine3A::IDENTITY,
                0.0,
                1.6,
                |_, _| Some(0.0),
                |_, _, _, _| Some(0.0),
            )
            .expect("hovering ground quad drapes");
        assert!(draped.dropped > 100, "dropped {}", draped.dropped);
        let lowest = draped
            .positions
            .iter()
            .fold(f32::INFINITY, |low, point| low.min(point[1]));
        assert!(lowest >= 0.0, "lowest {lowest}");
    }

    #[test]
    fn a_ground_quad_hovering_within_a_step_is_a_ground_batch() {
        let (positions, uvs, indices) = ground_quad();
        let hovering: Vec<_> = positions
            .iter()
            .map(|[x, y, z]| [*x, y + 1.0, *z])
            .collect();
        let mesh = batch(&hovering, &uvs, &indices);
        let drape = GroundDrape::default();
        assert!(drape.is_ground_batch(&mesh, Affine3A::IDENTITY, 0.0));
    }

    #[test]
    fn a_tilted_ground_plane_is_still_draped() {
        let (positions, uvs, indices) = ground_quad();
        let tilted: Vec<_> = positions
            .iter()
            .map(|[x, y, z]| [*x, y + 0.1 * x, *z])
            .collect();
        let mesh = batch(&tilted, &uvs, &indices);
        let drape = GroundDrape::default();
        assert!(drape.is_ground_batch(&mesh, Affine3A::IDENTITY, 0.0));
    }

    #[test]
    fn vertices_without_a_surface_keep_their_authored_height() {
        let (positions, uvs, indices) = ground_quad();
        let mesh = batch(&positions, &uvs, &indices);
        let drape = GroundDrape::default();
        let draped = drape
            .drape_batch(
                &mesh,
                Affine3A::IDENTITY,
                0.0,
                1.6,
                |x, _| (x < 0.0).then_some(0.0),
                |x, _z, _ceiling, _floor| (x < 0.0).then_some(0.0),
            )
            .expect("draped");
        assert!(draped.dropped > 0);
        assert!(draped.dropped < draped.positions.len());
        for vertex in &draped.positions {
            if vertex[0] >= 0.0 {
                assert!((vertex[1]).abs() < 1e-4, "authored height kept: {vertex:?}");
            }
        }
    }

    /// A vertex without a hit keeps its authored height, so a triangle that bridges the
    /// two heights would stretch. Its edge span is more than a step, and it is dropped.
    #[test]
    fn triangles_whose_draped_edge_spans_more_than_a_step_are_dropped() {
        let (positions, uvs, indices) = ground_quad();
        let lifted: Vec<_> = positions
            .iter()
            .map(|[x, y, z]| [*x, y + 1.9, *z])
            .collect();
        let mesh = batch(&lifted, &uvs, &indices);
        let drape = GroundDrape::default();
        let draped = drape
            .drape_batch(
                &mesh,
                Affine3A::IDENTITY,
                0.0,
                1.6,
                |x, _| (x < 0.0).then_some(0.0),
                |x, _z, _ceiling, _floor| (x < 0.0).then_some(0.0),
            )
            .expect("draped");
        assert!(draped.discarded > 0, "no bridging triangle was dropped");
        assert!(
            draped.triangle_count() > 0,
            "a flat patch keeps its triangles"
        );
        for triangle in draped.indices.chunks_exact(3) {
            let mut heights = [0.0f32; 3];
            for (height, index) in heights.iter_mut().zip(triangle) {
                *height = draped.positions[*index as usize][1];
            }
            let span = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - heights.iter().copied().fold(f32::INFINITY, f32::min);
            assert!(span <= 1.6, "kept triangle spans {span}: {heights:?}");
        }
    }

    #[test]
    fn placement_rotation_is_undone_so_vertices_stay_in_model_space() {
        let (positions, uvs, indices) = ground_quad();
        let mesh = batch(&positions, &uvs, &indices);
        let placement = Affine3A::from_translation(Vec3::new(10.0, 2.0, 0.0))
            * Affine3A::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let drape = GroundDrape::default();
        let draped = drape
            .drape_batch(
                &mesh,
                placement,
                2.0,
                1.6,
                |_, _| Some(2.0),
                |_x, _z, _ceiling, _floor| Some(2.5),
            )
            .expect("draped");
        // The tessellated vertices are new, so the round trip is checked against the
        // authored footprint: the placement puts the 6x6 quad at x 7..13, z -3..3.
        for local in &draped.positions {
            let world = placement.transform_point3(Vec3::from_array(*local));
            assert!((world.y - (2.5 + DRAPE_BIAS)).abs() < 1e-3);
            assert!((6.999..=13.001).contains(&world.x), "world {world:?}");
            assert!((-3.001..=3.001).contains(&world.z), "world {world:?}");
        }
    }
}
