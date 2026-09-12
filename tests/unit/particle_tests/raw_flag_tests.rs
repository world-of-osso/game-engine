use super::*;

fn load_authored_mist(fdid: u32) -> M2ParticleEmitter {
    let path = format!("data/models/{fdid}.m2");
    let model = crate::asset::m2::load_m2_uncached(Path::new(&path), &[0; 3])
        .expect("authored mist model must load");
    assert_eq!(model.particle_emitters.len(), 1);
    model.particle_emitters.into_iter().next().unwrap()
}

#[test]
fn raw_particle_flags_mist_particles_use_bone_parent_and_scale() {
    let root = Entity::from_bits(11);
    let bone = Entity::from_bits(22);
    for (fdid, flags) in [(1028937, 0x7483_0220), (2904370, 0x6693_1220)] {
        let emitter = load_authored_mist(fdid);
        assert_eq!(emitter.flags, flags, "fixture raw flags changed: {fdid}");
        assert_eq!(emitter_parent_entity(&emitter, Some(bone), root), bone);
        assert_eq!(emitter_scale_source(&emitter, Some(bone), root), bone);
        assert!(emitter_uses_bone_scale(&emitter));
        assert!(!emitter_uses_dynamic_wind(&emitter));
    }
}

#[test]
fn raw_particle_flags_both_mist_atlases_choose_random_cells() {
    for fdid in [1028937, 2904370] {
        let emitter = load_authored_mist(fdid);
        assert!(
            matches!(
                flipbook_sprite_mode(&emitter),
                Some(FlipbookSpriteMode::RandomCell)
            ),
            "raw random-frame bit must apply to mist {fdid}"
        );
    }
}

#[test]
fn raw_particle_flags_static_wind_needs_no_enable_bit() {
    let mut emitter = sample_emitter();
    emitter.flags = 0;
    emitter.wind_vector = [1.0, 2.0, 3.0];
    emitter.wind_time = 2.0;
    assert!(has_authored_wind(&emitter));
    assert!(!emitter_uses_dynamic_wind(&emitter));
    assert_eq!(wind_accel_bevy(&emitter, 1.5), Vec3::new(1.5, 4.5, -3.0));
}

#[test]
fn raw_particle_flags_dynamic_wind_uses_high_bit_alone() {
    const RAW_DYNAMIC_WIND: u32 = 0x8000_0000;
    let mut emitter = sample_emitter();
    emitter.flags = RAW_DYNAMIC_WIND;
    emitter.wind_vector = [1.0, 2.0, 3.0];
    emitter.wind_time = 2.0;
    assert!(emitter_uses_dynamic_wind(&emitter));
    assert!(!has_authored_wind(&emitter));
}

#[test]
fn raw_particle_flags_multitexture_does_not_disable_density_scaling() {
    const RAW_MULTITEXTURE: u32 = 0x1000_0000;
    const RAW_NO_GLOBAL_SCALE: u32 = 0x0200_0000;
    let mut emitter = sample_emitter();
    emitter.flags = RAW_MULTITEXTURE;
    assert_eq!(scaled_emission_rate(&emitter, 0.5), 10.0);
    emitter.flags |= RAW_NO_GLOBAL_SCALE;
    assert_eq!(scaled_emission_rate(&emitter, 0.5), 20.0);
}
