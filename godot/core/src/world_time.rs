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
