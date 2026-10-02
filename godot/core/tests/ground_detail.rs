//! Detail-doodad scatter and mesh expansion against the original client's output: the
//! fixtures are solarityclient's `ground-detail-native.txt` / `ground-detail-mesh-native.txt`
//! (build 12340 functions 7D3390 and 7B1B50 run on the chunk each case describes).

use std::collections::HashMap;

use game_engine_core::{
    adt,
    ground_detail::{
        BlizzardRand, DetailChunk, DetailModel, EffectDoodad, EffectTexture, GroundEffects,
        build_mesh, chunk_origin, chunk_seed, scatter, wow_to_engine,
    },
    wdt,
};

#[path = "fixtures/ground_detail_native.rs"]
mod native;

use native::{MESHES, PLACEMENTS};

#[test]
fn blizzard_rand_reproduces_client_table_sequence() {
    let mut random = BlizzardRand::new(0x1234_5678);
    let words: Vec<u32> = (0..4).map(|_| random.next_u32()).collect();
    assert_eq!(words, [0x28ea_c7b8, 0xccbc_2deb, 0x2889_ad69, 0x0e71_8081]);
}

struct Case {
    seed: u32,
    density: u16,
    effect_density: u32,
    holes: u16,
    stencil: u64,
    slope: f64,
    colors: bool,
    shadow: bool,
    count: usize,
}

fn cases() -> Vec<(Case, Vec<Vec<f64>>)> {
    let mut lines = PLACEMENTS.lines().filter(|line| !line.starts_with('#'));
    let mut cases = Vec::new();
    while let Some(line) = lines.next() {
        let fields: Vec<&str> = line.split_ascii_whitespace().collect();
        assert_eq!(fields[0], "case");
        let case = Case {
            seed: fields[1].parse().unwrap(),
            density: fields[2].parse().unwrap(),
            effect_density: fields[3].parse().unwrap(),
            holes: fields[4].parse().unwrap(),
            stencil: fields[5].parse().unwrap(),
            slope: fields[6].parse().unwrap(),
            colors: fields[7] == "1",
            shadow: fields[8] == "1",
            count: fields[9].parse().unwrap(),
        };
        let rows = (0..case.count)
            .map(|_| {
                lines
                    .next()
                    .unwrap()
                    .split_ascii_whitespace()
                    .map(|value| value.parse().unwrap())
                    .collect()
            })
            .collect();
        cases.push((case, rows));
    }
    cases
}

/// The fixture chunk: a plane of `slope` along north plus 0.1 along west.
fn heights(slope: f64) -> [f32; 145] {
    let mut heights = [0.0; 145];
    for row in 0..17usize {
        for column in 0..if row % 2 == 0 { 9 } else { 8 } {
            let index = (row / 2) * 17 + if row % 2 == 0 { 0 } else { 9 } + column;
            let x = -(row as f64 / 2.0) * (25.0 / 6.0);
            let y = -(column as f64 + if row % 2 == 0 { 0.0 } else { 0.5 }) * (25.0 / 6.0);
            heights[index] = (slope * x + 0.1 * y) as f32;
        }
    }
    heights
}

/// The fixture MCCV, in file (BGRA) order.
fn colors() -> [[u8; 4]; 145] {
    std::array::from_fn(|index| {
        [
            40 + (index % 70) as u8,
            55 + (index % 60) as u8,
            65 + (index % 50) as u8,
            255,
        ]
    })
}

/// Effect 1 scatters doodads 1, 2 and 3 (weights 5, 3, 0); doodad 2 aligns to the face
/// and doodad 3 keeps its own colour.
fn effects(density: u32) -> GroundEffects {
    let textures = HashMap::from([(
        1,
        EffectTexture {
            density,
            doodads: [1, 2, 0, 3],
            weights: [5, 3, 0, 0],
        },
    )]);
    let doodads = [(1, 0), (2, 1), (3, 2)]
        .into_iter()
        .map(|(id, flags)| {
            (
                id,
                EffectDoodad {
                    model_fdid: 100 + id,
                    flags,
                },
            )
        })
        .collect();
    GroundEffects::new(textures, doodads)
}

fn assert_close(actual: &[f32], expected: &[f64], context: &str) {
    for (component, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (f64::from(*actual) - expected).abs() < 0.0001,
            "{context} component {component}: {actual} != {expected}"
        );
    }
}

/// RGBA of a fixture's packed ARGB colour.
fn rgba(packed: f64) -> [u8; 4] {
    let [alpha, red, green, blue] = (packed as u32).to_be_bytes();
    [red, green, blue, alpha]
}

