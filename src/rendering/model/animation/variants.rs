//! Authored weighted variation lists. Links enumerate candidates, never playback order.

use super::*;

#[derive(Component, Default)]
pub(crate) struct VariantRandom {
    state: Option<u64>,
}

impl VariantRandom {
    pub(crate) fn sample(&mut self, owner: Entity, upper: u32) -> u32 {
        let state = self.state.get_or_insert(owner.to_bits());
        // SplitMix64 gives each entity its own deterministic stream, without a global lock.
        // Rejection avoids modulo bias for authored weight totals such as 32767.
        let threshold = upper.wrapping_neg() % upper;
        loop {
            *state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = *state;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            let value = (value ^ (value >> 31)) as u32;
            if value >= threshold {
                return value % upper;
            }
        }
    }
}

pub(super) struct VariationFamily {
    candidates: Vec<(usize, u32)>,
    total: u32,
}

impl VariationFamily {
    pub(super) fn read(sequences: &[M2AnimSequence], current: usize) -> Result<Self, String> {
        let sequence = sequences
            .get(current)
            .ok_or("missing active animation sequence")?;
        let base = sequences
            .iter()
            .position(|s| s.id == sequence.id && s.variation_id == 0)
            .ok_or_else(|| format!("animation {} has no base variation", sequence.id))?;
        Self::collect_linked(sequences, base, sequence.id)
    }

    fn collect_linked(
        sequences: &[M2AnimSequence],
        base: usize,
        animation: u16,
    ) -> Result<Self, String> {
        let mut family = Self {
            candidates: Vec::new(),
            total: 0,
        };
        let mut visited = std::collections::HashSet::new();
        let mut next = Some(base);
        while let Some(index) = next {
            if !visited.insert(index) {
                return Err(format!("animation {animation} has a cyclic variation list"));
            }
            let candidate = sequences
                .get(index)
                .ok_or_else(|| format!("invalid variation index {index}"))?;
            family.append(index, candidate, animation)?;
            next = match candidate.variation_next {
                -1 => None,
                index if index >= 0 => Some(index as usize),
                index => return Err(format!("invalid variation link {index}")),
            };
        }
        if family.candidates.len() > 1 && family.total == 0 {
            return Err(format!(
                "animation {animation} has no positive variation weights"
            ));
        }
        Ok(family)
    }

    fn append(
        &mut self,
        index: usize,
        sequence: &M2AnimSequence,
        animation: u16,
    ) -> Result<(), String> {
        if sequence.id != animation {
            return Err(format!(
                "animation {animation} links to different animation {}",
                sequence.id
            ));
        }
        let weight = u32::try_from(sequence.frequency)
            .map_err(|_| format!("negative variation weight at sequence {index}"))?;
        if sequence.duration == 0 && weight > 0 {
            return Err(format!("weighted variation {index} has zero duration"));
        }
        self.total = self
            .total
            .checked_add(weight)
            .ok_or("variation weight total overflow")?;
        self.candidates.push((index, weight));
        Ok(())
    }

    pub(super) fn validate_elapsed(
        &self,
        sequences: &[M2AnimSequence],
        elapsed_ms: f64,
    ) -> Result<(), String> {
        // Exact random selection is sequential. Reject pathological catch-up rather than
        // silently skipping draws or monopolizing a frame with millions of transitions.
        const MAX_BOUNDARIES: f64 = 4096.0;
        let shortest = self
            .candidates
            .iter()
            .filter(|(_, weight)| *weight > 0)
            .map(|(index, _)| sequences[*index].duration)
            .min()
            .ok_or("variation family has no playable duration")?;
        if elapsed_ms / f64::from(shortest) >= MAX_BOUNDARIES {
            return Err("elapsed animation requires more than 4096 variation boundaries".into());
        }
        Ok(())
    }

    pub(super) fn is_single(&self) -> bool {
        self.candidates.len() == 1
    }

    pub(super) fn choose(&self, sample: &mut impl FnMut(u32) -> u32) -> Result<usize, String> {
        if self.candidates.len() == 1 {
            return Ok(self.candidates[0].0);
        }
        self.choose_roll(sample(self.total))
    }

