#[path = "../../rust/src/assets/appearance_pixels.rs"]
mod appearance_pixels;
#[path = "../../rust/src/assets/texture_file.rs"]
mod texture_file;

use std::{collections::HashMap, path::Path};

use appearance_pixels::{compose_replacement_pixels, inactive_npc_texture_types, npc_pass_active};
use game_engine_core::{
    asset::m2_texture,
    blp,
    char_texture_cache::import_char_texture_cache,
    char_texture_data::{CharTextureData, CompositedModelTextures, TextureLayer, TextureLayout},
    char_texture_query_data::{query_char_texture_data, query_model_material_sizes},
    customization_cache::import_customization_cache,
    npc_appearance_data::query_authored_npc_appearance,
    npc_appearance_selection_data::NpcTexturePixels,
};
use rusqlite::{Connection, OpenFlags};

fn open_catalog(path: &Path) -> Connection {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
}

fn load_texture_pixels(data: &Path, fdid: u32) -> NpcTexturePixels {
    let bytes = std::fs::read(data.join(format!("textures/{fdid}.blp"))).unwrap();
    let image = blp::decode_rgba(&bytes).unwrap();
    (image.pixels, image.width, image.height)
}

#[test]
fn ailee_selected_type20_composes_authentic_pixels_on_256_canvas() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let profiles = open_catalog(&data.join("cache/npc_appearance.sqlite"));
    let appearance = query_authored_npc_appearance(&profiles, 136968)
        .unwrap()
        .unwrap();
    assert_eq!((appearance.race, appearance.sex), (95, 1));
    assert_eq!(appearance.baked_texture_fdid, Some(7352105));
    assert!(appearance.choice_ids.contains(&61137));
    let forever = data.join("db2/1.60.1.70205");
    let customization = open_catalog(&import_customization_cache(&forever).unwrap());
    let mut stmt = customization
        .prepare(
            "SELECT e.related_choice_id, m.texture_target_id, t.file_data_id
         FROM elements e JOIN materials m ON m.id=e.material_id
         JOIN texture_fdids t ON t.material_resources_id=m.material_resources_id
         WHERE e.choice_id=?1 ORDER BY e.rowid",
        )
        .unwrap();
    let mut materials = Vec::new();
    for choice in &appearance.choice_ids {
        for row in stmt
            .query_map([choice], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            })
            .unwrap()
        {
            let (related, target, fdid) = row.unwrap();
            if related == 0 || appearance.choice_ids.contains(&related) {
                materials.push((target, fdid));
            }
        }
    }
    assert_eq!(appearance.choice_ids.len(), 14);
    assert!(!materials.iter().any(|(target, _)| *target == 19));
    assert!(materials.contains(&(38, 3613861)));
    let catalog = open_catalog(&import_char_texture_cache(&forever).unwrap());
    let (layers, sections, layouts) = query_char_texture_data(&catalog).unwrap();
    let compositor = CharTextureData::from_parts(layers, sections, layouts)
        .with_material_sizes(query_model_material_sizes(&catalog).unwrap());
    let layout_id = 202;
    let inactive = inactive_npc_texture_types(&compositor, &materials, layout_id);
    assert!(inactive.contains(&7));
    assert!(!inactive.contains(&20));
    let layout = compositor.layout(layout_id).unwrap();
    let default = m2_texture::default_fdid_for_type(
        1,
        layout.width == 2048 && layout.height == 1024,
        &[0, 0, 0],
    )
    .unwrap();
    let decoded: HashMap<_, _> = materials
        .iter()
        .map(|&(_, fdid)| fdid)
        .chain([default])
        .map(|fdid| (fdid, load_texture_pixels(&data, fdid)))
        .collect();
    let composed = compositor
        .composite_model_textures_with(&materials, &[], layout_id, default, |fdid| {
            decoded.get(&fdid).cloned()
        })
        .unwrap();
    let baked = load_texture_pixels(&data, 7352105);
    let expected_body = baked.clone();
    let expected_type6 = game_engine_core::npc_appearance_selection_data::select_npc_type6_texture(
        compositor.declares_hair(&materials, layout_id),
        composed.hair.clone(),
        composed.head.clone(),
    )
    .unwrap()
    .unwrap();
    let expected_type19 = compositor
        .composite_texture_type(&materials, layout_id, 19, |fdid| {
            decoded.get(&fdid).cloned()
        })
        .unwrap();
    let textures = compose_replacement_pixels(
        &compositor,
        &materials,
        layout_id,
        composed,
        Some(baked),
        &decoded,
        None,
    )
    .unwrap();
    assert_eq!(
        textures.get(&1),
        Some(&expected_body),
        "authored bake retained"
    );
    assert_eq!(
        textures.get(&6),
        Some(&expected_type6),
        "hair/head policy retained"
    );
    assert_eq!(
        textures.get(&19),
        Some(&expected_type19),
        "all eye layers retained"
    );
    let type20 = textures
        .get(&20)
        .expect("selected authored NPC type20 omitted");
    assert_eq!((type20.1, type20.2), (256, 256));
    assert_eq!(type20.0.len(), 256 * 256 * 4);
    let source = decoded.get(&3613861).unwrap();
    assert_eq!((source.1, source.2), (256, 256));
    assert_eq!(
        type20, source,
        "authored opaque full-canvas layer preserves decoded pixels"
    );
    assert!(type20.0.chunks_exact(4).any(|pixel| pixel[3] != 0));
    assert!(!npc_pass_active(&[7], &[None], &inactive, &textures, true, &[]).unwrap());
    assert!(npc_pass_active(&[20], &[None], &inactive, &textures, true, &[]).unwrap());
}