#[test]
fn scatter_matches_original_client_placements() {
    let shadow: Vec<u8> = [0xaa, 0x55].repeat(256);
    let shadow: [u8; 512] = shadow.try_into().unwrap();
    for (case, rows) in cases() {
        let heights = heights(case.slope);
        let chunk = DetailChunk {
            heights: &heights,
            vertex_colors_bgra: case.colors.then(colors),
            shadow: case.shadow.then_some(&shadow),
            holes_low_res: case.holes,
            holes_high_res: None,
            texture_selection: [0; 8],
            detail_exclusion: case.stencil.to_le_bytes(),
            layer_effects: &[1],
        };
        let placements = scatter(
            &chunk,
            &effects(case.effect_density),
            case.seed,
            case.density,
        )
        .unwrap();
        assert_eq!(placements.len(), case.count, "case seed {}", case.seed);
        for (index, (placement, row)) in placements.iter().zip(&rows).enumerate() {
            let context = format!("case seed {} placement {index}", case.seed);
            assert_eq!(placement.doodad, row[0] as u32, "{context}");
            assert_eq!(placement.face, row[9] as u16, "{context}");
            assert_eq!(placement.color, rgba(row[10]), "{context}");
            let actual = [
                placement.position.as_slice(),
                &[placement.angle, placement.scale],
                placement.normal.as_slice(),
            ]
            .concat();
            assert_close(&actual, &row[1..9], &context);
        }
    }
}

/// The fixture's quad: skin vertex lookup 2, 0, 3, 1 of a 2-yard-tall card.
fn quad() -> DetailModel {
    DetailModel {
        texture_fdid: 1,
        vertices: vec![
            ([-1.0, 0.0, 2.0], [0.0, 0.0]),
            ([-1.0, 0.0, 0.0], [0.0, 1.0]),
            ([1.0, 0.0, 2.0], [1.0, 0.0]),
            ([1.0, 0.0, 0.0], [1.0, 1.0]),
        ],
        indices: vec![0, 1, 2, 2, 1, 3],
    }
}

#[test]
fn mesh_matches_original_client_vertex_expansion() {
    let shadow: Vec<u8> = [0xaa, 0x55].repeat(256);
    let shadow: [u8; 512] = shadow.try_into().unwrap();
    let model = quad();
    let mut lines = MESHES.lines().filter(|line| !line.starts_with('#'));
    for (case, _) in cases() {
        let heights = heights(case.slope);
        let chunk = DetailChunk {
            heights: &heights,
            vertex_colors_bgra: case.colors.then(colors),
            shadow: case.shadow.then_some(&shadow),
            holes_low_res: case.holes,
            holes_high_res: None,
            texture_selection: [0; 8],
            detail_exclusion: case.stencil.to_le_bytes(),
            layer_effects: &[1],
        };
        let effects = effects(case.effect_density);
        let placements = scatter(&chunk, &effects, case.seed, case.density).unwrap();
        let mesh = build_mesh(&placements, &effects, case.density, |_| Some(&model)).unwrap();
        let expected: usize = lines
            .next()
            .unwrap()
            .strip_prefix("case ")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(mesh.vertices.len(), expected, "case seed {}", case.seed);
        for (index, vertex) in mesh.vertices.iter().enumerate() {
            let row: Vec<f64> = lines
                .next()
                .unwrap()
                .split_ascii_whitespace()
                .map(|value| value.parse().unwrap())
                .collect();
            let context = format!("case seed {} vertex {index}", case.seed);
            assert_eq!(vertex.color, rgba(row[6]), "{context}");
            let actual = [
                vertex.position.as_slice(),
                vertex.normal.as_slice(),
                vertex.uv.as_slice(),
            ]
            .concat();
            let expected = [&row[..6], &row[7..9]].concat();
            assert_close(&actual, &expected, &context);
        }
    }
}

fn cached(path: &str) -> Vec<u8> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/");
    std::fs::read(format!("{root}{path}")).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// Northshire (azeroth_32_48) chunk 0: header 0x40 selects its MCLY layers per cell, and
