use super::*;

#[test]
fn placement_rotation_matches_current_model_rotation_formula() {
    let rot = [17.0, 123.0, -31.0];
    let actual = placement_rotation(rot);
    let expected = Quat::from_euler(
        EulerRot::YZX,
        (rot[1] - 180.0).to_radians(),
        (-rot[0]).to_radians(),
        rot[2].to_radians(),
    );

    let probe = Vec3::new(0.3, -0.4, 0.8);
    let actual_vec = actual * probe;
    let expected_vec = expected * probe;
    assert!(
        actual_vec.abs_diff_eq(expected_vec, 1e-5),
        "rotation mismatch: actual={actual_vec:?} expected={expected_vec:?}"
    );
}

#[test]
fn placement_rotation_zero_matches_current_yaw_correction() {
    let rotation = placement_rotation([0.0, 0.0, 0.0]);
    let probe = Vec3::X;
    let rotated = rotation * probe;
    let expected = -Vec3::X;
    assert!(
        rotated.abs_diff_eq(expected, 1e-5),
        "zero placement rotation should match the current Y-180 mapping: rotated={rotated:?} expected={expected:?}"
    );
}

#[test]
fn load_map_fogs_wdt_reads_warband_companion_file() {
    let fogs = load_map_fogs_wdt("2703").expect("expected 2703_fogs.wdt");

    assert_eq!(fogs.version, 2);
    assert_eq!(fogs.volumes.len(), 1);
    assert_eq!(fogs.volumes[0].model_fdid, 1_728_356);
    assert_eq!(fogs.volumes[0].fog_id, 1_725);
}

#[test]
fn placement_to_bevy_maps_absolute_wow_world_positions_into_loaded_adt_space() {
    let raw = [17282.818, 80.921, 25931.766];
    let actual = placement_to_bevy_absolute(raw);
    let (tile_y, tile_x) = crate::terrain_tile::bevy_to_tile_coords(actual[0], actual[2]);
    assert_eq!((tile_y, tile_x), (32, 48));
    assert!(
        (actual[0] + 8865.1).abs() < 1.0,
        "expected centered Bevy X near player space, got {}",
        actual[0]
    );
    assert!(
        (actual[2] - 216.2).abs() < 1.0,
        "expected centered Bevy Z near player space, got {}",
        actual[2]
    );
}

#[test]
fn placement_to_bevy_falls_back_to_local_wow_coords_when_absolute_result_misses_tile() {
    let raw = [-2982.99, 455.52, 468.06];
    let actual = placement_to_bevy_on_tile(raw, 31, 37);
    assert!(
        (actual[0] + 2982.99).abs() < 0.1,
        "expected local Bevy X near scene camera, got {}",
        actual[0]
    );
    assert!(
        (actual[1] - 455.52).abs() < 0.1,
        "expected local Bevy Y near scene camera, got {}",
        actual[1]
    );
    assert!(
        (actual[2] + 468.06).abs() < 0.1,
        "expected local Bevy Z near scene camera, got {}",
        actual[2]
    );
}

#[test]
fn doodad_transform_lifts_props_to_terrain_height() {
    let data = std::fs::read("data/terrain/azeroth_32_48.adt")
        .expect("expected test ADT data/terrain/azeroth_32_48.adt");
    let adt = crate::asset::adt::load_adt_for_tile(&data, 32, 48).expect("expected ADT to parse");
    let mut heightmap = crate::terrain_heightmap::TerrainHeightmap::default();
    heightmap.insert_tile(32, 48, &adt);

    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let terrain_y = heightmap
        .height_at(bx, bz)
        .expect("expected terrain at sample position");
    let center = 32.0 * TILE_SIZE;
    let doodad = adt_obj::DoodadPlacement {
        name_id: 0,
        unique_id: 0,
        position: [bz + center, terrain_y - 5.0, center - bx],
        rotation: [0.0, 0.0, 0.0],
        scale: 1.0,
        flags: 0,
        fdid: None,
        path: Some("test.m2".to_string()),
    };

    let transform = doodad_transform(&doodad, Some(&heightmap), 32, 48);

    assert!(
        (transform.translation.y - terrain_y).abs() < 0.001,
        "doodad should snap up to terrain, got doodad_y={} terrain_y={terrain_y}",
        transform.translation.y
    );
}

#[test]
fn wmo_vertex_colored_materials_are_unlit() {
    let material = super::terrain_objects_wmo::wmo_standard_material(None, 0, 0, true);
    assert!(material.unlit);
}

