//! Sound kit file choice by `SoundKitEntry.Frequency`.
use super::pick_file;
use game_engine_core::spell_visual::SoundFile;

fn file(fdid: u32, frequency: u32) -> SoundFile {
    SoundFile {
        fdid,
        frequency,
        volume: 1.0,
    }
}

/// Frostbolt's cast kit 85502: four files of frequency 1, one per roll in turn.
#[test]
fn equal_frequencies_cycle_through_every_file() {
    let files = [
        file(1631379, 1),
        file(1631380, 1),
        file(1631381, 1),
        file(1631382, 1),
    ];
    let picked: Vec<u32> = (0..8)
        .map(|roll| pick_file(&files, roll).unwrap().fdid)
        .collect();
    assert_eq!(
        picked,
        [
            1631379, 1631380, 1631381, 1631382, 1631379, 1631380, 1631381, 1631382
        ]
    );
}

/// A file of frequency 3 takes three of every four rolls; frequency 0 never plays.
#[test]
fn frequency_weights_the_choice() {
    let files = [file(10, 3), file(30, 0), file(20, 1)];
    let picked: Vec<u32> = (0..4)
        .map(|roll| pick_file(&files, roll).unwrap().fdid)
        .collect();
    assert_eq!(picked, [10, 10, 10, 20]);
    assert!(pick_file(&[file(30, 0)], 0).is_none());
}
