use std::path::Path;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use game_engine::asset::m2::load_m2_uncached;
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

criterion_group!(parser_benches, bench_csv_parsers, bench_m2_loading);
criterion_main!(parser_benches);