fn fixture() -> CharTextureData {
    CharTextureData::from_parts(
        vec![
            TextureLayer {
                texture_type: 7,
                layer: 20,
                blend_mode: 1,
                section_bitmask: -1,
                target_id: 19,
                layout_id: 202,
            },
            TextureLayer {
                texture_type: 20,
                layer: 18,
                blend_mode: 1,
                section_bitmask: -1,
                target_id: 38,
                layout_id: 202,
            },
            TextureLayer {
                texture_type: 6,
                layer: 1,
                blend_mode: 1,
                section_bitmask: -1,
                target_id: 10,
                layout_id: 202,
            },
            TextureLayer {
                texture_type: 19,
                layer: 1,
                blend_mode: 1,
                section_bitmask: -1,
                target_id: 25,
                layout_id: 202,
            },
        ],
        HashMap::new(),
        HashMap::from([(
            202,
            TextureLayout {
                width: 1,
                height: 1,
            },
        )]),
    )
    .with_material_sizes(HashMap::from([
        ((202, 7), (512, 256)),
        ((202, 20), (256, 256)),
        ((202, 6), (1, 1)),
        ((202, 19), (1, 1)),
    ]))
}

fn composed() -> CompositedModelTextures {
    CompositedModelTextures {
        body: (vec![10, 20, 30, 255], 1, 1),
        head: Some((vec![40, 50, 60, 255], 1, 1)),
        hair: Some((vec![70, 80, 90, 255], 1, 1)),
    }
}

#[test]
fn unselected_separate_layers_do_not_publish_textures_or_replace_body_and_head() {
    let composed = composed();
    let body = composed.body.clone();
    let head = composed.head.clone().unwrap();
    let textures =
        compose_replacement_pixels(&fixture(), &[], 202, composed, None, &HashMap::new(), None)
            .unwrap();
    assert_eq!(textures.len(), 2);
    assert_eq!(textures.get(&1), Some(&body));
    assert_eq!(textures.get(&6), Some(&head));
    assert!(!textures.contains_key(&19));
    assert!(!textures.contains_key(&20));
}

