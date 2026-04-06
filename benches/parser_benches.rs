use std::path::Path;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use game_engine::asset::adt::{load_adt_for_tile, load_adt_tex0};
use game_engine::asset::adt_format::adt_obj::load_adt_obj0;
use game_engine::asset::blp::load_blp_rgba;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::asset::m2::load_m2_uncached;
use game_engine::asset::m2_particle::parse_particle_emitters;
use game_engine::asset::wmo::{load_wmo_group, load_wmo_root};
use game_engine::csv_util::{parse_csv_line, parse_csv_line_trimmed};
use game_engine::particle_effect_builder::build_particle_effect_asset;

fn bench_csv_parsers(c: &mut Criterion) {
    let quoted = r#"5412968,"Skybox, Debug",1,"A, B, C",Trailing"#;
    let trimmed = r#" 5412968 , "Skybox" , 1 , "A, B, C" "#;

    c.bench_function("csv_util::parse_csv_line", |b| {
        b.iter(|| parse_csv_line(quoted));
    });
    c.bench_function("csv_util::parse_csv_line_trimmed", |b| {
        b.iter(|| parse_csv_line_trimmed(trimmed));
    });
}

fn bench_m2_loading(c: &mut Criterion) {
    let models = [
        ("torch", Path::new("data/models/145513.m2")),
        ("human_hd", Path::new("data/models/1011653.m2")),
    ];
    let mut group = c.benchmark_group("asset::m2::load_m2_uncached");
    for (label, path) in models {
        assert!(path.exists(), "missing benchmark model {}", path.display());
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, path| {
            b.iter(|| load_m2_uncached(path, &[0, 0, 0]).expect("load benchmark M2"));
        });
    }
    group.finish();
}

fn bench_blp_loading(c: &mut Criterion) {
    let textures = [
        ("torch_flame_small", Path::new("data/textures/198077.blp")),
        ("human_skin_large", Path::new("data/textures/1027767.blp")),
    ];
    let mut group = c.benchmark_group("asset::blp::load_blp_rgba");
    for (label, path) in textures {
        assert!(
            path.exists(),
            "missing benchmark texture {}",
            path.display()
        );
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, path| {
            b.iter(|| load_blp_rgba(path).expect("load benchmark BLP"));
        });
    }
    group.finish();
}

fn bench_adt_parsing(c: &mut Criterion) {
    let cases = [load_adt_bench_case(
        "elwynn_32_48",
        Path::new("data/terrain/azeroth_32_48.adt"),
        Path::new("data/terrain/azeroth_32_48_tex0.adt"),
        Path::new("data/terrain/azeroth_32_48_obj0.adt"),
        32_u32,
        48_u32,
    )];
    let mut group = c.benchmark_group("asset::adt");
    for case in &cases {
        register_adt_bench_case(&mut group, case);
    }
    group.finish();
}

struct AdtBenchCase {
    label: &'static str,
    root_bytes: Vec<u8>,
    tex_bytes: Vec<u8>,
    obj_bytes: Vec<u8>,
    tile_y: u32,
    tile_x: u32,
}

fn load_adt_bench_case(
    label: &'static str,
    root_path: &Path,
    tex_path: &Path,
    obj_path: &Path,
    tile_y: u32,
    tile_x: u32,
) -> AdtBenchCase {
    assert!(
        root_path.exists(),
        "missing benchmark ADT {}",
        root_path.display()
    );
    assert!(
        tex_path.exists(),
        "missing benchmark ADT tex {}",
        tex_path.display()
    );
    assert!(
        obj_path.exists(),
        "missing benchmark ADT obj {}",
        obj_path.display()
    );
    AdtBenchCase {
        label,
        root_bytes: std::fs::read(root_path).expect("read benchmark ADT root"),
        tex_bytes: std::fs::read(tex_path).expect("read benchmark ADT tex"),
        obj_bytes: std::fs::read(obj_path).expect("read benchmark ADT obj"),
        tile_y,
        tile_x,
    }
}