#[test]
fn waterfall_backdrop_filter_keeps_only_waterfall_effects() {
    let waterfall = adt_obj::DoodadPlacement {
        name_id: 0,
        unique_id: 0,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: 1.0,
        flags: 0,
        fdid: None,
        path: Some("world/expansion09/doodads/exterior/10xp_waterfall04.m2".to_string()),
    };
    let campfire = adt_obj::DoodadPlacement {
        name_id: 0,
        unique_id: 0,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: 1.0,
        flags: 0,
        fdid: None,
        path: Some("world/expansion09/doodads/centaur/10ct_centaur_campfire01.m2".to_string()),
    };

    assert!(is_waterfall_backdrop_doodad(&waterfall));
    assert!(!is_waterfall_backdrop_doodad(&campfire));
}

#[test]
fn charselect_filter_drops_ground_clutter_but_keeps_props() {
    let clutter = adt_obj::DoodadPlacement {
        name_id: 0,
        unique_id: 0,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: 1.0,
        flags: 0,
        fdid: None,
        path: Some("world/expansion09/doodads/highlands/10hgl_pineneedles_a02.m2".to_string()),
    };
    let prop = adt_obj::DoodadPlacement {
        name_id: 0,
        unique_id: 0,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: 1.0,
        flags: 0,
        fdid: None,
        path: Some("world/expansion09/doodads/centaur/10ct_centaur_campfire01.m2".to_string()),
    };

    assert!(is_charselect_clutter_doodad(&clutter));
    assert!(!is_charselect_clutter_doodad(&prop));
}

#[test]
#[ignore]
fn dump_charselect_nearby_doodads() {
    let adt_path = Path::new("data/terrain/2703_31_37.adt");
    let obj = load_obj0(adt_path).expect("obj0");
    let char_pos = Vec3::new(-2981.8, 452.9, -457.4);
    let camera_pos = Vec3::new(-2980.6, 455.1, -463.3);
    let view_dir = (char_pos - camera_pos).normalize();

    let mut nearest: Vec<_> = obj
        .doodads
        .iter()
        .map(|d| {
            let pos = Vec3::from(placement_to_bevy_on_tile(d.position, 31, 37));
            let to_char = pos.distance(char_pos);
            let to_camera = pos.distance(camera_pos);
            let delta = pos - camera_pos;
            let depth = delta.dot(view_dir);
            let ray_dist = (delta - view_dir * depth).length();
            let fdid = d.fdid.or_else(|| {
                d.path
                    .as_deref()
                    .and_then(game_engine::listfile::lookup_path)
            });
            let model = fdid
                .and_then(game_engine::listfile::lookup_fdid)
                .map(str::to_string)
                .or_else(|| d.path.clone())
                .unwrap_or_else(|| "<unknown>".to_string());
            (
                to_char,
                to_camera,
                depth,
                ray_dist,
                pos,
                fdid,
                d.unique_id,
                model,
            )
        })
        .collect();

    nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("Nearest doodads to charselect character:");
    for (dist_char, dist_cam, depth, ray_dist, pos, fdid, unique_id, model) in
        nearest.iter().take(40)
    {
        println!(
            "  d_char={dist_char:6.1} d_cam={dist_cam:6.1} depth={depth:6.1} ray={ray_dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} {}",
            pos.x, pos.y, pos.z, unique_id, fdid, model
        );
    }

    nearest.retain(|(_, _, depth, ray_dist, ..)| *depth > 0.0 && *ray_dist < 25.0);
    nearest.sort_by(|a, b| a.3.total_cmp(&b.3).then_with(|| a.2.total_cmp(&b.2)));
    println!("\nDoodads near the camera view ray:");
    for (dist_char, dist_cam, depth, ray_dist, pos, fdid, unique_id, model) in
        nearest.into_iter().take(60)
    {
        println!(
            "  ray={ray_dist:6.1} depth={depth:6.1} d_char={dist_char:6.1} d_cam={dist_cam:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} {}",
            pos.x, pos.y, pos.z, unique_id, fdid, model
        );
    }
}

