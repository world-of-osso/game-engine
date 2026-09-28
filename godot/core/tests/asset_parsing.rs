use game_engine_core::{adt, blp, m2, wdt, wmo};
use std::path::PathBuf;

fn fixture(relative: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(relative);
    std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn m2_skin_extracts_mesh_material_bones_and_animation() {
    let model = m2::parse_model(&fixture("models/boar.m2"), &fixture("models/boar00.skin"))
        .expect("cached boar model + skin");
    assert!(!model.vertices.is_empty());
    assert!(!model.indices.is_empty());
    assert!(!model.submeshes.is_empty());
    assert!(!model.materials.is_empty());
    assert!(!model.bones.is_empty());
    assert!(!model.sequences.is_empty());
    assert_eq!(model.vertices[0].position.len(), 3);
    assert!(
        model
            .indices
            .iter()
            .all(|&i| usize::from(i) < model.vertices.len())
    );
}

#[test]
fn m2_and_skin_errors_are_not_silently_replaced() {
    assert!(
        m2::parse_model(b"broken", b"SKIN")
            .err()
            .expect("invalid model")
            .contains("MD21")
    );
    let model = fixture("models/boar.m2");
    assert!(
        m2::parse_model(&model, b"invalid")
            .err()
            .expect("invalid skin")
            .contains("skin")
    );
}

#[test]
fn adt_companions_return_authored_data() {
    let root = adt::parse_root(&fixture("terrain/azeroth_32_48.adt")).expect("root ADT");
    let tex = adt::parse_tex(
        &fixture("terrain/azeroth_32_48_tex0.adt"),
        wdt::MphdFlags { raw: 0x4 },
        &root,
    )
    .expect("texture ADT");
    let obj = adt::parse_obj(&fixture("terrain/azeroth_32_48_obj0.adt")).expect("object ADT");
    assert_eq!(root.chunks.len(), 256);
    assert_eq!(tex.chunk_layers.len(), 256);
    assert!(!obj.doodads.is_empty() || !obj.wmos.is_empty());
    assert!(adt::parse_root(b"bad").is_err());
    assert!(adt::parse_tex(b"bad", wdt::MphdFlags::default(), &root).is_err());
}

#[test]
fn wmo_root_and_group_parse_with_errors() {
    let root = wmo::parse_root(&fixture("models/108121.wmo")).expect("cached WMO root");
    assert!(root.n_groups > 0);
    let group = wmo::parse_group(&fixture("models/108122.wmo")).expect("cached WMO group");
    assert!(!group.geometry.vertices.is_empty());
    assert!(wmo::parse_root(b"bad").is_err());
    assert!(wmo::parse_group(b"bad").is_err());
}

#[test]
fn m2_external_skeleton_preserves_authored_bones_and_tracks() {
    let model = fixture("models/humanmale_hd.m2");
    let skin = fixture("models/humanmale_hd00.skin");
    assert!(
        m2::parse_model(&model, &skin)
            .err()
            .expect("needs skeleton")
            .contains("SKID")
    );
    let parsed = m2::parse_model_with_skeleton(
        &model,
        &skin,
        Some(&fixture("models/humanmale_hd.skel")),
        |_| None,
    )
    .expect("HD skeleton");
    assert!(parsed.bones.len() > 100);
    assert_eq!(parsed.bone_tracks.len(), parsed.bones.len());
    assert!(!parsed.sequences.is_empty());
    assert!(m2::parse_model_with_skeleton(&model, &skin, Some(b"bad"), |_| None).is_err());
}

#[test]
fn wdt_flag_parsing_uses_authored_header() {
    let flags = wdt::parse_wdt_mphd_flags(&fixture("terrain/775971.wdt")).expect("WDT MPHD");
    assert_eq!(flags.height_texturing(), flags.raw & 0x80 != 0);
    assert!(wdt::parse_wdt_mphd_flags(b"bad").is_err());
}

#[test]
fn blp_returns_rgba_and_rejects_invalid_bytes() {
    let decoded = blp::decode_rgba(&fixture("textures/boarskinblue.blp")).expect("boar BLP");
    assert!(decoded.width > 0 && decoded.height > 0);
    assert_eq!(
        decoded.pixels.len(),
        (decoded.width * decoded.height * 4) as usize
    );
    assert!(blp::decode_rgba(b"bad").is_err());
}
