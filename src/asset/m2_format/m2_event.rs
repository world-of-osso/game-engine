//! M2 animation events (wowdev.wiki/M2 "Events"): named markers a sequence fires at
//! authored times, e.g. `$CSL`/`$CSR`/`$CST` (release pending spell missiles) or `$FSD`
//! (footfall).
//!
//! MD20 header 0x100: events (count + offset) → M2Event[n] (36 bytes each):
//!   0x00: identifier ([u8; 4], "$" + three characters)
//!   0x04: data (u32, passed when the event fires)
//!   0x08: bone (u32)
//!   0x0C: position [f32; 3] relative to the bone
//!   0x18: enabled M2TrackBase (interpolation u16, global sequence i16, timestamps
//!         M2Array<M2Array<u32>>): per sequence, the times the event fires.
//!
//! Event definitions stay in the `.m2` of `.skel` models; their tracks are indexed by the
//! skeleton's sequences and an external sequence's timestamps are in its `.anim` file's
//! `AFM2` chunk (wowdev.wiki/M2 ".anim files").

use super::m2_anim::{SequenceData, read_inner_u32_array};
use crate::asset::read_bytes::{read_m2_array_header, read_u32, read_vec3};

const MD20_EVENTS_COUNT_OFFSET: usize = 0x100;
const EVENT_SIZE: usize = 36;
const EVENT_TRACK_OFFSET: usize = 0x18;

#[derive(Debug, Clone)]
pub struct M2Event {
    pub identifier: [u8; 4],
    pub data: u32,
    pub bone: u32,
    pub position: [f32; 3],
    /// Per sequence: the times (ms) the event fires.
    pub timestamps: Vec<Vec<u32>>,
}

/// The MD20 events; sequence `i`'s timestamps are where `sources[i]` says (in `md20`
/// when absent), and a missing `.anim` leaves that sequence without firings.
pub fn parse_events(md20: &[u8], sources: &[SequenceData<'_>]) -> Result<Vec<M2Event>, String> {
    let (count, offset) = read_m2_array_header(md20, MD20_EVENTS_COUNT_OFFSET)?;
    (0..count)
        .map(|i| {
            let base = offset + i * EVENT_SIZE;
            let identifier = md20
                .get(base..base + 4)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or_else(|| format!("M2 event {i} out of bounds at {base:#x}"))?;
            let track = base + EVENT_TRACK_OFFSET;
            let (sequences, outer) = read_m2_array_header(md20, track + 4)?;
            let timestamps = (0..sequences)
                .map(|sequence| {
                    let inner = outer + sequence * 8;
                    match sources.get(sequence).copied() {
                        None | Some(SequenceData::InFile) => {
                            read_inner_u32_array(md20, md20, inner)
                        }
                        // Like bone tracks, an `.anim` track past its chunk has no keys.
                        Some(SequenceData::External(file)) => Ok(read_inner_u32_array(
                            md20, file, inner,
                        )
                        .unwrap_or_else(|error| {
                            eprintln!("M2 event {i} sequence {sequence}: .anim track: {error}");
                            Vec::new()
                        })),
                        Some(SequenceData::Missing) => Ok(Vec::new()),
                    }
                })
                .collect::<Result<_, String>>()
                .map_err(|error| format!("M2 event {i} timestamps: {error}"))?;
            Ok(M2Event {
                identifier,
                data: read_u32(md20, base + 4)?,
                bone: read_u32(md20, base + 8)?,
                position: read_vec3(md20, base + 12)?,
                timestamps,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../tests/unit/asset/m2_event_tests.rs"]
mod tests;
