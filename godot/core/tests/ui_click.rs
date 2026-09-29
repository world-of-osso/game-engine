use game_engine_core::ui_click_data::{CLICK_VOLUME_SCALE, generate_button_click_samples};

#[test]
fn legacy_click_pcm_keeps_normalized_phase_and_envelope() {
    let samples = generate_button_click_samples();
    assert_eq!(samples.len(), 1_764);
    assert_eq!(samples[0], 0);
    for index in [1, 7, 123, 441, 881, 1763] {
        let t = index as f32 / 1764.0;
        let envelope = (1.0 - t).powf(4.0);
        let tone = (t * 2.0 * std::f32::consts::PI * 1300.0).sin() * 0.6;
        let tick = (t * 2.0 * std::f32::consts::PI * 2600.0).sin() * 0.2;
        assert_eq!(
            samples[index],
            ((tone + tick) * envelope * 10500.0) as i16,
            "sample {index}"
        );
    }
    assert_eq!(CLICK_VOLUME_SCALE, 0.55);
}
