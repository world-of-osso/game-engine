use game_engine_core::world_time::half_minutes;

#[test]
fn server_local_time_is_the_lightdata_half_minute_of_the_day() {
    // 15:30:20 local is 15.5 hours and 20 s: 1860 half-minutes and two thirds.
    let at_receipt = half_minutes(15 * 3600 + 30 * 60 + 20, 0.016_666_67, 0.0);
    assert!((at_receipt - 1860.666_7).abs() < 1e-3, "{at_receipt}");
}

#[test]
fn game_time_runs_at_real_time_and_wraps_at_midnight() {
    // TimeSpeed 0.01666667 game minutes per second: an hour later is 120 half-minutes on.
    let later = half_minutes(12 * 3600, 0.016_666_67, 3600.0);
    assert!((later - 1560.0).abs() < 1e-2, "{later}");
    // 23:59 plus two minutes is 00:01.
    let wrapped = half_minutes(23 * 3600 + 59 * 60, 0.016_666_67, 120.0);
    assert!((wrapped - 2.0).abs() < 1e-2, "{wrapped}");
}
