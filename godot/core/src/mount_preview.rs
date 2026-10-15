//! Retail Mount metadata needed by ModelSceneMixin.AttachPlayerToMount.
use crate::{
    csv_util::read_db2_rows,
    model_scene_actor::{ModelSceneActor, read_scene_actors},
};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug)]
pub struct MountPreview {
    pub flags: u32,
    pub rider_animation_kit: u32,
    pub rider_spell_visual_kit: u32,
    pub actors: BTreeMap<String, ModelSceneActor>,
}
impl MountPreview {
    pub fn read(root: &Path, mount_id: u32, scene_id: u32) -> Result<Self, String> {
        let mut metadata = None;
        read_db2_rows(&root.join("db2/active/Mount.csv"), |row| {
            if row.parse::<u32>("ID")? == mount_id {
                metadata = Some((
                    row.parse("Flags")?,
                    row.parse("MountSpecialRiderAnimKitID")?,
                    row.parse("MountSpecialSpellVisualKitID")?,
                ));
            }
            Ok(())
        })?;
        let (flags, rider_animation_kit, rider_spell_visual_kit) =
            metadata.ok_or_else(|| format!("Mount {mount_id} metadata missing"))?;
        let actors = read_scene_actors(root, scene_id)?;
        for tag in ["mount", "player-rider"] {
            if !actors.contains_key(tag) {
                return Err(format!("Mount scene {scene_id} actor {tag} missing"));
            }
        }
        Ok(Self {
            flags,
            rider_animation_kit,
            rider_spell_visual_kit,
            actors,
        })
    }
    /// Products.TranslateDisplayInfo supplies display overrides, while the mount
    /// scene's actor roster/normalization/requested scale remain active.
    pub fn apply_display_scene(&mut self, root: &Path, scene_id: u32) -> Result<(), String> {
        let actors = read_scene_actors(root, scene_id)?;
        let display = actors
            .values()
            .min_by_key(|actor| actor.id)
            .ok_or_else(|| format!("Mount display scene {scene_id} has no actor"))?;
        let mount = self.actors.get_mut("mount").expect("validated mount actor");
        mount.position = display.position.map(|value| (value * 10.).round() / 10.);
        mount.orientation = display
            .orientation
            .map(|value| ((value.to_degrees() * 10.).round() / 10.).to_radians());
        mount.animation = display.animation;
        Ok(())
    }
    pub fn is_self_mount(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn shows_rider(&self, hidden: bool) -> bool {
        // TrinityCore a352b1fa MountFlags, shared Retail ModelSceneMixin exclusion.
        !hidden && self.flags & (2 | 4096) == 0
    }
}