fn register_adt_bench_case(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    case: &AdtBenchCase,
) {
    group.bench_with_input(
        BenchmarkId::new("load_adt_for_tile", case.label),
        &case.root_bytes,
        |b, bytes| {
            b.iter(|| {
                load_adt_for_tile(bytes, case.tile_y, case.tile_x)
                    .expect("parse benchmark ADT root")
            });
        },
    );
    group.bench_with_input(
        BenchmarkId::new("load_adt_tex0", case.label),
        &case.tex_bytes,
        |b, bytes| {
            b.iter(|| load_adt_tex0(bytes).expect("parse benchmark ADT tex"));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("load_adt_obj0", case.label),
        &case.obj_bytes,
        |b, bytes| {
            b.iter(|| load_adt_obj0(bytes).expect("parse benchmark ADT obj"));
        },
    );
}

fn bench_wmo_parsing(c: &mut Criterion) {
    let root_cases = [("charselect_root", Path::new("data/models/4214993.wmo"))];
    let group_cases = [("abbey_group_000", Path::new("data/models/107075.wmo"))];
    let mut group = c.benchmark_group("asset::wmo");
    for (label, path) in root_cases {
        assert!(
            path.exists(),
            "missing benchmark WMO root {}",
            path.display()
        );
        let bytes = std::fs::read(path).expect("read benchmark WMO root");
        group.bench_with_input(
            BenchmarkId::new("load_wmo_root", label),
            &bytes,
            |b, bytes| {
                b.iter(|| load_wmo_root(bytes).expect("parse benchmark WMO root"));
            },
        );
    }
    for (label, path) in group_cases {
        assert!(
            path.exists(),
            "missing benchmark WMO group {}",
            path.display()
        );
        let bytes = std::fs::read(path).expect("read benchmark WMO group");
        group.bench_with_input(
            BenchmarkId::new("load_wmo_group", label),
            &bytes,
            |b, bytes| {
                b.iter(|| load_wmo_group(bytes).expect("parse benchmark WMO group"));
            },
        );
    }
    group.finish();
}

fn bench_particle_emitter_parsing(c: &mut Criterion) {
    let cases = [("torch", Path::new("data/models/145513.m2"))];
    let mut group = c.benchmark_group("asset::m2_particle::parse_particle_emitters");
    for (label, path) in cases {
        assert!(
            path.exists(),
            "missing benchmark particle model {}",
            path.display()
        );
        let model_bytes = std::fs::read(path).expect("read benchmark particle model");
        let md20 = find_md21_chunk(&model_bytes).expect("benchmark particle model md21");
        group.bench_with_input(BenchmarkId::from_parameter(label), &md20, |b, md20| {
            b.iter(|| parse_particle_emitters(md20));
        });
    }
    group.finish();
}

fn bench_particle_effect_asset_build(c: &mut Criterion) {
    let cases = [("torch", Path::new("data/models/145513.m2"))];
    let mut group = c.benchmark_group("particle_effect_builder::build_particle_effect_asset");
    for (label, path) in cases {
        assert!(
            path.exists(),
            "missing benchmark particle model {}",
            path.display()
        );
        let loaded = load_m2_uncached(path, &[0, 0, 0]).expect("load benchmark particle model");
        let emitter = loaded
            .particle_emitters
            .first()
            .expect("benchmark particle emitter");
        group.bench_with_input(BenchmarkId::from_parameter(label), emitter, |b, emitter| {
            b.iter(|| build_particle_effect_asset(emitter, 1.0, 1.0));
        });
    }
    group.finish();
}

fn bench_character_texture_compositing(c: &mut Criterion) {
    let data_dir = Path::new("data");
    assert!(
        data_dir.exists(),
        "missing benchmark data dir {}",
        data_dir.display()
    );
    let texture_data = CharTextureData::load(data_dir);
    let cases = [
        ("hd_base", Vec::<(u16, u32)>::new(), Vec::<(u8, u32)>::new()),
        (
            "hd_boot_overlay",
            Vec::<(u16, u32)>::new(),
            vec![(6, 155028), (7, 152769)],
        ),
    ];
    let mut group = c.benchmark_group("asset::char_texture::composite_model_textures");
    for (label, materials, item_textures) in &cases {
        group.bench_with_input(BenchmarkId::from_parameter(label), label, |b, _| {
            b.iter(|| {
                texture_data
                    .composite_model_textures(materials, item_textures, 103)
                    .expect("benchmark composited character textures")
            });
        });
    }
    group.finish();
}

fn find_md21_chunk(data: &[u8]) -> Option<&[u8]> {
    let mut off = 0;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().ok()?) as usize;
        let end = off + 8 + size;
        if end > data.len() {
            return None;
        }
        if tag == b"MD21" {
            return Some(&data[off + 8..end]);
        }
        off = end;
    }
    None
}

