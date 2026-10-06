use super::*;

impl SpellVisualCatalog {
    pub(super) fn read_kits(&mut self, dir: &Path) -> Result<(), String> {
        self.read_effect_names(dir)?;
        let models = self.read_kit_models(dir)?;
        let anims = read_kit_anims(dir)?;
        self.read_kit_effects(dir, &models, &anims)
    }

    pub(super) fn read_effect_names(&mut self, dir: &Path) -> Result<(), String> {
        let names = Table::read(dir, "SpellVisualEffectName")?;
        let name_ints = names.ints(["ID", "ModelFileDataID"])?;
        let name_floats = names.floats(["Scale"])?;
        for ([id, model], [scale]) in name_ints.into_iter().zip(name_floats) {
            self.effect_names.insert(
                id as u32,
                EffectName {
                    model_fdid: model as u32,
                    scale,
                },
            );
        }
        Ok(())
    }

    pub(super) fn read_kit_models(&self, dir: &Path) -> Result<HashMap<u32, KitModel>, String> {
        let attaches = Table::read(dir, "SpellVisualKitModelAttach")?;
        let attach_ints = read_attach_ints(&attaches)?;
        let attach_floats = read_attach_floats(&attaches)?;
        Ok(attach_ints
            .into_iter()
            .zip(attach_floats)
            .filter_map(|(ints, floats)| parse_kit_model(ints, floats, &self.effect_names))
            .collect())
    }

    pub(super) fn read_kit_effects(
        &mut self,
        dir: &Path,
        models: &HashMap<u32, KitModel>,
        anims: &HashMap<u32, (i32, i32, u32)>,
    ) -> Result<(), String> {
        let effects = Table::read(dir, "SpellVisualKitEffect")?;
        let mut rows = effects.ints(["ParentSpellVisualKitID", "EffectType", "Effect", "ID"])?;
        // Kit effects in ID order, as authored.
        rows.sort_by_key(|&[kit, _, _, id]| (kit, id));
        for [kit, kind, effect, _] in rows {
            let kit = self.kits.entry(kit as u32).or_default();
            match kind as u32 {
                EFFECT_MODEL_ATTACH => kit.models.extend(models.get(&(effect as u32)).cloned()),
                EFFECT_SOUND_KIT => kit.sound_kits.push(effect as u32),
                EFFECT_UNIT_SOUND => kit.unit_sounds.extend(UnitSound::from_db2(effect as u32)),
                EFFECT_ANIM if kit.anim.is_none() => {
                    kit.anim = anims.get(&(effect as u32)).copied()
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn read_anims(&mut self, dir: &Path) -> Result<(), String> {
        let segments = Table::read(dir, "AnimKitSegment")?;
        for [kit, order, anim, loop_to] in segments.ints([
            "ParentAnimKitID",
            "OrderIndex",
            "AnimID",
            "LoopToSegmentIndex",
        ])? {
            self.anim_kits
                .entry(kit as u32)
                .or_default()
                .push(SegmentRow {
                    order: order as u32,
                    anim_id: anim as i32,
                    loop_to: loop_to as i32,
                });
        }
        for segments in self.anim_kits.values_mut() {
            segments.sort_by_key(|segment| segment.order);
        }
        Ok(())
    }

    /// `SpellVisualAnim`: an `AnimKit`'s first segment (looping when a segment loops
    /// back), else the loop clip (held until the kit ends unless the kit is a
    /// one-shot), else the initial clip once.
    pub(super) fn kit_animation(
        &self,
        (initial, looped, kit): (i32, i32, u32),
        end: VisualEvent,
    ) -> Option<KitAnimation> {
        let held = end != VisualEvent::OneShot;
        if kit != 0 {
            let segments = self.anim_kits.get(&kit)?;
            let first = segments.first()?;
            return Some(KitAnimation {
                anim_id: anim_id(i64::from(first.anim_id))?,
                looping: held && segments.iter().any(|segment| segment.loop_to >= 0),
            });
        }
        if let Some(id) = anim_id(i64::from(looped)) {
            return Some(KitAnimation {
                anim_id: id,
                looping: held,
            });
        }
        anim_id(i64::from(initial)).map(|anim_id| KitAnimation {
            anim_id,
            looping: false,
        })
    }
}

fn parse_kit_model(
    [id, name, attach, start, anim, end]: [i64; 6],
    [x, y, z, yaw, pitch, roll, scale, delay]: [f32; 8],
    names: &HashMap<u32, EffectName>,
) -> Option<(u32, KitModel)> {
    let effect = names.get(&(name as u32))?;
    if effect.model_fdid == 0 {
        return None;
    }
    let model = KitModel {
        model_fdid: effect.model_fdid,
        attachment: attachment(attach),
        offset: [x, y, z],
        yaw,
        pitch,
        roll,
        scale: scale * effect.scale,
        start_delay: delay,
        start_anim_id: anim_id(start),
        anim_id: anim_id(anim),
        end_anim_id: anim_id(end),
    };
    Some((id as u32, model))
}

fn read_attach_ints(attaches: &Table) -> Result<Vec<[i64; 6]>, String> {
    attaches.ints([
        "ID",
        "SpellVisualEffectNameID",
        "AttachmentID",
        "StartAnimID",
        "AnimID",
        "EndAnimID",
    ])
}

fn read_attach_floats(attaches: &Table) -> Result<Vec<[f32; 8]>, String> {
    attaches.floats([
        "Offset_0",
        "Offset_1",
        "Offset_2",
        "Yaw",
        "Pitch",
        "Roll",
        "Scale",
        "StartDelay",
    ])
}

fn read_kit_anims(dir: &Path) -> Result<HashMap<u32, (i32, i32, u32)>, String> {
    let anims = Table::read(dir, "SpellVisualAnim")?;
    let anims: HashMap<u32, (i32, i32, u32)> = anims
        .ints(["ID", "InitialAnimID", "LoopAnimID", "AnimKitID"])?
        .into_iter()
        .map(|[id, initial, looped, kit]| (id as u32, (initial as i32, looped as i32, kit as u32)))
        .collect();
    Ok(anims)
}