#[test]
fn source_absence_omits_multi_slot_pass_only_after_required_slots_validate() {
    let inactive = inactive_npc_texture_types(&fixture(), &[(38, 3613861)], 202);
    assert!(inactive.contains(&7));
    assert!(!inactive.contains(&1));
    assert!(!inactive.contains(&6));
    assert!(!inactive.contains(&20));
    assert!(!inactive.contains(&99));
    let textures = HashMap::from([(1, ()), (6, ()), (20, ())]);
    assert!(!npc_pass_active(&[20, 7], &[None, None], &inactive, &textures, true, &[]).unwrap());
    assert!(
        !npc_pass_active(&[7, 0], &[None, Some(123)], &inactive, &textures, true, &[]).unwrap()
    );
    assert!(npc_pass_active(&[20], &[None], &inactive, &textures, true, &[]).unwrap());
    for kind in [0, 1, 6, 20, 99] {
        let missing = HashMap::<u32, ()>::new();
        assert!(
            npc_pass_active(&[7, kind], &[None, None], &inactive, &missing, true, &[]).is_err()
        );
        assert!(
            npc_pass_active(&[kind, 7], &[None, None], &inactive, &missing, true, &[]).is_err()
        );
    }
    assert!(
        npc_pass_active(
            &[0, 99],
            &[Some(123), Some(456)],
            &inactive,
            &textures,
            true,
            &[]
        )
        .unwrap()
    );
    for kind in [1, 6] {
        assert!(
            npc_pass_active(
                &[kind],
                &[Some(123)],
                &inactive,
                &HashMap::<u32, ()>::new(),
                true,
                &[]
            )
            .is_err()
        );
    }
    let unknown_layout = inactive_npc_texture_types(&fixture(), &[], 999);
    assert!(unknown_layout.is_empty());
    assert!(npc_pass_active(&[7], &[None], &unknown_layout, &textures, true, &[]).is_err());
}

#[test]
fn hidden_unbound_cape_keeps_batch_but_does_not_draw() {
    // Authentic Ailee skin unit29: submesh68, mesh part1507, type2, no cape.
    let visible =
        game_engine_core::npc_appearance_selection_data::npc_geoset_visible(1507, &[(15, 0)], &[]);
    assert!(!visible);
    let inactive = inactive_npc_texture_types(&fixture(), &[(38, 3613861)], 202);
    assert_eq!(
        npc_pass_active(
            &[2],
            &[None],
            &inactive,
            &HashMap::<u32, ()>::new(),
            visible,
            &[]
        ),
        Ok(true),
        "hidden optional cape retains its Batch node, not a drawable texture requirement"
    );
}

#[test]
fn visible_unbound_cape_remains_explicit_error() {
    let visible = game_engine_core::npc_appearance_selection_data::npc_geoset_visible(
        1507,
        &[(15, 0)],
        &[(15, 7)],
    );
    assert!(visible);
    let inactive = inactive_npc_texture_types(&fixture(), &[(38, 3613861)], 202);
    assert_eq!(
        npc_pass_active(
            &[2],
            &[None],
            &inactive,
            &HashMap::<u32, ()>::new(),
            visible,
            &[]
        ),
        Err("missing required NPC pass texture type 2".to_owned())
    );
}

