//! Authored Mount/AnimKit joins and deterministic rider sampling.
//! Condition interpretation is an explicit inference from installed segment dependencies;
//! DB2 definitions name the columns but do not publish the native condition enums.
use crate::csv_util::read_db2_rows;
use std::{collections::HashMap, path::Path};

#[derive(Clone, Copy, Debug)]
pub struct MountSpecial {
    pub rider_kit: u32,
    pub visual_kit: u32,
}

#[derive(Clone, Debug)]
pub struct KitBoneScope {
    /// `None` is the authored full-body set, not an absent mask.
    pub root_key_bone: Option<i32>,
    pub priority: i32,
}

#[derive(Clone, Debug)]
pub struct KitSegment {
    pub id: u32,
    pub order: u32,
    pub animation: u16,
    pub offset_ms: f64,
    pub start_condition: u8,
    pub start_reference: u32,
    pub start_delay_ms: f64,
    pub end_condition: u8,
    pub end_reference: u32,
    pub end_delay_ms: f64,
    pub speed: f64,
    pub flags: u32,
    pub forced_variation: u16,
    pub override_config_flags: u32,
    pub config_flags: u32,
    pub loop_to: i32,
    pub blend_in_ms: f64,
    pub blend_out_ms: f64,
    pub bones: Vec<KitBoneScope>,
}

pub struct MountSpecialCatalog {
    mounts: HashMap<u32, MountSpecial>,
    kits: HashMap<u32, Vec<KitSegment>>,
    visual_anims: HashMap<u32, Vec<u32>>,
}

impl MountSpecialCatalog {
    pub fn read(root: &Path) -> Result<Self, String> {
        let dir = root.join("db2/active");
        let mut mounts = HashMap::new();
        read_db2_rows(&dir.join("Mount.csv"), |r| {
            mounts.insert(
                r.parse("ID")?,
                MountSpecial {
                    rider_kit: r.parse("MountSpecialRiderAnimKitID")?,
                    visual_kit: r.parse("MountSpecialSpellVisualKitID")?,
                },
            );
            Ok(())
        })?;
        let visual_anims = read_visual_anims(&dir)?;
        let mut wanted: Vec<u32> = mounts
            .values()
            .map(|mount| mount.rider_kit)
            .filter(|id| *id != 0)
            .collect();
        for mount in mounts.values() {
            if let Some(kits) = visual_anims.get(&mount.visual_kit) {
                wanted.extend(kits);
            }
        }
        let scopes = read_scopes(&dir)?;
        let mut config_flags: HashMap<u32, u32> = HashMap::new();
        read_db2_rows(&dir.join("AnimKitConfig.csv"), |r| {
            config_flags.insert(r.parse("ID")?, r.parse("ConfigFlags")?);
            Ok(())
        })?;
        let mut kits: HashMap<u32, Vec<KitSegment>> = HashMap::new();
        read_db2_rows(&dir.join("AnimKitSegment.csv"), |r| {
            let kit = r.parse::<u32>("ParentAnimKitID")?;
            if !wanted.contains(&kit) {
                return Ok(());
            }
            let config = r.parse::<u32>("AnimKitConfigID")?;
            kits.entry(kit).or_default().push(KitSegment {
                id: r.parse("ID")?,
                order: r.parse("OrderIndex")?,
                animation: r.parse("AnimID")?,
                offset_ms: r.parse("AnimStartTime")?,
                start_condition: r.parse("StartCondition")?,
                start_reference: r.parse("StartConditionParam")?,
                start_delay_ms: r.parse("StartConditionDelay")?,
                end_condition: r.parse("EndCondition")?,
                end_reference: r.parse("EndConditionParam")?,
                end_delay_ms: r.parse("EndConditionDelay")?,
                speed: r.parse("Speed")?,
                flags: r.parse("SegmentFlags")?,
                forced_variation: r.parse("ForcedVariation")?,
                override_config_flags: r.parse("OverrideConfigFlags")?,
                config_flags: *config_flags
                    .get(&config)
                    .ok_or_else(|| format!("AnimKitConfig {config} missing"))?,
                loop_to: r.parse("LoopToSegmentIndex")?,
                blend_in_ms: r.parse("BlendInTimeMs")?,
                blend_out_ms: r.parse("BlendOutTimeMs")?,
                bones: scopes
                    .get(&config)
                    .ok_or_else(|| format!("AnimKitConfig {config} has no bone sets"))?
                    .clone(),
            });
            Ok(())
        })?;
        for segments in kits.values_mut() {
            segments.sort_by_key(|segment| segment.order);
        }
        Ok(Self {
            mounts,
            kits,
            visual_anims,
        })
    }

