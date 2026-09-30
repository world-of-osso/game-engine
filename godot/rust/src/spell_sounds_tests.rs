//! Sound kit file choice by `SoundKitEntry.Frequency`, and where a late file starts.
use super::{late_offset, pick_file};
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

/// A 1.2 s one-shot whose file arrives 0.3 s late plays from 0.3 s; arriving at or after
/// 1.2 s it would already have ended and does not play. On time it plays from 0.
#[test]
fn a_late_one_shot_starts_where_it_would_be_or_not_at_all() {
    assert_eq!(late_offset(0.0, 1.2, false), Some(0.0));
    assert_eq!(late_offset(0.3, 1.2, false), Some(0.3));
    assert_eq!(late_offset(1.2, 1.2, false), None);
    assert_eq!(late_offset(4.0, 1.2, false), None);
}

/// A 2 s precast loop arriving 5 s late starts 1 s into the loop, like one that had
/// looped since the kit started.
#[test]
fn a_late_loop_starts_at_its_wrapped_position() {
    let offset = late_offset(5.0, 2.0, true).unwrap();
    assert!((offset - 1.0).abs() < 1e-5);
    assert_eq!(late_offset(0.0, 2.0, true), Some(0.0));
}
