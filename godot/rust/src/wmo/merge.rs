//! Exterior WMO batches with equal material inputs share one mesh per cell. Portal
//! culling draws the exterior groups together (its traversal starts from all of them), so
//! a merged draw keeps their visibility and gives up only frustum culling inside a cell;
//! Stormwind's district WMOs drop from about 540 to under 200 exterior draws each.
//! Interior groups keep one node per batch for exact portal culling.

use game_engine_core::wmo;

use super::scene::RetailWmoStreams;

/// Horizontal cell edge in WMO units (yards).
const MERGE_CELL_YARDS: f32 = 256.0;

/// Every input of a batch's material and vertex layout that differs between batches
/// of one WMO, plus its cell: batches with equal keys build equal materials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct MergeKey {
    material: u16,
    exterior_lit: bool,
    casts_shadow: bool,
    colors: bool,
    uv2: bool,
    uv3: bool,
    uv4: bool,
    second_mocv_alpha: bool,
    moc2: bool,
    cell: (i32, i32),
}

impl MergeKey {
    /// The key of an exterior group's opaque batch; `None` for a batch that keeps its
    /// own node: blended (its draw order matters), interior or outside the portal graph,
    /// or with a stream whose length differs from its vertices.
    pub(super) fn of(
        header: &wmo::WmoGroupHeader,
        portal_culled: bool,
        blend_mode: u32,
        mesh: &wmo::WmoMeshBatch,
        streams: &RetailWmoStreams,
    ) -> Option<Self> {
        let vertices = mesh.positions.len();
        let fits = |len: Option<usize>| len.is_none_or(|len| len == vertices);
        let aligned = mesh.normals.len() == vertices
            && mesh.uvs.len() == vertices
            && fits(mesh.colors.as_ref().map(Vec::len))
            && fits(streams.uv2.as_ref().map(Vec::len))
            && fits(streams.uv3.as_ref().map(Vec::len))
            && fits(streams.uv4.as_ref().map(Vec::len))
            && fits(streams.second_mocv_alpha.as_ref().map(Vec::len))
            && fits(streams.moc2.as_ref().map(Vec::len));
        if !header.group_flags.exterior || !portal_culled || blend_mode > 1 || !aligned {
            return None;
        }
        let center = |axis: usize| (header.bbox_min[axis] + header.bbox_max[axis]) * 0.5;
        let cell = |axis: usize| (center(axis) / MERGE_CELL_YARDS).floor() as i32;
        Some(Self {
            material: mesh.material_index,
            exterior_lit: super::doodad_light::group_exterior_lit(header.flags),
            casts_shadow: super::assets::group_casts_shadow(header.flags),
            colors: mesh.colors.is_some(),
            uv2: streams.uv2.is_some(),
            uv3: streams.uv3.is_some(),
            uv4: streams.uv4.is_some(),
            second_mocv_alpha: streams.second_mocv_alpha.is_some(),
            moc2: streams.moc2.is_some(),
            // WMO bounding boxes are Z-up: X and Y are horizontal.
            cell: (cell(0), cell(1)),
        })
    }

    pub(super) fn casts_shadow(&self) -> bool {
        self.casts_shadow
    }
}

/// The batches of one key, concatenated, and the groups they came from.
pub(super) struct MergedBatch {
    pub mesh: wmo::WmoMeshBatch,
    pub streams: RetailWmoStreams,
    pub groups: Vec<u32>,
}

impl MergedBatch {
    pub(super) fn new(mesh: wmo::WmoMeshBatch, streams: RetailWmoStreams, group: u32) -> Self {
        Self {
            mesh,
            streams,
            groups: vec![group],
        }
    }

    /// Appends a batch of the same key; its indices move past the vertices so far.
    pub(super) fn append(
        &mut self,
        mesh: &wmo::WmoMeshBatch,
        streams: &RetailWmoStreams,
        group: u32,
    ) {
        let offset = self.mesh.positions.len() as u32;
        self.mesh.positions.extend_from_slice(&mesh.positions);
        self.mesh.normals.extend_from_slice(&mesh.normals);
        self.mesh.uvs.extend_from_slice(&mesh.uvs);
        extend(&mut self.mesh.colors, &mesh.colors);
        self.mesh
            .indices
            .extend(mesh.indices.iter().map(|index| index + offset));
        extend(&mut self.streams.uv2, &streams.uv2);
        extend(&mut self.streams.uv3, &streams.uv3);
        extend(&mut self.streams.uv4, &streams.uv4);
        extend(
            &mut self.streams.second_mocv_alpha,
            &streams.second_mocv_alpha,
        );
        extend(&mut self.streams.moc2, &streams.moc2);
        if !self.groups.contains(&group) {
            self.groups.push(group);
        }
    }
}

/// Equal keys guarantee both are present or both absent.
fn extend<T: Clone>(into: &mut Option<Vec<T>>, from: &Option<Vec<T>>) {
    if let (Some(into), Some(from)) = (into, from) {
        into.extend_from_slice(from);
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
