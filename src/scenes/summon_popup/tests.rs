use std::time::Duration;

use game_engine::summon::SummonOffer;

use super::*;

const STONECALLER_IN_WESTFALL: &str =
    "Stonecaller wants to summon you to Westfall. The spell will be canceled in";

/// The popup systems at 200 ms frames (under `Time<Virtual>`'s 250 ms max delta), with `StaticPopupPlugin`'s timeout tick, a local
/// player and a 120 s offer from Westfall (zone 40).
fn app() -> (App, Entity) {
    game_engine::world_db::import_zone_name_cache().expect("import zone name cache");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<SummonClientState>()
        .init_resource::<PopupStack>()
        .add_message::<PopupResult>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(200),
        ))
        .add_systems(
            Update,
            (
                |time: Res<Time>,
                 mut stack: ResMut<PopupStack>,
                 mut results: MessageWriter<PopupResult>| {
                    stack.tick(time.delta());
                    results.write_batch(stack.drain_results());
                },
                answer_summon_popup,
                sync_summon_popup,
            )
                .chain(),
        );
    let player = app
        .world_mut()
        .spawn((LocalPlayer, CombatStatus(false)))
        .id();
    app.update();
    let now = app.world().resource::<Time>().elapsed_secs_f64();
    app.world_mut().resource_mut::<SummonClientState>().offer = Some(SummonOffer {
        summoner: "Stonecaller".into(),
        zone_id: 40,
        expires_at: now + 120.0,
    });
    (app, player)
}

fn popup(app: &App) -> Option<game_engine::ui::popup::PopupEntry> {
    app.world()
        .resource::<PopupStack>()
        .visible()
        .into_iter()
        .next()
}

fn advance(app: &mut App, seconds: usize) {
    for _ in 0..seconds * 5 {
        app.update();
    }
}

#[test]
fn expiry_text_counts_minutes_rounded_up_then_seconds() {
    assert_eq!(
        confirm_summon_text("Stonecaller", "Westfall", 120.0),
        format!("{STONECALLER_IN_WESTFALL} 2 Minutes.")
    );
    assert_eq!(
        confirm_summon_text("Stonecaller", "Westfall", 60.5),
        format!("{STONECALLER_IN_WESTFALL} 2 Minutes.")
    );
    assert_eq!(
        confirm_summon_text("Stonecaller", "Westfall", 60.0),
        format!("{STONECALLER_IN_WESTFALL} 1 Minute.")
    );
    assert_eq!(
        confirm_summon_text("Stonecaller", "Westfall", 59.2),
        format!("{STONECALLER_IN_WESTFALL} 1 Minute.")
    );
    assert_eq!(
        confirm_summon_text("Stonecaller", "Westfall", 58.5),
        format!("{STONECALLER_IN_WESTFALL} 59 Seconds.")
    );
    assert_eq!(
        confirm_summon_text("Stonecaller", "Westfall", 0.4),
        format!("{STONECALLER_IN_WESTFALL} 1 Second.")
    );
}

#[test]
fn an_offer_shows_confirm_summon_with_a_live_countdown() {
    let (mut app, _) = app();
    app.update();
    let shown = popup(&app).expect("CONFIRM_SUMMON");
    assert_eq!(shown.spec.key, CONFIRM_SUMMON_POPUP);
    assert_eq!(
        shown.spec.text,
        format!("{STONECALLER_IN_WESTFALL} 2 Minutes.")
    );
    assert_eq!(
        (
            shown.spec.accept_label.as_str(),
            shown.spec.cancel_label.as_deref()
        ),
        ("Accept", Some("Cancel"))
    );
    advance(&mut app, 70);
    assert_eq!(
        popup(&app).unwrap().spec.text,
        format!("{STONECALLER_IN_WESTFALL} 50 Seconds.")
    );
}

#[test]
fn accept_is_disabled_in_combat_and_accepting_after_combat_confirms() {
    let (mut app, player) = app();
    app.world_mut()
        .entity_mut(player)
        .insert(CombatStatus(true));
    app.update();
    assert!(!popup(&app).unwrap().accept_enabled);
    app.world_mut().resource_mut::<PopupStack>().accept_top();
    app.update();
    assert!(popup(&app).is_some(), "Accept ignored in combat");

    app.world_mut()
        .entity_mut(player)
        .insert(CombatStatus(false));
    app.update();
    app.world_mut().resource_mut::<PopupStack>().accept_top();
    app.update();
    assert!(popup(&app).is_none());
    let mut state = app.world_mut().resource_mut::<SummonClientState>();
    assert!(state.offer.is_none());
    assert_eq!(state.take_answers(), vec![true]);
}

#[test]
fn the_countdown_running_out_cancels_the_summon() {
    let (mut app, _) = app();
    advance(&mut app, 121);
    assert!(popup(&app).is_none());
    let mut state = app.world_mut().resource_mut::<SummonClientState>();
    assert!(state.offer.is_none());
    assert_eq!(state.take_answers(), vec![false]);
}
