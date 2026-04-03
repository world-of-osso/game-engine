use std::path::Path;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use game_engine::asset::adt::{load_adt_for_tile, load_adt_tex0};
use game_engine::asset::adt_format::adt_obj::load_adt_obj0;
use game_engine::asset::blp::load_blp_rgba;
use game_engine::asset::m2::load_m2_uncached;
use game_engine::asset::m2_particle::parse_particle_emitters;
use game_engine::asset::wmo::{load_wmo_group, load_wmo_root};
use game_engine::csv_util::{parse_csv_line, parse_csv_line_trimmed};

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
    let cases = [(
        "elwynn_32_48",
        Path::new("data/terrain/azeroth_32_48.adt"),
        Path::new("data/terrain/azeroth_32_48_tex0.adt"),
        Path::new("data/terrain/azeroth_32_48_obj0.adt"),
        32_u32,
        48_u32,
    )];
    let mut group = c.benchmark_group("asset::adt");
    for (label, root_path, tex_path, obj_path, tile_y, tile_x) in cases {
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
        let root_bytes = std::fs::read(root_path).expect("read benchmark ADT root");
        let tex_bytes = std::fs::read(tex_path).expect("read benchmark ADT tex");
        let obj_bytes = std::fs::read(obj_path).expect("read benchmark ADT obj");
        group.bench_with_input(
            BenchmarkId::new("load_adt_for_tile", label),
            &root_bytes,
            |b, bytes| {
                b.iter(|| {
                    load_adt_for_tile(bytes, tile_y, tile_x).expect("parse benchmark ADT root")
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("load_adt_tex0", label),
            &tex_bytes,
            |b, bytes| {
                b.iter(|| load_adt_tex0(bytes).expect("parse benchmark ADT tex"));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("load_adt_obj0", label),
            &obj_bytes,
            |b, bytes| {
                b.iter(|| load_adt_obj0(bytes).expect("parse benchmark ADT obj"));
            },
        );
    }
    group.finish();
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

criterion_group!(
    parser_benches,
    bench_csv_parsers,
    bench_m2_loading,
    bench_blp_loading,
    bench_adt_parsing,
    bench_wmo_parsing,
    bench_particle_emitter_parsing
);
criterion_main!(parser_benches);
