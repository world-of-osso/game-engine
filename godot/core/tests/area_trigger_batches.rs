//! Batches of the area-trigger ground models: raw shader id, decoded combiner and material.
use game_engine_core::m2;
use game_engine_core::m2_material::{gx_blend, pixel_shader_id, vertex_shader_id};

fn model(fdid: u32) -> m2::Model {
    let read = |name: String| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read(format!("{fdid}.m2")), &read(format!("{fdid}00.skin"))).unwrap()
}

#[test]
fn resolved_batches_report_textures_and_uv_sets() {
    for fdid in [1522961u32, 1248077, 1387770] {
        let model = model(fdid);
        let resolved = m2::resolve_render_batches(&model, &[], false, |_| None).unwrap();
        for batch in &resolved {
            println!(
                "RESOLVED {fdid} unit={} submesh={} shader={:#06x} blend={} tex={:?} tex2={:?} uv2_1={} uv2_2={} env2={} types={:?}",
                batch.source_unit_index,
                batch.submesh_index,
                batch.shader_id,
                batch.blend_mode,
                batch.texture_fdid,
                batch.texture_2_fdid,
                batch.use_uv_2_1,
                batch.use_uv_2_2,
                batch.use_env_map_2,
                batch.texture_type,
            );
        }
    }
}

#[test]
fn area_trigger_ground_batches_report_their_shader_ids() {
    for fdid in [1522961u32, 1248077, 1778192, 659607, 1387770, 1387771] {
        let model = model(fdid);
        println!(
            "MODEL {fdid} batches={} materials={:?} texture_types={:?}",
            model.batches.len(),
            model
                .materials
                .iter()
                .map(|mat| (mat.flags, mat.blend_mode))
                .collect::<Vec<_>>(),
            model.texture_types
        );
        for (index, batch) in model.batches.iter().enumerate() {
            let pixel = pixel_shader_id(batch.texture_count, batch.shader_id);
            let vertex = vertex_shader_id(batch.texture_count, batch.shader_id);
            let material = model
                .materials
                .get(usize::from(batch.material_layer))
                .map(|mat| (mat.flags, mat.blend_mode, gx_blend(mat.blend_mode)));
            println!(
                "  batch {index} shader_id={:#06x} tex_count={} submesh={} layer={} priority={} flags={:#04x} material={:?} pixel={:?} vertex={:?}",
                batch.shader_id,
                batch.texture_count,
                batch.submesh_index,
                batch.material_layer,
                batch.priority_plane,
                batch.flags,
                material,
                pixel,
                vertex,
            );
        }
    }
}
