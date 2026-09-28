//! Authored weighted variation lists. Links enumerate candidates, never playback order.

use super::m2_anim::M2AnimSequence;

/// The variations of one animation ID, linked from its base variation by `variation_next`.
///
/// A weighted zero-duration candidate plays for no time. WebWowViewerCpp's
/// `AnimationManager::animate` (wowViewerLib/src/engine/managers/animationManager.cpp:838-850
/// rolls by `frequency`, :896-921 switches at the end of the current record) makes it
/// current, and the next update rolls again because its time already reaches its
/// duration. Rolling until a positive-duration candidate wins has the same outcome as one
/// roll over the positive-duration candidates' weights, which is what `choose` draws.
/// FDID 588287 `pa_redbird_stand.m2` authors Stand variation 1 this way (0 ms, 30583/32767).
pub struct VariationFamily {
    /// Linked candidates in authored order.
    linked: usize,
    /// Weighted positive-duration candidates with their weights: where a loop boundary lands.
    playable: Vec<(usize, u32)>,
    total: u32,
    shortest_ms: u32,
}

impl VariationFamily {
    pub fn read(sequences: &[M2AnimSequence], current: usize) -> Result<Self, String> {
        let id = sequences
            .get(current)
            .ok_or_else(|| format!("M2 sequence index {current} is out of range"))?
            .id;
        let base = sequences
            .iter()
            .position(|sequence| sequence.id == id && sequence.variation_id == 0)
            .ok_or_else(|| format!("M2 animation {id} lacks base variation"))?;
        let mut family = Self {
            linked: 0,
            playable: Vec::new(),
            total: 0,
            shortest_ms: u32::MAX,
        };
        let mut seen = std::collections::HashSet::new();
        let mut next = Some(base);
        while let Some(index) = next {
            if !seen.insert(index) {
                return Err(format!("M2 animation {id} has cyclic variations"));
            }
            let sequence = sequences
                .get(index)
                .ok_or_else(|| format!("M2 animation {id} links absent variation {index}"))?;
            family.append(index, sequence, id)?;
            next = match sequence.variation_next {
                -1 => None,
                index if index >= 0 => Some(index as usize),
                index => {
                    return Err(format!(
                        "M2 animation {id} has invalid variation link {index}"
                    ));
                }
            };
        }
        if family.linked > 1 && family.total == 0 {
            return Err(format!(
                "M2 animation {id} has {} variations but none is weighted with a positive duration",
                family.linked
            ));
        }
        Ok(family)
    }

    fn append(&mut self, index: usize, sequence: &M2AnimSequence, id: u16) -> Result<(), String> {
        if sequence.id != id {
            return Err(format!(
                "M2 animation {id} links other animation {} at sequence {index}",
                sequence.id
            ));
        }
        let weight = u32::try_from(sequence.frequency).map_err(|_| {
            format!(
                "M2 animation {id} variation {} (sequence {index}) has negative weight {}",
                sequence.variation_id, sequence.frequency
            )
        })?;
        self.linked += 1;
        if weight == 0 || sequence.duration == 0 {
            return Ok(());
        }
        self.total = self
            .total
            .checked_add(weight)
            .ok_or_else(|| format!("M2 animation {id} variation weight overflow"))?;
        self.shortest_ms = self.shortest_ms.min(sequence.duration);
        self.playable.push((index, weight));
        Ok(())
    }

    /// Only the current sequence is linked: it loops without drawing.
    pub fn is_single(&self) -> bool {
        self.linked == 1
    }

    /// Reject catch-up that would cross 4096 or more variation boundaries: exact random
    /// selection is sequential, so it cannot skip draws or monopolize a frame.
    pub fn validate_elapsed(&self, elapsed_ms: f64) -> Result<(), String> {
        if elapsed_ms / f64::from(self.shortest_ms) >= 4096.0 {
            return Err("M2 elapsed animation requires 4096 or more variation boundaries".into());
        }
        Ok(())
    }

    /// The sequence a loop boundary continues with; `sample(total)` must return below `total`.
    pub fn choose(&self, sample: &mut impl FnMut(u32) -> u32) -> Result<usize, String> {
        if let [(index, _)] = self.playable[..] {
            return Ok(index);
        }
        self.choose_roll(sample(self.total))
    }

    pub fn choose_roll(&self, mut roll: u32) -> Result<usize, String> {
        if roll >= self.total {
            return Err(format!("M2 variation roll {roll} exceeds {}", self.total));
        }
        for &(index, weight) in &self.playable {
            if roll < weight {
                return Ok(index);
            }
            roll -= weight;
        }
        Err("M2 variation weights do not cover roll".into())
    }
}
