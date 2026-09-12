use super::*;
use bevy::ecs::system::RunSystemOnce;
use std::{path::Path, time::Duration};

fn cauldron_app() -> (App, Entity) {
    let model = crate::asset::m2::load_m2_uncached(Path::new("data/models/4238519.m2"), &[0; 3])
        .expect("cached cauldron must load");
    assert_eq!(model.sequences[0].duration, 3333);
    assert_eq!(model.global_sequences[1], 3333);
    assert_eq!(model.lights[0].diffuse_intensity.global_sequence, -1);
    assert_eq!(model.lights[1].diffuse_intensity.global_sequence, 1);
    let mut app = App::new();
    app.init_resource::<Time>();
    let owner = app.world_mut().spawn_empty().id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            crate::m2_spawn::spawn_model_point_lights(
                &mut commands,
                &model.lights,
                &None,
                owner,
                owner,
            );
        })
        .unwrap();
    (app, owner)
}

fn intensities(app: &mut App) -> Vec<f32> {
    let mut lights: Vec<_> = app
        .world_mut()
        .query::<(&RuntimeM2PointLight, &PointLight)>()
        .iter(app.world())
        .map(|(runtime, light)| (runtime.light.bone_index, light.intensity))
        .collect();
    lights.sort_by_key(|(bone, _)| *bone);
    assert_eq!(lights.len(), 2);
    lights.into_iter().map(|(_, intensity)| intensity).collect()
}

fn advance_lights(app: &mut App, milliseconds: u64) -> Vec<f32> {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(milliseconds));
    app.world_mut().run_system_once(sync_model_lights).unwrap();
    intensities(app)
}

#[test]
fn m2_light_clock_static_cauldron_advances_and_repeats_authored_periods() {
    let (mut app, _) = cauldron_app();
    let initial = intensities(&mut app);
    let sampled = advance_lights(&mut app, 500);
    for index in 0..2 {
        assert!(
            (sampled[index] - initial[index]).abs() > 1.0,
            "light {index} must advance without a skeletal animation player"
        );
    }
    let repeated = advance_lights(&mut app, 3333);
    for index in 0..2 {
        assert!(
            (sampled[index] - repeated[index]).abs() < 0.01,
            "light {index} must repeat its authored 3333ms period"
        );
    }
}

#[test]
fn m2_light_clock_global_track_is_independent_of_player_sequence_and_time() {
    let (mut app, owner) = cauldron_app();
    app.world_mut().entity_mut(owner).insert(M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
    });
    let initial = intensities(&mut app);
    let sampled = advance_lights(&mut app, 500);
    assert_eq!(
        sampled[0], initial[0],
        "local track follows the paused player"
    );
    assert!(
        (sampled[1] - initial[1]).abs() > 1.0,
        "global track must still advance"
    );
    app.world_mut()
        .get_mut::<M2AnimPlayer>(owner)
        .unwrap()
        .current_seq_idx = 1;
    let repeated = advance_lights(&mut app, 3333);
    assert!(
        (sampled[1] - repeated[1]).abs() < 0.01,
        "global track uses sequence zero and its own period, not player sequence one"
    );
}
