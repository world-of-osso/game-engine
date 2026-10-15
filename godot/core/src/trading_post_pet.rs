//! PerksProgramModel.SetupModelSceneForPets: independent pet and dressed player.
//! Native desiredScale and animation kit23877 remain explicitly unsupported.
use crate::{
    csv_util::read_db2_rows,
    model_scene_actor::{ModelSceneActor, read_scene_actors},
    model_scene_camera::{ModelSceneCamera, read_primary_camera},
};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct PetScenes {
    pub category: u32,
    pub display: u32,
    pub player: u32,
}

#[derive(Clone, Debug)]
pub struct PetComparison {
    pub pet: ModelSceneActor,
    pub player: ModelSceneActor,
    pub player_camera: ModelSceneCamera,
}

impl PetComparison {
    pub fn read(root: &Path, scenes: PetScenes, race: u8, sex: u8) -> Result<Self, String> {
        let pet = read_pet_actor(root, scenes)?;
        let mut player = read_player_actor(root, scenes.player, race, sex)?;
        // Model.lua697–700: relative offset, preserving the selected race/sex actor.
        player.position[0] -= 2.;
        player.position[1] += 2.2;
        let mut player_camera = read_primary_camera(root, scenes.player)?;
        player_camera.yaw = 150_f32.to_radians();
        Ok(Self {
            pet,
            player,
            player_camera,
        })
    }
}

fn read_pet_actor(root: &Path, scenes: PetScenes) -> Result<ModelSceneActor, String> {
    let mut actors = read_scene_actors(root, scenes.category)?;
    let mut pet = actors
        .remove("pet")
        .ok_or_else(|| format!("UiModelScene {} has no pet actor", scenes.category))?;
    let displays = read_scene_actors(root, scenes.display)?;
    let display = displays
        .values()
        .min_by_key(|actor| actor.id)
        .ok_or_else(|| format!("Pet display scene {} has no actor", scenes.display))?;
    // Products.TranslateDisplayInfo overrides transform, not the category actor roster.
    pet.position = display.position.map(|value| (value * 10.).round() / 10.);
    pet.orientation = display
        .orientation
        .map(|value| ((value.to_degrees() * 10.).round() / 10.).to_radians());
    Ok(pet)
}

fn read_player_actor(
    root: &Path,
    scene: u32,
    race: u8,
    sex: u8,
) -> Result<ModelSceneActor, String> {
    let gender = match sex {
        0 => "male",
        1 => "female",
        _ => return Err(format!("Trading Post player sex {sex} unsupported")),
    };
    let mut race_name = None;
    read_db2_rows(&root.join("db2/active/ChrRaces.csv"), |row| {
        if row.parse::<u32>("ID")? == u32::from(race) {
            race_name = Some(row.field("ClientFileString")?.to_lowercase());
        }
        Ok(())
    })?;
    let race_name = race_name.ok_or_else(|| format!("ChrRaces {race} missing"))?;
    let mut actors = read_scene_actors(root, scene)?;
    // SetupPlayerModelScene tries race+gender, then race. No generic actor substitute.
    actors
        .remove(&format!("{race_name}-{gender}"))
        .or_else(|| actors.remove(&race_name))
        .ok_or_else(|| {
            format!("UiModelScene {scene} has no {race_name}-{gender}/{race_name} actor")
        })
}
