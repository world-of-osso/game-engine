//! Disc batches of the area-trigger ground models: textures, wrap, transforms and alpha
//! coverage, next to what our shader does with the same binding.
use game_engine_core::blp;
use game_engine_core::m2;
use game_engine_core::m2_material::{BatchBinding, MaterialTracks, batch_binding};

fn model(fdid: u32) -> m2::Model {
    let read = |name: String| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read(format!("{fdid}.m2")), &read(format!("{fdid}00.skin"))).unwrap()
}

fn texture(fdid: u32) -> Option<blp::RgbaImage> {
    let path = format!(
        "{}/../../data/textures/{fdid}.blp",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).ok()?;
    blp::decode_rgba(&bytes).ok()
}

/// Alpha coverage of a texture: the share of texels above 0.5 and the alpha at the centre and
/// at the four edge midpoints, which is what separates a round disc from a square.
fn alpha_report(fdid: u32) -> Option<String> {
    let image = texture(fdid)?;
    let (width, height) = (image.width as usize, image.height as usize);
    let alpha = |x: usize, y: usize| image.pixels[(y * width + x) * 4 + 3];
    let opaque = (0..width * height)
        .filter(|&index| image.pixels[index * 4 + 3] > 127)
        .count();
    let corner = alpha(0, 0);
    let edges = [
        alpha(width / 2, 0),
        alpha(width - 1, height / 2),
        alpha(width / 2, height - 1),
        alpha(0, height / 2),
    ];
    let centre = alpha(width / 2, height / 2);
    Some(format!(
        "{fdid} {width}x{height} opaque={:.2} centre={centre} edge_mid={edges:?} corner={corner}",
        opaque as f32 / (width * height) as f32
    ))
}

#[test]
fn disc_batches_report_texture_alpha_and_wrap() {
    for fdid in [1522961u32, 1248077, 1387770, 1387771] {
        let model = model(fdid);
        let tracks = MaterialTracks::of(&model);
        println!(
            "MODEL {fdid} batches={} textures={:?} texture_types={:?} anim={} trans={} colors={}",
            model.batches.len(),
            model.texture_fdids,
            model.texture_types,
            model.texture_animations.len(),
            model.transparency_tracks.len(),
            model.color_tracks.len()
        );
        let resolved =
            m2::resolve_render_batches(&model, &[], true, |fdid| Some(format!("/x/{fdid}.blp")))
                .unwrap();
        for batch in &resolved {
            let binding: BatchBinding =
                batch_binding(&model, &model.batches[batch.source_unit_index], &[])
                    .expect("binding");
            println!(
                "  unit {} submesh {} shader {:#06x} blend {} flags {:#04x} pixel {} vertex {} wrap {:#06x} transforms {:?} weights {:?} color {:?} weight0 {}",
                batch.source_unit_index,
                batch.submesh_index,
                batch.shader_id,
                batch.blend_mode,
                model.batches[batch.source_unit_index].flags,
                binding.pixel_shader,
                binding.vertex_shader,
                binding.texture_wrap,
                binding.texture_transforms,
                binding.texture_weights,
                binding.color,
                binding.apply_weight,
            );
            if let Some(submesh) = model.submeshes.get(batch.submesh_index) {
                let start = usize::from(submesh.vertex_start);
                let count = usize::from(submesh.vertex_count);
                let positions: Vec<[f32; 3]> = model.vertices[start..start + count]
                    .iter()
                    .map(|vertex| vertex.position)
                    .collect();
                let centre = positions.iter().fold([0.0f32; 3], |mut acc, p| {
                    acc[0] += p[0];
                    acc[1] += p[1];
                    acc[2] += p[2];
                    acc
                });
                let n = positions.len() as f32;
                let centre = [centre[0] / n, centre[1] / n, centre[2] / n];
                let radii: Vec<f32> = positions
                    .iter()
                    .map(|p| {
                        ((p[0] - centre[0]).powi(2)
                            + (p[1] - centre[1]).powi(2)
                            + (p[2] - centre[2]).powi(2))
                        .sqrt()
                    })
                    .collect();
                let max = radii.iter().copied().fold(0.0f32, f32::max);
                let min = radii.iter().copied().fold(f32::MAX, f32::min);
                let ring = radii.iter().filter(|&&r| r > max * 0.95).count();
                println!(
                    "    mesh verts={count} tris={} radius {min:.2}..{max:.2} outer_ring={ring}/{count}",
                    submesh.triangle_count
                );
            }
            for slot in &binding.textures {
                match slot.and_then(|fdid| alpha_report(fdid)) {
                    Some(report) => println!("    t{slot:?}: {report}"),
                    None => println!("    slot {slot:?}: texture not local"),
                }
            }
        }
        drop(tracks);
    }
}