/// the retail GroundEffectTexture rows of those layers scatter Elwynn detail models.
#[test]
fn northshire_chunk_scatters_its_layers_elwynn_detail_models() {
    let tex_bytes = cached("terrain/azeroth_32_48_tex0.adt");
    let root = adt::parse_root_for_tile(
        &cached("terrain/azeroth_32_48.adt"),
        32,
        48,
        Some(&tex_bytes),
    )
    .unwrap();
    let tex = adt::parse_tex(&tex_bytes, wdt::MphdFlags { raw: 0x4 }, &root).unwrap();
    let effects = GroundEffects::parse(
        &String::from_utf8(cached("db2/12.1.0.69933/GroundEffectTexture.csv")).unwrap(),
        &String::from_utf8(cached("db2/12.1.0.69933/GroundEffectDoodad.csv")).unwrap(),
    )
    .unwrap();
    let elwynn_grass = effects.texture(757).unwrap();
    assert_eq!(elwynn_grass.doodads, [4, 5, 7, 9]);
    assert_eq!(effects.doodad(4).unwrap().model_fdid, 218812);

    let chunk = &root.chunks[0];
    // Raw header 0x40 row 0 bytes 00 55: cells 4..7 select layer 1.
    assert_eq!(chunk.texture_selection[0], 0x5500);
    assert_eq!(chunk.detail_exclusion, [0; 8]);
    let layers: Vec<u32> = tex.chunk_layers[0]
        .layers
        .iter()
        .map(|layer| layer.effect_id)
        .collect();
    assert_eq!(layers, [993, 1106]);
    let allowed: Vec<u32> = layers
        .iter()
        .filter_map(|&id| effects.texture(id))
        .flat_map(|effect| effect.doodads)
        .collect();
    let seed = chunk_seed((32, 48), chunk.index_x, chunk.index_y);
    let placements = scatter(&DetailChunk::from_adt(chunk, &layers), &effects, seed, 64).unwrap();
    assert!(placements.len() > 64, "{} placements", placements.len());
    for placement in &placements {
        assert!(allowed.contains(&placement.doodad), "{placement:?}");
    }
}

#[test]
fn chunk_seed_puts_global_row_high_and_column_low() {
    // Tile azeroth_32_48 (first file number 32 runs along columns), chunk column 3, row 5.
    assert_eq!(
        chunk_seed((32, 48), 3, 5),
        ((48 * 16 + 5) << 16) | (32 * 16 + 3)
    );
}

/// A placement converted to engine axes stands on the chunk's terrain surface: its height
/// matches the terrain triangle under it (vertex 0 frame of `adt::chunk_geometry`).
#[test]
fn placements_stand_on_the_rendered_terrain() {
    let tex_bytes = cached("terrain/azeroth_32_48_tex0.adt");
    let root = adt::parse_root_for_tile(
        &cached("terrain/azeroth_32_48.adt"),
        32,
        48,
        Some(&tex_bytes),
    )
    .unwrap();
    let tex = adt::parse_tex(&tex_bytes, wdt::MphdFlags { raw: 0x4 }, &root).unwrap();
    let effects = GroundEffects::parse(
        &String::from_utf8(cached("db2/12.1.0.69933/GroundEffectTexture.csv")).unwrap(),
        &String::from_utf8(cached("db2/12.1.0.69933/GroundEffectDoodad.csv")).unwrap(),
    )
    .unwrap();
    let mut total = 0;
    for (chunk, layers) in root.chunks.iter().zip(&tex.chunk_layers) {
        let layers: Vec<u32> = layers.layers.iter().map(|layer| layer.effect_id).collect();
        let seed = chunk_seed((32, 48), chunk.index_x, chunk.index_y);
        let placements =
            scatter(&DetailChunk::from_adt(chunk, &layers), &effects, seed, 64).unwrap();
        let origin = chunk_origin(chunk, (32, 48));
        let geometry = adt::chunk_geometry(chunk, Some((32, 48)));
        assert_eq!(
            [geometry.positions[0][0], geometry.positions[0][2]],
            [origin[0], origin[2]]
        );
        assert_eq!(geometry.positions[0][1], origin[1] + chunk.heights[0]);
        for placement in &placements {
            let local = wow_to_engine(placement.position);
            let point = [
                origin[0] + local[0],
                origin[1] + local[1],
                origin[2] + local[2],
            ];
            let surface = terrain_height(&geometry, point[0], point[2])
                .unwrap_or_else(|| panic!("{placement:?} outside chunk {}", chunk.index_x));
            assert!(
                (point[1] - surface).abs() < 1e-2,
                "{placement:?}: {} vs {surface}",
                point[1]
            );
        }
        total += placements.len();
    }
    assert!(total > 10_000, "{total} placements on the tile");
}

/// Height of the terrain triangle containing engine `(x, z)`.
fn terrain_height(geometry: &adt::Geometry, x: f32, z: f32) -> Option<f32> {
    geometry.indices.chunks_exact(3).find_map(|triangle| {
        let [a, b, c] = [0, 1, 2].map(|corner| geometry.positions[triangle[corner] as usize]);
        let area = (b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2]);
        let u = ((b[0] - x) * (c[2] - z) - (c[0] - x) * (b[2] - z)) / area;
        let v = ((c[0] - x) * (a[2] - z) - (a[0] - x) * (c[2] - z)) / area;
        let w = 1.0 - u - v;
        (u >= -1e-4 && v >= -1e-4 && w >= -1e-4).then(|| u * a[1] + v * b[1] + w * c[1])
    })
}
