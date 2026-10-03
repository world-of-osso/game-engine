//! Opt-in branch bending without changing cached M2 meshes or shared materials.
use game_engine_core::elastic_tree::TreeAnnotation;
use glam::Vec3;
use godot::{
    classes::{ArrayMesh, MeshInstance3D, Node3D, RenderingServer, ShaderMaterial, mesh},
    prelude::*,
};

const BEND_BOUNDS_META: &str = "elastic_tree_bounds";
const PIVOT_PARAMETERS: [&str; 2] = ["tree_pivot_0", "tree_pivot_1"];
const ROTATION_PARAMETERS: [&str; 2] = ["tree_rotation_0", "tree_rotation_1"];

/// Prepare direct M2 Batch children. Each placement owns its weighted meshes;
/// skeleton bindings, node transforms, and material overrides remain unchanged.
pub fn prepare_batches(
    root: &Gd<Node3D>,
    annotation: &TreeAnnotation,
) -> Result<Vec<Gd<MeshInstance3D>>, String> {
    if annotation.branches.len() > 2 {
        return Err(format!(
            "Tree {} has {} branches; renderer supports at most two",
            annotation.model_fdid,
            annotation.branches.len()
        ));
    }
    let prepared = root
        .get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<MeshInstance3D>().ok())
        .filter(|batch| batch.get_name().to_string().starts_with("Batch"))
        .map(|batch| prepare_batch(batch, annotation))
        .collect::<Result<Vec<_>, _>>()?;
    if prepared.is_empty() {
        return Err(format!(
            "Tree {} has no M2 Batch children",
            annotation.model_fdid
        ));
    }
    // Do not change any live instance until every batch has been prepared.
    let mut batches: Vec<_> = prepared
        .into_iter()
        .map(|batch| install_batch(batch, annotation))
        .collect();
    update_bends(&mut batches, &[]);
    Ok(batches)
}

struct PreparedBatch {
    batch: Gd<MeshInstance3D>,
    material: Gd<ShaderMaterial>,
    weighted: Gd<ArrayMesh>,
    bounds: Aabb,
}

fn prepare_batch(
    batch: Gd<MeshInstance3D>,
    annotation: &TreeAnnotation,
) -> Result<PreparedBatch, String> {
    let context = format!("Tree {} batch {}", annotation.model_fdid, batch.get_name());
    let material = batch_material(&batch).map_err(|error| format!("{context}: {error}"))?;
    let source = batch
        .get_mesh()
        .ok_or_else(|| format!("{context}: missing mesh"))?
        .try_cast::<ArrayMesh>()
        .map_err(|_| format!("{context}: expected ArrayMesh"))?;
    let (weighted, padding) =
        build_weighted_mesh(&source, annotation).map_err(|error| format!("{context}: {error}"))?;
    let bounds = source
        .get_aabb()
        .merge(batch.get_custom_aabb())
        .grow(padding);
    Ok(PreparedBatch {
        batch,
        material,
        weighted,
        bounds,
    })
}

fn install_batch(prepared: PreparedBatch, annotation: &TreeAnnotation) -> Gd<MeshInstance3D> {
    let PreparedBatch {
        mut batch,
        mut material,
        weighted,
        bounds,
    } = prepared;
    batch.set_mesh(&weighted);
    batch.set_custom_aabb(bounds);
    batch.set_meta(BEND_BOUNDS_META, &bounds.to_variant());
    for (index, parameter) in PIVOT_PARAMETERS.iter().enumerate() {
        let pivot = annotation
            .branches
            .get(index)
            .map_or(Vector3::ZERO, |branch| Vector3::from_array(branch.pivot));
        material.set_shader_parameter(*parameter, &pivot.to_variant());
    }
    material.set_shader_parameter("tree_bend_enabled", &true.to_variant());
    batch
}

/// Upload model-local axis-angle vectors in radians; absent branches reset to rest.
/// Reapply prepared bounds because M2 sequence changes replace custom AABBs.
pub fn update_bends(batches: &mut [Gd<MeshInstance3D>], rotations: &[Vec3]) {
    for batch in batches {
        // prepare_batches validated the material before returning this placement.
        let mut material = batch_material(batch).expect("Prepared tree batch lost its material");
        for (index, parameter) in ROTATION_PARAMETERS.iter().enumerate() {
            let rotation = rotations.get(index).copied().unwrap_or(Vec3::ZERO);
            material.set_shader_parameter(
                *parameter,
                &Vector3::from_array(rotation.to_array()).to_variant(),
            );
        }
        let bounds = batch.get_meta(BEND_BOUNDS_META).to::<Aabb>();
        batch.set_custom_aabb(bounds);
    }
}

// build_model's load_material allocates a distinct ShaderMaterial for every batch.
// Using that existing ownership avoids Godot's world-wide instance uniform pool.
fn batch_material(batch: &Gd<MeshInstance3D>) -> Result<Gd<ShaderMaterial>, String> {
    batch
        .get_material_override()
        .ok_or("missing batch material")?
        .try_cast::<ShaderMaterial>()
        .map_err(|_| "expected M2 ShaderMaterial".into())
}

fn build_weighted_mesh(
    source: &Gd<ArrayMesh>,
    annotation: &TreeAnnotation,
) -> Result<(Gd<ArrayMesh>, f32), String> {
    if source.get_surface_count() == 0 {
        return Err("mesh has no surfaces".into());
    }
    let mut weighted = ArrayMesh::new_gd();
    weighted.set_blend_shape_mode(source.get_blend_shape_mode());
    weighted.set_lightmap_size_hint(source.get_lightmap_size_hint());
    for index in 0..source.get_blend_shape_count() {
        weighted.add_blend_shape(&source.get_blend_shape_name(index));
    }
    let mut padding = 0.0_f32;
    for surface in 0..source.get_surface_count() {
        let surface_padding = copy_weighted_surface(source, &mut weighted, surface, annotation)
            .map_err(|error| format!("surface {surface}: {error}"))?;
        padding = padding.max(surface_padding);
    }
    Ok((weighted, padding))
}

