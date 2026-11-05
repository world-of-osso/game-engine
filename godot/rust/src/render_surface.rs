//! Client-side source of the visible render surface: WMO render batches and M2 render
//! geometry, as they are drawn.
//!
//! Collision keeps the MOPY collision-only WMO faces, so render stairs and carpets can sit
//! above the collision floor. This cache reads the mesh arrays of spawned nodes once, keeps
//! their triangles in the mesh's own space and builds one BVH per root node in world space.
//! Terrain heights join the same query in `TerrainObjects::visible_surface`.
//!
//! Movement, the server and the navmesh never read this.

use std::{collections::HashMap, rc::Rc};

use game_engine_core::render_surface::RenderSurface;
use glam::{Affine3A, Vec3};
use godot::{
    classes::{ArrayMesh, MeshInstance3D, Node3D, mesh},
    prelude::*,
};

/// `Transform3D` to the `glam` affine the client already uses for doodad placement.
pub(crate) fn affine(transform: Transform3D) -> Affine3A {
    let column = |vector: Vector3| Vec3::new(vector.x, vector.y, vector.z);
    Affine3A::from_cols(
        column(transform.basis.col_a()).into(),
        column(transform.basis.col_b()).into(),
        column(transform.basis.col_c()).into(),
        column(transform.origin).into(),
    )
}

/// One mesh's triangles in the mesh's own space.
pub(crate) struct MeshTriangles {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

/// Mesh triangle sets and per-root world surfaces, built on first use.
#[derive(Default)]
pub(crate) struct RenderSurfaceCache {
    meshes: HashMap<InstanceId, Option<Rc<MeshTriangles>>>,
    surfaces: HashMap<InstanceId, Option<Rc<RenderSurface>>>,
    roots: HashMap<InstanceId, Option<Rc<RenderSurface>>>,
}

impl RenderSurfaceCache {
    /// Drops every cached surface: spawned nodes and their meshes were freed or replaced.
    pub(crate) fn clear(&mut self) {
        self.meshes.clear();
        self.surfaces.clear();
        self.roots.clear();
    }

    /// The world-space render surface drawn under `root`, reading its mesh children once.
    ///
    /// A mesh on a skeleton is skipped: it draws the animated pose, not the authored
    /// vertices these arrays hold.
    pub(crate) fn root_surface(&mut self, root: &Gd<Node3D>) -> Option<Rc<RenderSurface>> {
        let key = root.instance_id();
        if let Some(cached) = self.roots.get(&key) {
            return cached.clone();
        }
        let surface = self.build_root(root).map(Rc::new);
        self.roots.insert(key, surface.clone());
        surface
    }

    fn build_root(&mut self, root: &Gd<Node3D>) -> Option<RenderSurface> {
        let meshes = root
            .find_children_ex("*")
            .type_("MeshInstance3D")
            .owned(false)
            .recursive(true)
            .done();
        let mut vertices: Vec<Vec3> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();
        for node in meshes.iter_shared() {
            let Ok(instance) = node.try_cast::<MeshInstance3D>() else {
                continue;
            };
            if !instance.get_skeleton_path().is_empty() {
                continue;
            }
            let Some(mesh) = instance
                .get_mesh()
                .and_then(|mesh| mesh.try_cast::<ArrayMesh>().ok())
            else {
                continue;
            };
            let Some(local) = self.mesh_triangles(&mesh) else {
                continue;
            };
            let world_from_mesh = affine(instance.get_global_transform());
            let base = vertices.len() as u32;
            vertices.extend(
                local
                    .vertices
                    .iter()
                    .map(|&vertex| world_from_mesh.transform_point3(vertex)),
            );
            triangles.extend(
                local
                    .triangles
                    .iter()
                    .map(|triangle| triangle.map(|index| index + base)),
            );
        }
        RenderSurface::build(vertices, triangles)
    }

    /// The triangles of a mesh as authored, in the mesh's own space, read once.
    pub(crate) fn mesh_triangles(&mut self, mesh: &Gd<ArrayMesh>) -> Option<Rc<MeshTriangles>> {
        let key = mesh.instance_id();
        if let Some(cached) = self.meshes.get(&key) {
            return cached.clone();
        }
        let extracted = extract_mesh(mesh).map(Rc::new);
        self.meshes.insert(key, extracted.clone());
        extracted
    }