    pub fn mount(&self, id: u32) -> Result<MountSpecial, String> {
        self.mounts
            .get(&id)
            .copied()
            .ok_or_else(|| format!("Mount {id} missing"))
    }

    pub fn kit(&self, id: u32) -> Result<&[KitSegment], String> {
        self.kits
            .get(&id)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("AnimKit {id} missing"))
    }

    pub fn rider_kits(&self, mount: MountSpecial) -> Result<Vec<&[KitSegment]>, String> {
        let mut ids = Vec::new();
        if mount.rider_kit != 0 {
            ids.push(mount.rider_kit);
        }
        if let Some(visual) = self.visual_anims.get(&mount.visual_kit) {
            ids.extend(visual);
        }
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter().map(|id| self.kit(id)).collect()
    }
}

fn read_visual_anims(dir: &Path) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut anims: HashMap<u32, u32> = HashMap::new();
    read_db2_rows(&dir.join("SpellVisualAnim.csv"), |r| {
        anims.insert(r.parse("ID")?, r.parse("AnimKitID")?);
        Ok(())
    })?;
    let mut kits: HashMap<u32, Vec<u32>> = HashMap::new();
    read_db2_rows(&dir.join("SpellVisualKitEffect.csv"), |r| {
        if r.parse::<u32>("EffectType")? == 6 {
            let effect = r.parse::<u32>("Effect")?;
            if let Some(&kit) = anims.get(&effect).filter(|id| **id != 0) {
                kits.entry(r.parse("ParentSpellVisualKitID")?)
                    .or_default()
                    .push(kit);
            }
        }
        Ok(())
    })?;
    Ok(kits)
}

fn read_scopes(dir: &Path) -> Result<HashMap<u32, Vec<KitBoneScope>>, String> {
    let mut roots: HashMap<u32, i32> = HashMap::new();
    let mut priorities: HashMap<u32, i32> = HashMap::new();
    read_db2_rows(&dir.join("AnimKitBoneSet.csv"), |r| {
        roots.insert(r.parse("ID")?, r.parse("BoneDataID")?);
        Ok(())
    })?;
    read_db2_rows(&dir.join("AnimKitPriority.csv"), |r| {
        priorities.insert(r.parse("ID")?, r.parse("Priority")?);
        Ok(())
    })?;
    let mut scopes: HashMap<u32, Vec<KitBoneScope>> = HashMap::new();
    read_db2_rows(&dir.join("AnimKitConfigBoneSet.csv"), |r| {
        let bone_set = r.parse::<u32>("AnimKitBoneSetID")?;
        let priority = r.parse::<u32>("AnimKitPriorityID")?;
        let root = *roots
            .get(&bone_set)
            .ok_or_else(|| format!("AnimKitBoneSet {bone_set} missing"))?;
        let priority = *priorities
            .get(&priority)
            .ok_or_else(|| format!("AnimKitPriority {priority} missing"))?;
        scopes
            .entry(r.parse("ParentAnimKitConfigID")?)
            .or_default()
            .push(KitBoneScope {
                root_key_bone: (root >= 0).then_some(root),
                priority,
            });
        Ok(())
    })?;
    Ok(scopes)
}

#[derive(Debug)]
struct TimedSegment {
    segment: KitSegment,
    start_ms: f64,
    end_ms: f64,
    duration_ms: f64,
}

#[derive(Debug)]
pub struct KitTimeline {
    segments: Vec<TimedSegment>,
}

