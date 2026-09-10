use bevy::prelude::*;

use game_engine::camera_control::WowCamera;

pub(super) fn set_camera_direction(
    in_world: bool,
    cameras: &mut Query<(&Camera, &mut WowCamera)>,
    yaw_degrees: Option<f32>,
    pitch_degrees: Option<f32>,
) -> Result<String, String> {
    if !in_world {
        return Err("camera direction requires InWorld".into());
    }
    let mut active = cameras.iter_mut().filter(|(camera, _)| camera.is_active);
    let Some((_, mut camera)) = active.next() else {
        return Err("no active in-world camera".into());
    };
    if active.next().is_some() {
        return Err("multiple active in-world cameras".into());
    }
    camera.set_direction_degrees(yaw_degrees, pitch_degrees)?;
    Ok(format!(
        "camera yaw={:.3} pitch={:.3} degrees",
        camera.yaw.to_degrees(),
        camera.pitch.to_degrees()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    fn request(world: &mut World, in_world: bool) -> Result<String, String> {
        let mut state = SystemState::<Query<(&Camera, &mut WowCamera)>>::new(world);
        let mut cameras = state.get_mut(world).unwrap();
        set_camera_direction(in_world, &mut cameras, Some(90.0), Some(60.0))
    }

    #[test]
    fn camera_direction_targets_active_camera_only() {
        let mut world = World::new();
        let inactive = world
            .spawn((
                Camera {
                    is_active: false,
                    ..default()
                },
                WowCamera::default(),
            ))
            .id();
        let active = world.spawn((Camera::default(), WowCamera::default())).id();
        let response = request(&mut world, true).unwrap();
        assert_eq!(response, "camera yaw=90.000 pitch=60.000 degrees");
        assert!((world.get::<WowCamera>(active).unwrap().pitch.to_degrees() - 60.0).abs() < 1e-5);
        assert_eq!(world.get::<WowCamera>(inactive).unwrap().pitch, -0.3);
    }

    #[test]
    fn camera_direction_rejects_wrong_scene_missing_and_ambiguous_cameras() {
        let mut world = World::new();
        assert_eq!(
            request(&mut world, true).unwrap_err(),
            "no active in-world camera"
        );
        let camera = world.spawn((Camera::default(), WowCamera::default())).id();
        assert_eq!(
            request(&mut world, false).unwrap_err(),
            "camera direction requires InWorld"
        );
        world.spawn((Camera::default(), WowCamera::default()));
        assert_eq!(
            request(&mut world, true).unwrap_err(),
            "multiple active in-world cameras"
        );
        assert_eq!(world.get::<WowCamera>(camera).unwrap().pitch, -0.3);
    }
}
