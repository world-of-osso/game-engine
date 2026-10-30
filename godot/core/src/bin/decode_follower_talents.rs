//! Offline conversion of follower SimC loadout strings using the client talent importer.
use std::io::{self, Read};
use std::path::Path;

use game_engine_core::talent_data::{
    load_talent_page, load_trait_rules, loadout_string::decode_total_ranks, rules::TraitContext,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Profile {
    spec_id: u32,
    class_id: u32,
    talents: String,
}

#[derive(Serialize)]
struct ResultRow {
    spec_id: u32,
    entries: Option<Vec<(u32, u32)>>,
    error: Option<String>,
}

fn decode_profile(data: &Path, profile: &Profile) -> Result<Vec<(u32, u32)>, String> {
    let page = load_talent_page(data, profile.class_id, profile.spec_id)?;
    let tree = load_trait_rules(data, page.tree_id)?;
    let context = TraitContext {
        spec_id: profile.spec_id,
        level: 90,
    };
    let entries = decode_total_ranks(&tree, context, &profile.talents)
        .map_err(|error| format!("loadout decode: {error:?}"))?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let rank = entry.total();
            (rank > 0).then_some((entry.entry_id, rank))
        })
        .collect())
}

fn run() -> Result<(), String> {
    let data = std::env::args_os()
        .nth(1)
        .ok_or("expected DB2 directory argument")?;
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("read profiles: {error}"))?;
    let profiles: Vec<Profile> =
        serde_json::from_str(&input).map_err(|error| format!("parse profiles: {error}"))?;
    let rows: Vec<_> = profiles
        .iter()
        .map(|profile| match decode_profile(Path::new(&data), profile) {
            Ok(entries) => ResultRow {
                spec_id: profile.spec_id,
                entries: Some(entries),
                error: None,
            },
            Err(error) => ResultRow {
                spec_id: profile.spec_id,
                entries: None,
                error: Some(error),
            },
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string(&rows).map_err(|error| format!("encode results: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