#[test]
#[ignore]
fn dump_charselect_nearby_wmos() {
    let adt_path = Path::new("data/terrain/2703_31_37.adt");
    let obj = load_obj0(adt_path).expect("obj0");
    let char_pos = Vec3::new(-2981.8, 452.9, -457.4);
    let camera_pos = Vec3::new(-2980.6, 455.1, -463.3);
    let view_dir = (char_pos - camera_pos).normalize();

    let mut nearest: Vec<_> = obj
        .wmos
        .iter()
        .map(|w| {
            let pos = Vec3::from(placement_to_bevy_on_tile(w.position, 31, 37));
            let to_char = pos.distance(char_pos);
            let to_camera = pos.distance(camera_pos);
            let delta = pos - camera_pos;
            let depth = delta.dot(view_dir);
            let ray_dist = (delta - view_dir * depth).length();
            let fdid = w.fdid.or_else(|| {
                w.path
                    .as_deref()
                    .and_then(game_engine::listfile::lookup_path)
            });
            let model = fdid
                .and_then(game_engine::listfile::lookup_fdid)
                .map(str::to_string)
                .or_else(|| w.path.clone())
                .unwrap_or_else(|| "<unknown>".to_string());
            (
                to_char,
                to_camera,
                depth,
                ray_dist,
                pos,
                fdid,
                w.unique_id,
                w.rotation,
                model,
            )
        })
        .collect();

    nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("Nearest WMOs to charselect character:");
    for (dist_char, dist_cam, depth, ray_dist, pos, fdid, unique_id, rotation, model) in
        nearest.iter().take(40)
    {
        println!(
            "  d_char={dist_char:6.1} d_cam={dist_cam:6.1} depth={depth:6.1} ray={ray_dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} rot={rotation:?} {}",
            pos.x, pos.y, pos.z, unique_id, fdid, model
        );
    }
}

#[test]
#[ignore]
fn dump_charselect_neighbor_tile_objects() {
    let char_pos = Vec3::new(-2981.8, 452.9, -457.4);
    for (tile_y, tile_x) in [(31, 36), (31, 37)] {
        let adt_path_string = format!("data/terrain/2703_{tile_y}_{tile_x}.adt");
        let adt_path = Path::new(&adt_path_string);
        let Some(obj) = load_obj0(adt_path) else {
            println!("missing obj0 for tile ({tile_y}, {tile_x})");
            continue;
        };
        println!("\nTile ({tile_y}, {tile_x}) doodads near campsite:");
        let mut doodads: Vec<_> = obj
            .doodads
            .iter()
            .filter_map(|d| {
                let pos = Vec3::from(placement_to_bevy_on_tile(d.position, tile_y, tile_x));
                let dist = pos.distance(char_pos);
                if dist > 80.0 {
                    return None;
                }
                let fdid = d.fdid.or_else(|| {
                    d.path
                        .as_deref()
                        .and_then(game_engine::listfile::lookup_path)
                });
                let model = fdid
                    .and_then(game_engine::listfile::lookup_fdid)
                    .map(str::to_string)
                    .or_else(|| d.path.clone())
                    .unwrap_or_else(|| "<unknown>".to_string());
                Some((dist, pos, fdid, d.unique_id, d.rotation, model))
            })
            .collect();
        doodads.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (dist, pos, fdid, uid, rotation, model) in doodads.into_iter().take(80) {
            println!(
                "  d={dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} rot={rotation:?} {}",
                pos.x, pos.y, pos.z, uid, fdid, model
            );
        }

        println!("\nTile ({tile_y}, {tile_x}) WMOs near campsite:");
        let mut wmos: Vec<_> = obj
            .wmos
            .iter()
            .filter_map(|w| {
                let pos = Vec3::from(placement_to_bevy_on_tile(w.position, tile_y, tile_x));
                let dist = pos.distance(char_pos);
                if dist > 200.0 {
                    return None;
                }
                let fdid = w.fdid.or_else(|| {
                    w.path
                        .as_deref()
                        .and_then(game_engine::listfile::lookup_path)
                });
                let model = fdid
                    .and_then(game_engine::listfile::lookup_fdid)
                    .map(str::to_string)
                    .or_else(|| w.path.clone())
                    .unwrap_or_else(|| "<unknown>".to_string());
                Some((dist, pos, fdid, w.unique_id, w.rotation, model))
            })
            .collect();
        wmos.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (dist, pos, fdid, uid, rotation, model) in wmos.into_iter().take(80) {
            println!(
                "  d={dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} rot={rotation:?} {}",
                pos.x, pos.y, pos.z, uid, fdid, model
            );
        }
    }
}

