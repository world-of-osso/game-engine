use super::load_adt_raw;

fn assert_load_adt_fails(data: &[u8], expected_substr: &str) {
    match load_adt_raw(data) {
        Ok(_) => panic!("expected load_adt_raw to fail"),
        Err(err) => assert!(
            err.contains(expected_substr),
            "expected error containing '{expected_substr}', got: {err}"
        ),
    }
}

fn assert_load_adt_errs(data: &[u8]) {
    assert!(load_adt_raw(data).is_err(), "expected load_adt_raw to fail");
}

#[test]
fn load_adt_raw_empty_data_fails() {
    assert_load_adt_fails(&[], "MCNK");
}

#[test]
fn load_adt_raw_truncated_chunk_header() {
    assert_load_adt_fails(b"KNCM", "MCNK");
}

#[test]
fn load_adt_raw_truncated_chunk_payload() {
    let mut data = Vec::new();
    data.extend_from_slice(b"KNCM");
    data.extend_from_slice(&1000u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 4]);
    assert_load_adt_fails(&data, "truncated");
}

#[test]
fn load_adt_raw_zero_length_mcnk() {
    let mut data = Vec::new();
    data.extend_from_slice(b"KNCM");
    data.extend_from_slice(&0u32.to_le_bytes());
    assert_load_adt_errs(&data);
}

#[test]
fn load_adt_raw_mcnk_too_small_for_header() {
    let mut data = Vec::new();
    data.extend_from_slice(b"KNCM");
    data.extend_from_slice(&64u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 64]);
    assert_load_adt_errs(&data);
}

#[test]
fn chunk_iter_skips_unknown_chunks() {
    let mut data = Vec::new();
    data.extend_from_slice(b"UNKN");
    data.extend_from_slice(&4u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 4]);
    assert_load_adt_fails(&data, "MCNK");
}

#[test]
fn chunk_iter_handles_back_to_back_chunks() {
    let mut data = Vec::new();
    data.extend_from_slice(b"AAA1");
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 2]);
    data.extend_from_slice(b"BBB2");
    data.extend_from_slice(&0u32.to_le_bytes());
    assert_load_adt_fails(&data, "MCNK");
}
