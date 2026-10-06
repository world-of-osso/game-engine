use super::*;

impl SpellVisualCatalog {
    /// The `SoundKit`s kits, missiles, unit voices and melee reference, with their
    /// `SoundKitEntry` files.
    pub(super) fn read_sound_kits(&mut self, dir: &Path) -> Result<(), String> {
        let referenced: std::collections::HashSet<u32> = self
            .kits
            .values()
            .flat_map(|kit| kit.sound_kits.iter().copied())
            .chain(self.missiles.values().flatten().map(|row| row.sound_kit))
            .chain(self.voices.sound_kits())
            .chain(self.melee.sound_kits())
            .filter(|&id| id != 0)
            .collect();
        self.read_referenced_sounds(dir, &referenced)?;
        self.read_sound_files(dir)
    }

    pub(super) fn read_referenced_sounds(
        &mut self,
        dir: &Path,
        referenced: &std::collections::HashSet<u32>,
    ) -> Result<(), String> {
        let kits = Table::read(dir, "SoundKit")?;
        let kit_ints = kits.ints(["ID", "Flags"])?;
        let kit_floats = kits.floats(["VolumeFloat", "MinDistance", "DistanceCutoff"])?;
        for ([id, flags], [volume, min_distance, distance_cutoff]) in
            kit_ints.into_iter().zip(kit_floats)
        {
            if referenced.contains(&(id as u32)) {
                self.sound_kits.insert(
                    id as u32,
                    KitSound {
                        sound_kit_id: id as u32,
                        volume,
                        looping: flags & SOUND_KIT_LOOPING != 0,
                        min_distance,
                        distance_cutoff,
                        files: Vec::new(),
                    },
                );
            }
        }
        Ok(())
    }

    pub(super) fn read_sound_files(&mut self, dir: &Path) -> Result<(), String> {
        let entries = Table::read(dir, "SoundKitEntry")?;
        let entry_ints = entries.ints(["ID", "SoundKitID", "FileDataID", "Frequency"])?;
        let entry_floats = entries.floats(["Volume"])?;
        let mut rows: Vec<_> = entry_ints.into_iter().zip(entry_floats).collect();
        rows.sort_by_key(|([id, ..], _)| *id);
        for ([_, kit, fdid, frequency], [volume]) in rows {
            if let Some(sound) = self.sound_kits.get_mut(&(kit as u32)) {
                sound.files.push(SoundFile {
                    fdid: fdid as u32,
                    frequency: frequency as u32,
                    volume,
                });
            }
        }
        Ok(())
    }
}