#[test]
#[ignore]
fn compare_wmo_swizzles_against_modf_extents() {
    #[derive(Clone, Copy)]
    struct RawModfEntry {
        unique_id: u32,
        fdid: Option<u32>,
        position: [f32; 3],
        rotation: [f32; 3],
        extents_min: [f32; 3],
        extents_max: [f32; 3],
        scale: f32,
    }

    fn parse_raw_modf_entries(path: &Path) -> Vec<RawModfEntry> {
        let data = std::fs::read(path).expect("obj0");
        let mut modf = None;
        for chunk in crate::asset::adt::ChunkIter::new(&data) {
            let (tag, payload) = chunk.expect("chunk");
            if tag == b"FDOM" {
                modf = Some(payload.to_vec());
                break;
            }
        }
        let payload = modf.expect("modf");
        let count = payload.len() / 64;
        (0..count)
            .map(|i| {
                let base = i * 64;
                let name_id = u32::from_le_bytes(payload[base..base + 4].try_into().unwrap());
                let unique_id = u32::from_le_bytes(payload[base + 4..base + 8].try_into().unwrap());
                let read_f32 = |off: usize| {
                    f32::from_le_bytes(payload[base + off..base + off + 4].try_into().unwrap())
                };
                let position = [read_f32(8), read_f32(12), read_f32(16)];
                let rotation = [read_f32(20), read_f32(24), read_f32(28)];
                let extents_min = [read_f32(32), read_f32(36), read_f32(40)];
                let extents_max = [read_f32(44), read_f32(48), read_f32(52)];
                let flags = u16::from_le_bytes(payload[base + 56..base + 58].try_into().unwrap());
                let scale_raw =
                    u16::from_le_bytes(payload[base + 62..base + 64].try_into().unwrap());
                let scale = if (flags & WMO_SCALE_FLAG) != 0 {
                    scale_raw as f32 / WMO_SCALE_UNIT
                } else {
                    1.0
                };
                let fdid = if (flags & WMO_NAME_IS_FDID_FLAG) != 0 {
                    Some(name_id)
                } else {
                    None
                };
                RawModfEntry {
                    unique_id,
                    fdid,
                    position,
                    rotation,
                    extents_min,
                    extents_max,
                    scale,
                }
            })
            .collect()
    }

    fn sort_bbox(min: [f32; 3], max: [f32; 3]) -> (Vec3, Vec3) {
        (
            Vec3::new(min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2])),
            Vec3::new(min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2])),
        )
    }

    fn wow_bbox_to_bevy(min: [f32; 3], max: [f32; 3]) -> (Vec3, Vec3) {
        let min = placement_to_bevy_absolute(min);
        let max = placement_to_bevy_absolute(max);
        sort_bbox(min, max)
    }

    fn corners(min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 8] {
        [
            [min[0], min[1], min[2]],
            [min[0], min[1], max[2]],
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [max[0], min[1], min[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
        ]
    }

    fn swizzle_current(x: f32, y: f32, z: f32) -> [f32; 3] {
        crate::asset::wmo::wmo_local_to_bevy(x, y, z)
    }

    fn swizzle_like_m2(x: f32, y: f32, z: f32) -> [f32; 3] {
        crate::asset::m2::wow_to_bevy(x, y, z)
    }

    fn fitted_bbox(
        root: &crate::asset::wmo::WmoRootData,
        transform: Transform,
        swizzle: fn(f32, f32, f32) -> [f32; 3],
    ) -> (Vec3, Vec3) {
        let mut mins = Vec3::splat(f32::INFINITY);
        let mut maxs = Vec3::splat(f32::NEG_INFINITY);
        for info in &root.group_infos {
            for corner in corners(info.bbox_min, info.bbox_max) {
                let local = Vec3::from(swizzle(corner[0], corner[1], corner[2]));
                let world = transform.transform_point(local);
                mins = mins.min(world);
                maxs = maxs.max(world);
            }
        }
        (mins, maxs)
    }

    let path = Path::new("data/terrain/2703_31_37_obj0.adt");
    let raw_entries = parse_raw_modf_entries(path);
    for raw in raw_entries
        .into_iter()
        .filter(|entry| matches!(entry.fdid, Some(4214993 | 3803037)))
    {
        let root_fdid = raw.fdid.expect("fdid");
        let root_path = super::terrain_objects_wmo::ensure_wmo_asset(root_fdid).expect("wmo");
        let root_data = std::fs::read(&root_path).expect("wmo root data");
        let root = crate::asset::wmo::load_wmo_root(&root_data).expect("wmo root");
        let pos = Vec3::from(placement_to_bevy_on_tile(raw.position, 31, 37));
        let rotation = placement_rotation(raw.rotation);
        let transform = Transform::from_translation(pos)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(raw.scale));

        let (expected_min, expected_max) = wow_bbox_to_bevy(raw.extents_min, raw.extents_max);
        let (current_min, current_max) = fitted_bbox(&root, transform, swizzle_current);
        let (m2_min, m2_max) = fitted_bbox(&root, transform, swizzle_like_m2);

        let current_err = current_min.distance(expected_min) + current_max.distance(expected_max);
        let m2_err = m2_min.distance(expected_min) + m2_max.distance(expected_max);

        println!(
            "uid={} fdid={} current_err={:.3} m2_err={:.3}\n  expected min={:?} max={:?}\n  current  min={:?} max={:?}\n  m2_like  min={:?} max={:?}",
            raw.unique_id,
            root_fdid,
            current_err,
            m2_err,
            expected_min,
            expected_max,
            current_min,
            current_max,
            m2_min,
            m2_max
        );
    }
}
