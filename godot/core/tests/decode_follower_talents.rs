use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use game_engine_core::talent_data::{
    load_talent_page, load_trait_rules,
    loadout_string::encode,
    rules::{TraitContext, build_config},
};

#[test]
fn cli_decodes_real_loadout_and_reports_bad_input_per_profile() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let page = load_talent_page(&data, 8, 62).unwrap();
    let tree = load_trait_rules(&data, page.tree_id).unwrap();
    let context = TraitContext {
        spec_id: 62,
        level: 90,
    };
    let entries = build_config(&tree, context, &[]).unwrap();
    let text = encode(&tree, context.spec_id, &entries).unwrap();
    let input = serde_json::json!([
        {"spec_id": 62, "class_id": 8, "talents": text},
        {"spec_id": 62, "class_id": 8, "talents": "invalid!"}
    ]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_decode_follower_talents"))
        .arg(&data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| {
            let rank = entry.total();
            (rank > 0).then_some(vec![entry.entry_id, rank])
        })
        .collect();
    assert_eq!(actual[0]["entries"], serde_json::json!(expected));
    assert!(actual[0]["error"].is_null());
    assert!(actual[1]["entries"].is_null());
    assert!(actual[1]["error"].as_str().unwrap().contains("BadString"));
}

#[test]
fn cli_subtracts_native_grant_from_hunter_profile_total_ranks() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let input = serde_json::json!([{
        "spec_id": 253, "class_id": 3,
        "talents": "C0PAAAAAAAAAAAAAAAAAAAAAAAMmxwCsAzwwMsBAgZYMzyMDzYmxMMzYMzwMjZMziZmxMmBjpZAAAAAzAAAwYmZAmZjxGMLgtBgBA"
    }]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_decode_follower_talents"))
        .arg(&data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(actual[0]["error"].is_null(), "{actual}");
    assert!(
        actual[0]["entries"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!([117588, 1]))
    );
}
