use super::*;

#[test]
fn parse_humanmale_hd_attachments() {
    let path = std::path::Path::new("data/models/humanmale_hd.m2");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(path).unwrap();
    // Find MD21 chunk
    let md20 = find_md20(&data).expect("no MD21 chunk");
    assert!(parse_attachments(md20).unwrap().is_empty());
    assert!(parse_attachment_lookup(md20).unwrap().is_empty());

    let skel = std::fs::read("data/models/humanmale_hd.skel").unwrap();
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
    if !path.exists() {
        return;
    }
    let data = std::fs::read(path).unwrap();
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
fn root_attachment_arrays_are_distinct_from_collision_arrays() {
    let mut md20 = vec![0; 0x200];
    md20[..4].copy_from_slice(b"MD20");
    md20[4..8].copy_from_slice(&274_u32.to_le_bytes());
    md20[0xd8..0xdc].copy_from_slice(&1_u32.to_le_bytes());
    md20[0xdc..0xe0].copy_from_slice(&0x140_u32.to_le_bytes());
    md20[0xe0..0xe4].copy_from_slice(&1_u32.to_le_bytes());
    md20[0xe4..0xe8].copy_from_slice(&0x142_u32.to_le_bytes());
    md20[0xf0..0xf4].copy_from_slice(&1_u32.to_le_bytes());
    md20[0xf4..0xf8].copy_from_slice(&0x180_u32.to_le_bytes());
    md20[0xf8..0xfc].copy_from_slice(&2_u32.to_le_bytes());
    md20[0xfc..0x100].copy_from_slice(&0x1a8_u32.to_le_bytes());
    md20[0x180..0x184].copy_from_slice(&19_u32.to_le_bytes());
    md20[0x184..0x186].copy_from_slice(&7_u16.to_le_bytes());
    for (index, coordinate) in [1.0_f32, -2.0, 3.5].into_iter().enumerate() {
        let offset = 0x188 + index * 4;
        md20[offset..offset + 4].copy_from_slice(&coordinate.to_le_bytes());
    }
    md20[0x1a8..0x1aa].copy_from_slice(&(-1_i16).to_le_bytes());
    md20[0x1aa..0x1ac].copy_from_slice(&0_i16.to_le_bytes());
    let attachments = parse_attachments(&md20).unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].id, 19);
    assert_eq!(attachments[0].bone, 7);
    assert_eq!(attachments[0].position, [1.0, -2.0, 3.5]);
    assert_eq!(parse_attachment_lookup(&md20).unwrap(), [-1, 0]);
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