pub struct KitSample<'a> {
    pub segment_id: u32,
    pub animation: u16,
    pub variation: u16,
    pub time_ms: f64,
    pub weight: f32,
    pub bones: &'a [KitBoneScope],
}

impl KitTimeline {
    pub fn resolve(
        segments: &[KitSegment],
        mut duration: impl FnMut(u16) -> Option<f64>,
    ) -> Result<Self, String> {
        let mut resolved = Vec::new();
        for segment in segments {
            let duration_ms = duration(segment.animation).ok_or_else(|| {
                format!(
                    "AnimKit segment {}: missing animation {}",
                    segment.id, segment.animation
                )
            })?;
            if !duration_ms.is_finite()
                || duration_ms <= 0.0
                || !segment.speed.is_finite()
                || segment.speed == 0.0
            {
                return Err(format!(
                    "AnimKit segment {}: invalid duration/speed",
                    segment.id
                ));
            }
            let start_ms = match segment.start_condition {
                0 => 0.0,
                1 => reference(&resolved, segment.start_reference)?.start_ms,
                2 => reference(&resolved, segment.start_reference)?.end_ms,
                kind => {
                    return Err(format!(
                        "AnimKit segment {}: unsupported start condition {kind}",
                        segment.id
                    ));
                }
            } + segment.start_delay_ms;
            let end_ms = match segment.end_condition {
                0 if segment.loop_to == segment.order as i32 => f64::INFINITY,
                0 => start_ms + (duration_ms - segment.offset_ms).max(0.0) / segment.speed.abs(),
                1 => f64::INFINITY,
                4 if segment.end_reference == segment.order => start_ms + segment.end_delay_ms,
                4 => reference(&resolved, segment.end_reference)?.start_ms + segment.end_delay_ms,
                kind => {
                    return Err(format!(
                        "AnimKit segment {}: unsupported end condition {kind}",
                        segment.id
                    ));
                }
            };
            if segment.loop_to >= 0 && segment.loop_to != segment.order as i32 {
                return Err(format!(
                    "AnimKit segment {}: unsupported cross-segment loop {}",
                    segment.id, segment.loop_to
                ));
            }
            resolved.push(TimedSegment {
                segment: segment.clone(),
                start_ms,
                end_ms,
                duration_ms,
            });
        }
        Ok(Self { segments: resolved })
    }

    pub fn sample(&self, elapsed_ms: f64) -> Vec<KitSample<'_>> {
        self.segments
            .iter()
            .filter_map(|timed| {
                let segment = &timed.segment;
                let age = elapsed_ms - timed.start_ms;
                if age < 0.0 || elapsed_ms >= timed.end_ms + segment.blend_out_ms {
                    return None;
                }
                let clock = (elapsed_ms.min(timed.end_ms) - timed.start_ms) * segment.speed.abs()
                    + segment.offset_ms;
                let held = segment.end_condition == 1 || segment.loop_to >= 0;
                let clock = if held {
                    clock % timed.duration_ms
                } else {
                    clock.min(timed.duration_ms)
                };
                let time_ms = if segment.speed < 0.0 {
                    timed.duration_ms - clock
                } else {
                    clock
                };
                let fade_in = if segment.blend_in_ms == 0.0 {
                    1.0
                } else {
                    (age / segment.blend_in_ms).min(1.0)
                };
                let fade_out = if elapsed_ms <= timed.end_ms {
                    1.0
                } else {
                    1.0 - (elapsed_ms - timed.end_ms) / segment.blend_out_ms
                };
                Some(KitSample {
                    segment_id: segment.id,
                    animation: segment.animation,
                    variation: segment.forced_variation,
                    time_ms,
                    weight: (fade_in * fade_out) as f32,
                    bones: &segment.bones,
                })
            })
            .collect()
    }
}

fn reference(segments: &[TimedSegment], order: u32) -> Result<&TimedSegment, String> {
    segments
        .iter()
        .find(|timed| timed.segment.order == order)
        .ok_or_else(|| format!("AnimKit unresolved reference order {order}"))
}
