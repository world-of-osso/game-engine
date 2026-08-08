use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InWorldSceneStage {
    Empty,
    Character,
    Skybox,
    Terrain,
    Npcs,
    Lighting,
    Particles,
    Ui,
}

impl InWorldSceneStage {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "empty" => Ok(Self::Empty),
            "character" => Ok(Self::Character),
            "skybox" => Ok(Self::Skybox),
            "terrain" => Ok(Self::Terrain),
            "npcs" => Ok(Self::Npcs),
            "lighting" => Ok(Self::Lighting),
            "particles" => Ok(Self::Particles),
            "ui" => Ok(Self::Ui),
            _ => Err(format!(
                "unknown InWorld stage '{value}'; expected empty, character, skybox, terrain, npcs, lighting, particles, or ui"
            )),
        }
    }

    pub(crate) fn includes(self, required: Self) -> bool {
        self >= required
    }
}

pub(crate) fn configured_inworld_scene_stage(
    stage: Option<Res<InWorldSceneStage>>,
) -> InWorldSceneStage {
    stage.as_deref().copied().unwrap_or(InWorldSceneStage::Ui)
}

pub(crate) fn inworld_scene_stage_includes(
    stage: Option<Res<InWorldSceneStage>>,
    required: InWorldSceneStage,
) -> bool {
    configured_inworld_scene_stage(stage).includes(required)
}
