use game_engine_core::asset::m2_format::m2_attach::*;

#[path = "../required_asset.rs"]
mod required_asset;
use required_asset::require_asset;

#[test]
fn a7_authentic_solar_spell_and_soul_aegis_disable_attached_children() {
    let models = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    for (fdid, sequence) in [(1081566, 2), (971117, 1)] {
        let bytes = std::fs::read(models.join(format!("{fdid}.m2"))).unwrap();
        let attachments = parse_attachments(find_md20(&bytes).unwrap()).unwrap();
        assert!(attachments[0].active_at(0, 0, &[], 0.0));
        assert!(!attachments[0].active_at(sequence, 0, &[], 0.0));
        assert_eq!(attachments[0].activation.sequences[sequence].1, [0]);
    }
}

#[test]
fn parse_humanmale_hd_attachments() {
    let path = std::path::Path::new("data/models/humanmale_hd.m2");
    let data = std::fs::read(require_asset(path)).unwrap();
    // Find MD21 chunk
    let md20 = find_md20(&data).expect("no MD21 chunk");
    assert!(parse_attachments(md20).unwrap().is_empty());
    assert!(parse_attachment_lookup(md20).unwrap().is_empty());

    let skel = std::fs::read(require_asset("data/models/humanmale_hd.skel")).unwrap();
    let ska1 = find_chunk(&skel, b"SKA1").expect("no SKA1 chunk in skeleton");
    let attachments = parse_ska1_attachments(ska1).unwrap();
    let lookup = parse_ska1_attachment_lookup(ska1).unwrap();
    assert_eq!(attachments.len(), 45);
    assert_eq!(lookup.len(), 75);
    assert!(attachments.iter().any(|attachment| attachment.id == 0));
}

#[test]
fn parse_torch_attachments() {
    let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
    let data = std::fs::read(require_asset(path)).unwrap();
    let md20 = find_md20(&data).expect("no MD21 chunk");
    let attachments = parse_attachments(md20).unwrap();
    assert_eq!(
        attachments.iter().map(|a| a.id).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        attachments.iter().map(|a| a.bone).collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
    assert_eq!(parse_attachment_lookup(md20).unwrap(), [-1, 0, 1, 2, 3]);
}

#[test]
fn authored_creation_models_parse_root_attachments_and_lookup() {
    let cases: &[(u32, &[(u32, u16, [f32; 3])], &[i16])] = &[
        (
            623712,
            &[
                (0, 108, [3.5623965, -2.0384471, -0.24659234]),
                (1, 109, [5.7180047, -6.0218797, -0.41022673]),
            ],
            &[0, 1],
        ),
        (
            623714,
            &[
                (0, 83, [3.9399729, -2.0585501, -0.4162232]),
                (1, 84, [4.4563966, -4.28894, -0.2603436]),
            ],
            &[0, 1],
        ),
        (
            623716,
            &[
                (0, 115, [1.8312383, -0.6164761, -0.23094828]),
                (1, 116, [2.2220323, -2.5642111, -0.10935279]),
                (19, 117, [5.4025064, -3.4535482, -0.18255499]),
                (19, 118, [6.489863, -2.0776408, -0.2457679]),
                (19, 119, [6.5616326, -1.4483128, -0.2457679]),
                (19, 120, [4.712396, -4.040183, -0.18255499]),
            ],
            &[
                0, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 5,
            ],
        ),
    ];
    for &(id, expected_attachments, expected_lookup) in cases {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("data/models/{id}.m2"));
        let data =
            std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let md20 = find_md20(&data).expect("no MD21 chunk");
        assert_eq!(
            u32::from_le_bytes(md20[4..8].try_into().unwrap()),
            274,
            "{id}"
        );
        let actual = parse_attachments(md20)
            .unwrap()
            .into_iter()
            .map(|attachment| (attachment.id, attachment.bone, attachment.position))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected_attachments, "{id}");
        assert_eq!(
            parse_attachment_lookup(md20).unwrap(),
            expected_lookup,
            "{id}"
        );
    }
}

fn find_md20(data: &[u8]) -> Option<&[u8]> {
    find_chunk(data, b"MD21")
}

fn find_chunk<'a>(data: &'a [u8], needle: &[u8; 4]) -> Option<&'a [u8]> {
    let mut off = 0;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
        let end = off + 8 + size;
        if end > data.len() {
            break;
        }
        if tag == needle {
            return Some(&data[off + 8..end]);
        }
        off = end;
    }
    None
}