#[test]
fn real_missing_blp_errors_before_source_unselected_omission() {
    let dir = std::env::temp_dir().join(format!("npc-missing-blp-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let fdid = 3613861;
    let mut missing = Vec::new();
    assert_eq!(
        texture_file::read_texture_file(fdid, &dir, |id| missing.push(id)).unwrap(),
        None
    );
    assert_eq!(missing, vec![fdid]);
    let inactive = inactive_npc_texture_types(&fixture(), &[(38, fdid)], 202);
    let textures = HashMap::<u32, ()>::new();
    for visible in [true, false] {
        for kind in [0, 2, 20, 99] {
            for (types, sources) in [
                ([7, kind], [None, Some(fdid)]),
                ([kind, 7], [Some(fdid), None]),
            ] {
                assert_eq!(
                    npc_pass_active(&types, &sources, &inactive, &textures, visible, &missing),
                    Err(format!(
                        "missing required NPC pass texture type {kind} FDID {fdid}"
                    )),
                    "actual NotFound must fail before omission, regardless of slot order or visibility"
                );
            }
        }
    }
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
fn real_file_receipts_distinguish_inactive_and_required_slots() {
    let dir = std::env::temp_dir().join(format!("npc-file-receipts-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let inactive = inactive_npc_texture_types(&fixture(), &[(38, 3613861)], 202);
    let mut missing = Vec::new();
    let fdid = 3613861;
    texture_file::read_texture_file(fdid, &dir, |id| missing.push(id)).unwrap();
    let textures = HashMap::from([(20, ())]);
    assert_eq!(
        npc_pass_active(
            &[7, 20],
            &[Some(fdid), None],
            &inactive,
            &textures,
            true,
            &missing
        ),
        Ok(false)
    );
    assert!(
        npc_pass_active(
            &[7, 0],
            &[Some(fdid), Some(fdid)],
            &inactive,
            &textures,
            true,
            &missing
        )
        .is_err()
    );
    std::fs::write(
        dir.join(format!("{fdid}.blp")),
        b"owned read-boundary bytes",
    )
    .unwrap();
    missing.clear();
    assert_eq!(
        texture_file::read_texture_file(fdid, &dir, |id| missing.push(id)).unwrap(),
        Some(b"owned read-boundary bytes".to_vec())
    );
    assert!(missing.is_empty());
    assert_eq!(
        npc_pass_active(
            &[7, 0],
            &[None, Some(fdid)],
            &inactive,
            &textures,
            true,
            &missing
        ),
        Ok(false)
    );
    assert_eq!(
        npc_pass_active(&[0], &[Some(fdid)], &inactive, &textures, true, &missing),
        Ok(true)
    );
    // These bytes test only the shared filesystem boundary, not BLP decoding.
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn texture_read_error_is_not_an_absence_receipt() {
    let dir = std::env::temp_dir().join(format!("npc-read-error-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::create_dir(dir.join("3613861.blp")).unwrap();
    let mut missing = Vec::new();
    let error = texture_file::read_texture_file(3613861, &dir, |id| missing.push(id)).unwrap_err();
    assert!(error.starts_with("Cannot read texture 3613861:"));
    assert!(missing.is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn hidden_body_and_hair_still_require_replacements() {
    let inactive = inactive_npc_texture_types(&fixture(), &[], 202);
    for kind in [0, 1, 6] {
        assert!(
            npc_pass_active(
                &[kind],
                &[None],
                &inactive,
                &HashMap::<u32, ()>::new(),
                false,
                &[]
            )
            .is_err()
        );
    }
}

#[test]
fn selected_missing_pixels_error_instead_of_publishing_blank_type20() {
    let result = compose_replacement_pixels(
        &fixture(),
        &[(38, 3613861)],
        202,
        composed(),
        None,
        &HashMap::new(),
        None,
    );
    assert_eq!(
        result,
        Err("missing selected NPC texture FDID 3613861".to_owned())
    );
}

#[test]
fn selected_type7_without_canvas_is_not_source_absence() {
    let data = fixture().with_material_sizes(HashMap::new());
    let materials = [(19, 777)];
    assert!(!inactive_npc_texture_types(&data, &materials, 202).contains(&7));
    let decoded = HashMap::from([(777, (vec![1, 2, 3, 255], 1, 1))]);
    assert_eq!(
        compose_replacement_pixels(&data, &materials, 202, composed(), None, &decoded, None),
        Err("cannot composite selected NPC texture type 7 for layout 202".to_owned()),
    );
}

#[test]
fn selected_missing_unknown_target_remains_error() {
    assert_eq!(
        compose_replacement_pixels(
            &fixture(),
            &[(999, 777)],
            202,
            composed(),
            None,
            &HashMap::new(),
            None
        ),
        Err("missing selected NPC texture FDID 777".to_owned()),
    );
    assert!(!inactive_npc_texture_types(&fixture(), &[(19, 0)], 202).contains(&7));
}

#[test]
fn separate_canvases_do_not_overwrite_declared_hair_and_missing_hair_errors() {
    let mut composed = composed();
    let hair = composed.hair.clone().unwrap();
    let decoded = HashMap::from([(100, (vec![1, 2, 3, 255], 1, 1))]);
    let textures = compose_replacement_pixels(
        &fixture(),
        &[(10, 100)],
        202,
        composed.clone(),
        None,
        &decoded,
        None,
    )
    .unwrap();
    assert_eq!(textures.get(&6), Some(&hair));
    composed.hair = None;
    assert_eq!(
        compose_replacement_pixels(
            &fixture(),
            &[(10, 100)],
            202,
            composed,
            None,
            &decoded,
            None
        ),
        Err("declared NPC hair target 10 did not produce a texture".to_owned()),
    );
}