fn bench_skin_fdid_resolution(c: &mut Criterion) {
    let data_dir = Path::new("data");
    let outfit_data = game_engine::outfit_data::OutfitData::load(data_dir);
    // Warm up the lazy load
    let _ = outfit_data.resolve_item_model_skin_fdids_for_model_path(Path::new("dummy.m2"));

    let cases: &[(&str, &Path)] = &[
        (
            "named_torch",
            Path::new("data/models/club_1h_torch_a_01.m2"),
        ),
        ("fdid_torch", Path::new("data/models/145513.m2")),
    ];
    let mut group = c.benchmark_group("outfit_data::resolve_skin_fdids");
    for &(label, path) in cases {
        if !path.exists() {
            eprintln!("skipping bench {label}: {} not found", path.display());
            continue;
        }
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, path| {
            b.iter(|| outfit_data.resolve_item_model_skin_fdids_for_model_path(path));
        });
    }
    group.finish();
}

fn bench_minimap_blit(c: &mut Criterion) {
    let comp_size = 768;
    let tile_px = 256;
    let mut composite = vec![0u8; comp_size * comp_size * 4];
    let tile_data = vec![128u8; tile_px * tile_px * 4];

    c.bench_function("minimap::blit_image_256", |b| {
        b.iter(|| {
            game_engine::minimap_render::blit_image(
                &mut composite,
                comp_size,
                &tile_data,
                tile_px,
                256,
                256,
            );
        });
    });
}

fn bench_minimap_crop(c: &mut Criterion) {
    let comp_size = 768;
    let composite = vec![128u8; comp_size * comp_size * 4];

    c.bench_function("minimap::crop_with_circle_200", |b| {
        b.iter(|| {
            game_engine::minimap_render::crop_with_circle(&composite, comp_size, 384, 384, 200);
        });
    });
}

fn bench_minimap_render_tile(c: &mut Criterion) {
    use game_engine::asset::adt::ChunkHeightGrid;

    // Build 256 synthetic chunks (16x16 grid) with realistic height data.
    let chunks: Vec<Option<ChunkHeightGrid>> = (0..256)
        .map(|i| {
            let ix = i % 16;
            let iy = i / 16;
            let mut heights = [0.0f32; 145];
            for (hi, h) in heights.iter_mut().enumerate() {
                *h = 100.0 + (ix as f32 * 10.0) + (iy as f32 * 5.0) + (hi as f32 * 0.1);
            }
            Some(ChunkHeightGrid {
                index_x: ix as u32,
                index_y: iy as u32,
                origin_x: 0.0,
                origin_z: 0.0,
                base_y: 100.0,
                heights,
            })
        })
        .collect();

    c.bench_function("minimap::render_tile_image_256", |b| {
        b.iter(|| game_engine::minimap_render::render_tile_image(&chunks, 256));
    });
}

fn bench_adt_texture_decode(c: &mut Criterion) {
    let tex_path = Path::new("data/terrain/azeroth_32_48_tex0.adt");
    if !tex_path.exists() {
        eprintln!(
            "skipping bench_adt_texture_decode: {} not found",
            tex_path.display()
        );
        return;
    }
    let tex_bytes = std::fs::read(tex_path).expect("read benchmark ADT tex");
    let tex_data = load_adt_tex0(&tex_bytes).expect("parse benchmark ADT tex");

    c.bench_function("adt::texture_decode_blp_for_tile", |b| {
        b.iter(|| {
            for fdid in &tex_data.texture_fdids {
                let path = format!("data/textures/{fdid}.blp");
                let _ = load_blp_rgba(Path::new(&path));
            }
        });
    });
}

criterion_group!(
    parser_benches,
    bench_csv_parsers,
    bench_m2_loading,
    bench_blp_loading,
    bench_adt_parsing,
    bench_wmo_parsing,
    bench_particle_emitter_parsing,
    bench_particle_effect_asset_build,
    bench_character_texture_compositing,
    bench_skin_fdid_resolution,
    bench_minimap_blit,
    bench_minimap_crop,
    bench_minimap_render_tile,
    bench_adt_texture_decode
);
criterion_main!(parser_benches);
