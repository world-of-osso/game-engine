use super::unified_gpu::{WallShading, render_batch_center};
use super::*;

/// Northshire Abbey (107074) group 0 is an interior group with MOCV: 13 interior
/// batches, then 1 exterior batch.
const NORTHSHIRE_ABBEY_ROOT_FDID: u32 = 107074;
const ABBEY_INTERIOR_BATCH: usize = 0;
const ABBEY_EXTERIOR_BATCH: usize = 13;

/// Retail lights interior-group vertices (fixed MOCV alpha 0) with the interior
/// ambient plus the doubled MOCV, and no sun: `texture * (ambient + 2 * MOCV)`
/// (WebWowViewerCpp `calcLight`, `precomputedLight = vColor.rgb * 2`).
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_interior_batch_is_lit_by_interior_ambient_plus_doubled_mocv() {
    assert_matches_retail_interior(ABBEY_INTERIOR_BATCH, wmo::WmoBatchType::Interior);
}

/// The exterior batch of an interior group is still interior-lit: only its MOCV
/// alpha, 0 after the fixup, selects the interior/exterior blend.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_exterior_batch_of_interior_group_is_interior_lit() {
    assert_matches_retail_interior(ABBEY_EXTERIOR_BATCH, wmo::WmoBatchType::Exterior);
}

fn assert_matches_retail_interior(batch_index: usize, batch_type: wmo::WmoBatchType) {
    let (root, group) = load_abbey_group_0();
    assert_eq!(group.batches[batch_index].batch_type, batch_type);
    let wall = render_batch_center(&root, &group, batch_index, WallShading::Wmo);
    let albedo = render_batch_center(&root, &group, batch_index, WallShading::Albedo);
    let retail = retail_interior_pixel(&root, &group, batch_index, albedo);
    println!("batch {batch_index}: wmo {wall:?}, retail {retail:?}");
    let close = wall
        .iter()
        .zip(retail)
        .all(|(wall, retail)| wall.abs_diff(retail) <= 4);
    assert!(
        close,
        "batch {batch_index}: wmo {wall:?} != retail {retail:?}"
    );
}

/// WebWowViewerCpp's default interior sun direction (WoW z-up (-0.30822, -0.30822,
/// -0.9)) in Bevy axes.
const RETAIL_INTERIOR_SUN_DIRECTION: Vec3 = Vec3::new(-0.30822, -0.9, 0.30822);

/// The Retail equation at the centroid of the batch's largest triangle, which the
/// camera centers on: `texel * applyAndMixAmbients(ambient + 2 * MOCV)`, no direct
/// light, in authored (sRGB byte) space. `albedo` is the texel as the GPU samples it.
fn retail_interior_pixel(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    batch_index: usize,
    albedo: [u8; 4],
) -> [u8; 4] {
    let batch = &group.batches[batch_index];
    let triangle = largest_triangle_indices(&batch.mesh);
    let mocv = centroid_of::<4>(&batch.mesh, Mesh::ATTRIBUTE_COLOR, triangle);
    let normal = Vec3::from(centroid_of::<3>(
        &batch.mesh,
        Mesh::ATTRIBUTE_NORMAL,
        triangle,
    ));
    let texel = Vec3::new(albedo[0] as f32, albedo[1] as f32, albedo[2] as f32) / 255.0;

    let hemisphere =
        Vec3::from(wmo_interior_ambient(root, 0)) + 2.0 * Vec3::new(mocv[0], mocv[1], mocv[2]);
    let n_dot_l = normal
        .normalize()
        .dot(-RETAIL_INTERIOR_SUN_DIRECTION.normalize())
        .clamp(0.0, 1.0);
    let light = (hemisphere * 0.7).lerp(hemisphere * 1.1, 0.5 + 0.5 * n_dot_l);
    let shaded = texel * light;
    [
        (shaded.x * 255.0).round().min(255.0) as u8,
        (shaded.y * 255.0).round().min(255.0) as u8,
        (shaded.z * 255.0).round().min(255.0) as u8,
        255,
    ]
}

fn largest_triangle_indices(mesh: &Mesh) -> [usize; 3] {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("batch has no positions");
    };
    let indices: Vec<usize> = mesh.indices().expect("indexed batch").iter().collect();
    indices
        .chunks_exact(3)
        .map(|tri| [tri[0], tri[1], tri[2]])
        .max_by(|x, y| {
            let area = |tri: &[usize; 3]| {
                let [a, b, c] = tri.map(|i| Vec3::from(positions[i]));
                (b - a).cross(c - a).length()
            };
            area(x).total_cmp(&area(y))
        })
        .expect("batch has triangles")
}

fn centroid_of<const N: usize>(
    mesh: &Mesh,
    attribute: bevy::mesh::MeshVertexAttribute,
    triangle: [usize; 3],
) -> [f32; N] {
    let values: Vec<Vec<f32>> = match mesh.attribute(attribute) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(values)) => {
            values.iter().map(|value| value.to_vec()).collect()
        }
        Some(bevy::mesh::VertexAttributeValues::Float32x3(values)) => {
            values.iter().map(|value| value.to_vec()).collect()
        }
        Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => {
            values.iter().map(|value| value.to_vec()).collect()
        }
        _ => panic!("batch lacks {}", attribute.name),
    };
    std::array::from_fn(|channel| {
        triangle
            .iter()
            .map(|&vertex| values[vertex][channel])
            .sum::<f32>()
            / 3.0
    })
}

fn load_abbey_group_0() -> (wmo::WmoRootData, wmo::WmoGroupData) {
    let root_data = std::fs::read(format!("data/models/{NORTHSHIRE_ABBEY_ROOT_FDID}.wmo"))
        .expect("Northshire Abbey root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse abbey root");
    let group_fdid = root.group_file_data_ids[0];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("Northshire Abbey group 0 WMO in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse group 0");
    assert!(group.header.group_flags.interior);
    (root, group)
}
