use criterion::{Criterion, criterion_group, criterion_main};
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

criterion_group!(parser_benches, bench_csv_parsers);
criterion_main!(parser_benches);
