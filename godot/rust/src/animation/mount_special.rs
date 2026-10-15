//! MountSpecial owns the mount sequence until completion; rider kits layer all segments.
use super::{AnimationState, BonePose, WowAnimationPlayer};
use game_engine_core::mount_special::{KitSegment, KitTimeline};
use std::collections::HashMap;

pub(super) struct RiderKits {
    timelines: Vec<KitTimeline>,
    sequences: HashMap<(u16, u16), usize>,
    start_ms: f64,
    pub(super) end_ms: f64,
}

impl AnimationState {
    pub fn play_mount_special(&mut self, fallbacks: &HashMap<u16, u16>) -> Result<f64, String> {
        let clip = self
            .resolve_clip(94, fallbacks)
            .ok_or("Mount has no MountSpecial sequence or authored fallback")?;
        let index = self
            .base_sequence(self.replacement_id(clip))
            .ok_or("MountSpecial resolved sequence missing")?;
        let duration = f64::from(self.sequences[index].duration);
        self.start_transition(index, false);
        self.source_id = clip;
        self.mount_special_end = Some(self.global_ms + duration);
        Ok(duration)
    }

    pub fn play_mount_kits(
        &mut self,
        kits: &[&[KitSegment]],
        duration_ms: f64,
        fallbacks: &HashMap<u16, u16>,
    ) -> Result<(), String> {
        let mut sequences = HashMap::new();
        let mut timelines = Vec::new();
        for kit in kits {
            for segment in *kit {
                let clip = self
                    .resolve_clip(segment.animation, fallbacks)
                    .ok_or_else(|| {
                        format!(
                            "Rider has no animation {} for segment {}",
                            segment.animation, segment.id
                        )
                    })?;
                let id = self.replacement_id(clip);
                let index = self
                    .sequences
                    .iter()
                    .position(|sequence| {
                        sequence.id == id && sequence.variation_id == segment.forced_variation
                    })
                    .ok_or_else(|| {
                        format!(
                            "Rider animation {id} missing variation {}",
                            segment.forced_variation
                        )
                    })?;
                for scope in &segment.bones {
                    if let Some(root) = scope.root_key_bone
                        && !self.kit_bones.contains_key(&root)
                    {
                        return Err(format!("Rider skeleton has no AnimKit bone root {root}"));
                    }
                }
                sequences.insert((segment.animation, segment.forced_variation), index);
            }
            timelines.push(KitTimeline::resolve(kit, |id| {
                let index = sequences
                    .iter()
                    .find(|((animation, _), _)| *animation == id)?
                    .1;
                Some(f64::from(self.sequences[*index].duration))
            })?);
        }
        self.rider_kits = (!timelines.is_empty()).then_some(RiderKits {
            timelines,
            sequences,
            start_ms: self.global_ms,
            end_ms: self.global_ms + duration_ms,
        });
        Ok(())
    }

    pub fn cancel_mount_special(&mut self) {
        self.mount_special_end = None;
        if let Some(kits) = &mut self.rider_kits {
            kits.end_ms = kits.end_ms.min(self.global_ms);
        }
    }

    pub(super) fn apply_mount_kits(&self, mut base: Vec<BonePose>) -> Vec<BonePose> {
        let Some(kits) = &self.rider_kits else {
            return base;
        };
        let elapsed = self.global_ms.min(kits.end_ms) - kits.start_ms;
        let release =
            (1.0 - (self.global_ms - kits.end_ms).max(0.0) / 150.0).clamp(0.0, 1.0) as f32;
        let mut layers = Vec::new();
        for timeline in &kits.timelines {
            for sample in timeline.sample(elapsed) {
                let index = kits.sequences[&(sample.animation, sample.variation)];
                let poses = self.sample_sequence(index, sample.time_ms);
                for scope in sample.bones {
                    layers.push((
                        scope.priority,
                        scope.root_key_bone,
                        sample.weight * release,
                        poses.clone(),
                    ));
                }
            }
        }
        // Equal-priority authored order is stable; higher priorities compose last.
        layers.sort_by_key(|(priority, _, _, _)| *priority);
        for (_, root, weight, poses) in layers {
            for (bone, (current, target)) in base.iter_mut().zip(poses).enumerate() {
                if root.is_none_or(|root| self.kit_bones[&root][bone]) {
                    *current = current.blend(target, weight);
                }
            }
        }
        base
    }
}

pub(super) fn bone_masks(model: &game_engine_core::m2::Model) -> HashMap<i32, Vec<bool>> {
    model
        .bones
        .iter()
        .enumerate()
        .filter(|(_, bone)| bone.key_bone_id == 4)
        .map(|(root, bone)| {
            let mask = (0..model.bones.len())
                .map(|index| {
                    let mut current = index as i32;
                    while current >= 0 {
                        if current as usize == root {
                            return true;
                        }
                        current = i32::from(model.bones[current as usize].parent_bone_id);
                    }
                    false
                })
                .collect();
            (bone.key_bone_id, mask)
        })
        .collect()
}

impl WowAnimationPlayer {
    pub(crate) fn play_mount_special(
        &mut self,
        fallbacks: &HashMap<u16, u16>,
    ) -> Result<f64, String> {
        let duration = self
            .animation
            .as_mut()
            .ok_or("Mount animation model not bound")?
            .play_mount_special(fallbacks)?;
        self.write_poses();
        Ok(duration)
    }

    pub(crate) fn play_mount_kits(
        &mut self,
        kits: &[&[KitSegment]],
        duration: f64,
        fallbacks: &HashMap<u16, u16>,
    ) -> Result<(), String> {
        self.animation
            .as_mut()
            .ok_or("Rider animation model not bound")?
            .play_mount_kits(kits, duration, fallbacks)?;
        self.write_poses();
        Ok(())
    }

    pub(crate) fn cancel_mount_special(&mut self) {
        if let Some(animation) = &mut self.animation {
            animation.cancel_mount_special();
        }
    }
}
