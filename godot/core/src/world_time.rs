//! The client's time of day, from the server's `LoginSetTimeSpeed` (TrinityCore
//! `SMSG_LOGIN_SET_TIME_SPEED`, Player.cpp:24988-24996): the realm's local time advancing at
//! its speed. LightData keys the day in half-minutes, 0..2880.

/// Half-minutes in a day.
pub const HALF_MINUTES_PER_DAY: f64 = 2880.0;

/// The time of day in half-minutes, `elapsed_seconds` after a `LoginSetTimeSpeed` at
/// `second_of_day` with `speed` game minutes per real second.
pub fn half_minutes(second_of_day: u32, speed: f32, elapsed_seconds: f64) -> f32 {
    let start = f64::from(second_of_day) / 30.0;
    let advanced = elapsed_seconds * f64::from(speed) * 2.0;
    (start + advanced).rem_euclid(HALF_MINUTES_PER_DAY) as f32
}

/// Half-minutes the lit time of day steps by: one real second at the realm's
/// real-time speed (0.0166667 game minutes per second). Within a step the LightData
/// colours move by far less than one 8-bit level, so lighting re-samples, rebuilds the
/// sky cubemap and rebinds materials once per step instead of every frame.
pub const LIGHT_STEP_HALF_MINUTES: f32 = 1.0 / 32.0;

/// `minutes` (half-minutes) floored to the lit step.
pub fn light_half_minutes(minutes: f32) -> f32 {
    (minutes / LIGHT_STEP_HALF_MINUTES).floor() * LIGHT_STEP_HALF_MINUTES
}
