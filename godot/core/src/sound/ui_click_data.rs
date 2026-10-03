/// Legacy UI click: phase uses normalized sample position, not elapsed seconds.
pub const CLICK_VOLUME_SCALE: f32 = 0.55;
pub const CLICK_SAMPLE_RATE: i32 = 44_100;

pub fn generate_button_click_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 40;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(4.0);
        let tone = (t * 2.0 * std::f32::consts::PI * 1_300.0).sin() * 0.6;
        let tick = (t * 2.0 * std::f32::consts::PI * 2_600.0).sin() * 0.2;
        samples.push(((tone + tick) * envelope * 10_500.0) as i16);
    }
    samples
}