    fn choose_roll(&self, mut roll: u32) -> Result<usize, String> {
        if roll >= self.total {
            return Err(format!(
                "variation roll {roll} exceeds weight total {}",
                self.total
            ));
        }
        for &(index, weight) in &self.candidates {
            if roll < weight {
                return Ok(index);
            }
            roll -= weight;
        }
        Err("variation weights did not cover the supplied roll".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn wolf_sequences() -> Vec<M2AnimSequence> {
        crate::asset::m2::load_m2(std::path::Path::new("data/models/126487.m2"), &[0; 3])
            .unwrap()
            .sequences
    }

    fn wolf_data() -> M2AnimData {
        M2AnimData {
            bones: vec![],
            spherical_billboards: vec![],
            sequences: wolf_sequences(),
            bone_tracks: vec![],
            joint_entities: vec![],
        }
    }

    fn player(index: usize, looping: bool) -> M2AnimPlayer {
        M2AnimPlayer {
            current_seq_idx: index,
            time_ms: 0.0,
            looping,
            transition: None,
        }
    }

    #[test]
    fn wolf_weighted_variations_large_elapsed_matches_partitioned_time() {
        let data = wolf_data();
        let rolls = [32766, 0, 30445, 31537, 0];
        let mut whole_roll = 0;
        let mut split_roll = 0;
        let mut whole = player(2, true);
        let mut split = player(2, true);
        super::super::runtime::advance_player_time(&mut whole, &data, 120_000.0, |_| {
            let roll = rolls[whole_roll % rolls.len()];
            whole_roll += 1;
            roll
        })
        .unwrap();
        for _ in 0..1200 {
            super::super::runtime::advance_player_time(&mut split, &data, 100.0, |_| {
                let roll = rolls[split_roll % rolls.len()];
                split_roll += 1;
                roll
            })
            .unwrap();
        }
        assert_eq!(whole_roll, split_roll);
        assert!(whole_roll > 20 && whole_roll < 91);
        assert_eq!(whole.current_seq_idx, split.current_seq_idx);
        assert_eq!(whole.time_ms, split.time_ms);
        assert_eq!(
            whole.transition.as_ref().map(|t| t.blend_elapsed_ms),
            split.transition.as_ref().map(|t| t.blend_elapsed_ms)
        );
    }

    #[test]
    fn wolf_weighted_variations_reject_unbounded_catchup_without_dropping_time() {
        let data = wolf_data();
        let mut active = player(11, true);
        active.time_ms = 30.0;
        let result =
            super::super::runtime::advance_player_time(&mut active, &data, f32::MAX, |_| {
                panic!("invalid catch-up must not draw")
            });
        assert!(result.unwrap_err().contains("4096"));
        assert_eq!(active.current_seq_idx, 11);
        assert_eq!(active.time_ms, 30.0);
    }

    #[test]
    fn wolf_weighted_variations_leave_non_looping_completion_unchanged() {
        let data = wolf_data();
        let mut once = player(11, false);
        super::super::runtime::advance_player_time(&mut once, &data, 120_000.0, |_| {
            panic!("non-looping playback must not sample variants")
        })
        .unwrap();
        assert_eq!(once.current_seq_idx, 11);
        assert_eq!(once.time_ms, 4000.0);
        assert!(once.transition.is_none());
    }

    #[test]
    fn weighted_sequence_parser_retains_replay_and_signed_weight() {
        let mut bytes = [0u8; 64];
        bytes[0x10..0x12].copy_from_slice(&(-5i16).to_le_bytes());
        bytes[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
        bytes[0x18..0x1c].copy_from_slice(&7u32.to_le_bytes());
        bytes[0x3c..0x3e].copy_from_slice(&(-1i16).to_le_bytes());
        let rows = crate::asset::m2_anim::parse_sequences_at(&bytes, 0, 1).unwrap();
        assert_eq!(rows[0].frequency, -5);
        assert_eq!(rows[0].replay, [3, 7]);
        assert_eq!(rows[0].variation_next, -1);
    }

    #[test]
    fn wolf_weighted_variations_match_every_authored_roll() {
        let sequences = wolf_sequences();
        let family = VariationFamily::read(&sequences, 11).unwrap();
        let mut counts = [0u32; 46];
        for roll in 0..32767 {
            counts[family.choose_roll(roll).unwrap()] += 1;
        }
        assert_eq!(
            [counts[2], counts[9], counts[10], counts[11]],
            [30445, 1092, 1170, 60]
        );
        assert_eq!(counts.iter().sum::<u32>(), 32767);
        assert!(family.choose_roll(32767).is_err());
        for index in [2, 9, 10, 11] {
            assert_eq!(sequences[index].replay, [0, 0]);
        }
    }

    #[test]
    fn wolf_weighted_variations_reject_invalid_metadata() {
        let original = wolf_sequences();
        for fault in 0..5 {
            let mut sequences = original.clone();
            match fault {
                0 => sequences[11].variation_next = 2,
                1 => sequences[9].frequency = -1,
                2 => sequences[9].id = 98,
                3 => sequences[9].duration = 0,
                _ => {
                    for index in [2, 9, 10, 11] {
                        sequences[index].frequency = 0;
                    }
                }
            }
            assert!(
                VariationFamily::read(&sequences, 2).is_err(),
                "fault {fault}"
            );
        }
    }

    #[test]
    fn wolf_weighted_variations_use_independent_reproducible_entity_streams() {
        let a = Entity::from_bits(1);
        let b = Entity::from_bits(2);
        let draw = |owner| {
            let mut random = VariantRandom::default();
            (0..32)
                .map(|_| random.sample(owner, 32767))
                .collect::<Vec<_>>()
        };
        assert_eq!(draw(a), draw(a));
        assert_ne!(draw(a), draw(b));
        assert!(draw(a).into_iter().all(|roll| roll < 32767));
    }
}