fn copy_weighted_surface(
    source: &Gd<ArrayMesh>,
    weighted: &mut Gd<ArrayMesh>,
    surface: i32,
    annotation: &TreeAnnotation,
) -> Result<f32, String> {
    let mut arrays = source.surface_get_arrays(surface);
    let padding = weight_surface(&mut arrays, annotation)?;
    let lods = read_surface_lods(source, surface)?;
    let blend_shapes: Array<AnyArray> = source
        .surface_get_blend_shape_arrays(surface)
        .iter_shared()
        .map(VarArray::upcast_any_array)
        .collect();
    weighted
        .add_surface_from_arrays_ex(source.surface_get_primitive_type(surface), &arrays)
        .blend_shapes(&blend_shapes)
        .lods(&lods)
        .flags(source.surface_get_format(surface) | mesh::ArrayFormat::COLOR)
        .done();
    if weighted.get_surface_count() != surface + 1 {
        return Err("Godot rejected weighted arrays".into());
    }
    weighted.surface_set_name(surface, &source.surface_get_name(surface));
    if let Some(material) = source.surface_get_material(surface) {
        weighted.surface_set_material(surface, &material);
    }
    Ok(padding)
}

fn weight_surface(arrays: &mut VarArray, annotation: &TreeAnnotation) -> Result<f32, String> {
    let vertices = arrays
        .get(mesh::ArrayType::VERTEX.ord() as usize)
        .ok_or("missing vertex stream")?
        .try_to::<PackedVector3Array>()
        .map_err(|error| format!("invalid vertex stream: {error}"))?;
    let color_slot = mesh::ArrayType::COLOR.ord() as usize;
    let color_stream = arrays.get(color_slot).ok_or("missing color slot")?;
    let mut colors = if color_stream.is_nil() {
        PackedColorArray::from(vec![Color::WHITE; vertices.len()].as_slice())
    } else {
        color_stream
            .try_to::<PackedColorArray>()
            .map_err(|error| format!("invalid color stream: {error}"))?
    };
    if colors.len() != vertices.len() {
        return Err("color and vertex counts differ".into());
    }
    let mut padding = 0.0_f32;
    for (index, vertex) in vertices.as_slice().iter().enumerate() {
        let position = Vec3::from_array(vertex.to_array());
        let weights = annotation.vertex_weights(position);
        let mut color = colors[index];
        color.r = weights[0];
        color.g = weights[1];
        colors[index] = color;
        padding = padding.max(bend_displacement_bound(position, weights, annotation));
    }
    arrays.set(color_slot, &colors.to_variant());
    Ok(padding)
}

fn bend_displacement_bound(position: Vec3, weights: [f32; 2], annotation: &TreeAnnotation) -> f32 {
    let mut displacement = 0.0;
    for (branch, weight) in annotation.branches.iter().zip(weights) {
        let radius = position.distance(Vec3::from_array(branch.pivot)) + displacement;
        let angle = (branch.max_angle * weight).abs().min(std::f32::consts::PI);
        // The second rotation can act on a vertex already displaced by the first.
        displacement += 2.0 * radius * (angle * 0.5).sin();
    }
    displacement
}

/// ArrayMesh has no bound LOD getter. RenderingServer exposes packed LOD indices.
fn read_surface_lods(source: &Gd<ArrayMesh>, surface: i32) -> Result<VarDictionary, String> {
    let data = RenderingServer::singleton().mesh_get_surface(source.get_rid(), surface);
    let mut result = VarDictionary::new();
    let Some(value) = data.get("lods") else {
        return Ok(result);
    };
    let lods = value
        .try_to::<VarArray>()
        .map_err(|error| format!("invalid LOD list: {error}"))?;
    let vertex_count = source.surface_get_array_len(surface);
    for value in lods.iter_shared() {
        let (distance, indices) = decode_lod(value, vertex_count)?;
        result.set(&distance, &indices);
    }
    Ok(result)
}

fn decode_lod(value: Variant, vertex_count: i32) -> Result<(Variant, PackedInt32Array), String> {
    let lod = value
        .try_to::<VarDictionary>()
        .map_err(|error| format!("invalid LOD entry: {error}"))?;
    let distance = lod.get("edge_length").ok_or("LOD missing edge length")?;
    let bytes = lod
        .get("index_data")
        .ok_or("LOD missing indices")?
        .try_to::<PackedByteArray>()
        .map_err(|error| format!("invalid LOD indices: {error}"))?;
    Ok((distance, decode_indices(&bytes, vertex_count)?))
}

fn decode_indices(bytes: &PackedByteArray, vertex_count: i32) -> Result<PackedInt32Array, String> {
    const U16_INDEX_CAPACITY: i32 = 65_536;
    let stride = if vertex_count > U16_INDEX_CAPACITY {
        4
    } else {
        2
    };
    if bytes.len() % stride != 0 {
        return Err("LOD index bytes have invalid length".into());
    }
    let indices: Vec<i32> = bytes
        .as_slice()
        .chunks_exact(stride)
        .map(|chunk| {
            if stride == 2 {
                i32::from(u16::from_le_bytes([chunk[0], chunk[1]]))
            } else {
                i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
            }
        })
        .collect();
    Ok(PackedInt32Array::from(indices.as_slice()))
}
