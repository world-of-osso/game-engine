use super::*;

fn read_lod(fdid: u32) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../data/terrain/{fdid}.adt"));
    std::fs::read(path).unwrap()
}

#[test]
fn neltharus_outer_lod_maps_expand_the_authored_right_and_bottom_borders() {
    for (fdid, right, bottom) in [
        (4334712, true, false),
        (4335067, false, true),
        (4335112, true, true),
    ] {
        let maps = parse_lod_liquid_maps(&read_lod(fdid)).unwrap().unwrap();
        assert_eq!(maps.depth.len(), MAP_TEXELS);
        assert_eq!(maps.coverage.len(), MAP_TEXELS);
        for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                let expected = if (right && x == 127) || (bottom && y == 127) {
                    255
                } else {
                    0
                };
                let texel = y * MAP_SIDE + x;
                assert_eq!(maps.depth[texel], expected, "depth {fdid} ({x},{y})");
                assert_eq!(maps.coverage[texel], expected, "coverage {fdid} ({x},{y})");
            }
        }
    }
}

#[test]
fn forge_pool_lod_has_no_maps_to_apply_to_wmo_liquid() {
    assert_eq!(parse_lod_liquid_maps(&read_lod(4334887)).unwrap(), None);
    assert_eq!(parse_lod_liquid_maps(&read_lod(4334837)).unwrap(), None);
}

#[test]
fn decoded_maps_and_source_survive_repacking_mesh_arrays_and_dropping_container() {
    let (maps, source) = {
        let mut container = read_lod(4334712);
        let mut mesh = crate::adt::parse_lod(&container).unwrap();
        let source = mesh.liquid_directory.take().unwrap().raw;
        let maps = decode_directory(&source).unwrap().unwrap();
        mesh.heights.clear();
        mesh.indices.clear();
        container.fill(0);
        (maps, source)
    };
    assert_eq!(maps.source, source);
    assert_eq!(maps.depth[127], 255);
    assert_eq!(maps.coverage[127], 255);
    assert_eq!(maps.coverage[126], 0);
}

#[test]
fn zero_tokens_are_no_ops_and_literals_preserve_all_byte_values() {
    // literal0, repeat0(value77), literal4, repeat2. No terminator needed.
    let encoded = [0, 0, 0x80, 0, 77, 0, 4, 0, 40, 128, 255, 0x80, 2, 19];
    assert_eq!(decode_rle(&encoded, 6).unwrap(), [0, 40, 128, 255, 19, 19]);
}

#[test]
fn corrupt_runs_and_short_literals_fail_explicitly() {
    for encoded in [&[0x80][..], &[0x80, 1], &[0, 2, 1], &[0x80, 3, 7]] {
        assert!(decode_rle(encoded, 2).unwrap_err().contains("MLLD RLE"));
    }
}

#[test]
fn raw_depth_is_copied_without_changing_byte_units() {
    let source: Vec<_> = (0..MAP_TEXELS).map(|i| (i % 256) as u8).collect();
    assert_eq!(
        decode_depth(&source, HAS_MAPS | COMPRESSED_ALPHA).unwrap(),
        source
    );
    assert!(
        decode_depth(&source[..10], HAS_MAPS)
            .unwrap_err()
            .contains("raw depth")
    );
}
