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

pub(crate) fn effective_inworld_scene_stage(stage: Option<InWorldSceneStage>) -> InWorldSceneStage {
    stage.unwrap_or(InWorldSceneStage::Ui)
}

pub(crate) fn configured_inworld_scene_stage(
    stage: Option<Res<InWorldSceneStage>>,
) -> InWorldSceneStage {
    effective_inworld_scene_stage(stage.as_deref().copied())
}

pub(crate) fn inworld_scene_stage_includes(
    stage: Option<Res<InWorldSceneStage>>,
    required: InWorldSceneStage,
) -> bool {
    configured_inworld_scene_stage(stage).includes(required)
}

pub(crate) fn player_visual_is_enabled(stage: InWorldSceneStage, is_local: bool) -> bool {
    let required = if is_local {
        InWorldSceneStage::Character
    } else {
        InWorldSceneStage::Npcs
    };
    stage.includes(required)
}

pub(crate) fn npc_visuals_are_enabled(stage: InWorldSceneStage) -> bool {
    stage.includes(InWorldSceneStage::Npcs)
}

pub(crate) fn terrain_is_required_for_loading(stage: InWorldSceneStage) -> bool {
    stage.includes(InWorldSceneStage::Terrain)
}

pub(crate) fn inworld_scene_stage_allows_character(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Character)
}

pub(crate) fn inworld_scene_stage_allows_skybox(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Skybox)
}

pub(crate) fn inworld_scene_stage_allows_terrain(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Terrain)
}

pub(crate) fn inworld_scene_stage_allows_npcs(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Npcs)
}

pub(crate) fn inworld_scene_stage_allows_lighting(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Lighting)
}

pub(crate) fn inworld_scene_stage_allows_particles(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Particles)
}

pub(crate) fn inworld_scene_stage_allows_ui(stage: Option<Res<InWorldSceneStage>>) -> bool {
    inworld_scene_stage_includes(stage, InWorldSceneStage::Ui)
}

pub(crate) fn configured_inworld_scene_stage_for_app(app: &App) -> InWorldSceneStage {
    effective_inworld_scene_stage(app.world().get_resource::<InWorldSceneStage>().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Counter(usize);

    fn increment_counter(mut counter: ResMut<Counter>) {
        counter.0 += 1;
    }

    #[test]
    fn character_guard_skips_empty_then_runs_after_stage_change() {
        let mut app = App::new();
        app.insert_resource(InWorldSceneStage::Empty)
            .init_resource::<Counter>()
            .add_systems(
                Update,
                increment_counter.run_if(inworld_scene_stage_allows_character),
            );

        app.update();
        assert_eq!(app.world().resource::<Counter>().0, 0);

        app.world_mut()
            .insert_resource(InWorldSceneStage::Character);
        app.update();
        assert_eq!(app.world().resource::<Counter>().0, 1);
    }

    #[test]
    fn local_character_precedes_remote_entity_visuals() {
        assert!(!player_visual_is_enabled(InWorldSceneStage::Empty, true));
        assert!(player_visual_is_enabled(InWorldSceneStage::Character, true));
        assert!(!player_visual_is_enabled(
            InWorldSceneStage::Character,
            false
        ));
        assert!(player_visual_is_enabled(InWorldSceneStage::Npcs, false));
        assert!(!npc_visuals_are_enabled(InWorldSceneStage::Terrain));
        assert!(npc_visuals_are_enabled(InWorldSceneStage::Npcs));
    }

    #[test]
    fn terrain_loading_is_required_only_from_terrain_stage() {
        assert!(!terrain_is_required_for_loading(InWorldSceneStage::Empty));
        assert!(!terrain_is_required_for_loading(InWorldSceneStage::Skybox));
        assert!(terrain_is_required_for_loading(InWorldSceneStage::Terrain));
        assert!(terrain_is_required_for_loading(InWorldSceneStage::Ui));
    }

    #[test]
    fn unconfigured_stage_preserves_the_full_scene() {
        assert_eq!(effective_inworld_scene_stage(None), InWorldSceneStage::Ui);
    }
}