    /// The authored triangles of a mesh as a BVH surface, in the mesh's own space.
    pub(crate) fn mesh_surface(&mut self, mesh: &Gd<ArrayMesh>) -> Option<Rc<RenderSurface>> {
        let key = mesh.instance_id();
        if let Some(cached) = self.surfaces.get(&key) {
            return cached.clone();
        }
        let surface = self
            .mesh_triangles(mesh)
            .and_then(|triangles| {
                RenderSurface::build(triangles.vertices.clone(), triangles.triangles.clone())
            })
            .map(Rc::new);
        self.surfaces.insert(key, surface.clone());
        surface
    }
}

/// Reads every surface of `mesh`: positions and indices, merged into one triangle list.
fn extract_mesh(mesh: &Gd<ArrayMesh>) -> Option<MeshTriangles> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for surface in 0..mesh.get_surface_count() {
        let arrays = mesh.surface_get_arrays(surface);
        let Some(positions) = arrays
            .get(mesh::ArrayType::VERTEX.ord() as usize)
            .and_then(|value| value.try_to::<PackedVector3Array>().ok())
        else {
            continue;
        };
        let base = vertices.len() as u32;
        vertices.extend(
            positions
                .as_slice()
                .iter()
                .map(|vertex| Vec3::new(vertex.x, vertex.y, vertex.z)),
        );
        let indices = arrays
            .get(mesh::ArrayType::INDEX.ord() as usize)
            .and_then(|value| value.try_to::<PackedInt32Array>().ok())
            .filter(|indices| !indices.is_empty());
        let source: Vec<u32> = match indices {
            Some(indices) => indices
                .as_slice()
                .iter()
                .filter_map(|index| (*index >= 0).then_some(*index as u32))
                .collect(),
            None => (0..positions.len() as u32).collect(),
        };
        triangles.extend(
            complete_triangles(source.as_slice(), positions.len())
                .into_iter()
                .map(|triangle| triangle.map(|index| base + index)),
        );
    }
    (!triangles.is_empty()).then_some(MeshTriangles {
        vertices,
        triangles,
    })
}

/// Keeps only complete triangles and drops vertices outside the surface.
fn complete_triangles(indices: &[u32], vertices: usize) -> Vec<[u32; 3]> {
    indices
        .chunks_exact(3)
        .filter(|triangle| triangle.iter().all(|&index| (index as usize) < vertices))
        .map(|triangle| [triangle[0], triangle[1], triangle[2]])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wmo::assets::NativeWmoAsset;
    use game_engine_core::render_surface::SurfaceSet;
    use shared::ground::WmoCollision;

    fn data_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn read_wmo(fdid: u32) -> NativeWmoAsset {
        let data_root = data_root();
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        crate::wmo::assets::read_wmo(&resolver, &data_root, fdid).expect("local WMO")
    }

    /// Render batches of `asset` in WMO-local space, one surface per group.
    fn render_surfaces(asset: &NativeWmoAsset) -> SurfaceSet {
        let mut surfaces = SurfaceSet::default();
        for group in &asset.groups {
            let mut vertices = Vec::new();
            let mut triangles = Vec::new();
            for batch in &group.batches {
                let base = vertices.len() as u32;
                vertices.extend(batch.positions.iter().map(|point| Vec3::from_array(*point)));
                triangles.extend(batch.indices.chunks_exact(3).map(|triangle| {
                    [
                        base + triangle[0] as u32,
                        base + triangle[1] as u32,
                        base + triangle[2] as u32,
                    ]
                }));
            }
            if let Some(surface) = RenderSurface::build(vertices, triangles) {
                surfaces.push(surface);
            }
        }
        surfaces
    }

    fn collision(asset: &NativeWmoAsset) -> WmoCollision {
        WmoCollision::new(
            Affine3A::IDENTITY,
            asset
                .groups
                .iter()
                .map(|group| group.collision.clone())
                .collect(),
        )
    }

    /// Stormwind's city WMO draws stair treads over collision-only ramps; the query must
    /// return the tread, not the ramp under it.
    #[test]
    fn stormwind_render_stairs_are_the_highest_visible_surface() {
        let asset = read_wmo(107_243);
        let surfaces = render_surfaces(&asset);
        let collision = collision(&asset);
        assert!(surfaces.len() > 1, "stormwind renders several groups");
        let mut heights = Vec::new();
        let mut checked = 0usize;
        let mut over_ramp = 0usize;
        let mut sample = None;
        for part in 0..surfaces.len() {
            let Some(surface) = surfaces.part(part) else {
                continue;
            };
            // A tread triangle's centroid is a column on that tread.
            let stride = (surface.triangle_count() / 97).max(1);
            for index in (0..surface.triangle_count()).step_by(stride) {
                let Some(centroid) = surface.triangle_centroid(index) else {
                    continue;
                };
                let (x, z, tread) = (centroid.x, centroid.z, centroid.y);
                checked += 1;
                heights.clear();
                collision.collision_heights(x, z, tread + 1.6, tread - 0.2, &mut heights);
                let Some(floor) = heights.iter().copied().max_by(f32::total_cmp) else {
                    continue;
                };
                let render = surfaces.highest_upward_below(x, z, floor + 1.6, floor);
                if let Some(render) = render.map(|(height, _)| height) {
                    if render - floor > 0.02 {
                        over_ramp += 1;
                        sample = sample.or(Some((x, z, floor, render)));
                    }
                }
            }
        }
        assert!(checked > 100, "checked {checked} render columns");
        assert!(
            over_ramp > 0,
            "no stormwind render tread stands above its collision face"
        );
        let (x, z, floor, render) = sample.expect("first raised tread");
        assert!(render > floor, "({x}, {z}) render {render} > floor {floor}");
    }
}
