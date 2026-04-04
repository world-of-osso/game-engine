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
    let attachments = parse_attachments(md20).unwrap();
    assert!(!attachments.is_empty(), "HD human should have attachments");

    // Should have a right hand attachment (id=0)
    let right_hand = attachments.iter().find(|a| a.id == 0);
    assert!(
        right_hand.is_some(),
        "Should have right hand attachment (id=0)"
    );
    let rh = right_hand.unwrap();
    assert!(rh.bone < 200, "Bone index should be reasonable");

    let lookup = parse_attachment_lookup(md20).unwrap();
    assert!(!lookup.is_empty(), "Should have attachment lookup");
    println!(
        "humanmale_hd MD21 attachments ids={:?} lookup11={:?} lookup20={:?}",
        attachments.iter().map(|a| a.id).collect::<Vec<_>>(),
        lookup.get(11),
        lookup.get(20)
    );
    if let Some(ska1) = find_chunk(&data, b"SKA1") {
        let ska1_attachments = parse_ska1_attachments(ska1).unwrap();
        let ska1_lookup = parse_ska1_attachment_lookup(ska1).unwrap();
        println!(
            "humanmale_hd SKA1 attachments ids={:?} lookup11={:?} lookup20={:?}",
            ska1_attachments.iter().map(|a| a.id).collect::<Vec<_>>(),
            ska1_lookup.get(11),
            ska1_lookup.get(20)
        );
    }
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
    // Item models may or may not have attachments — just ensure no crash
    let lookup = parse_attachment_lookup(md20).unwrap();
    let _ = (attachments, lookup);
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
