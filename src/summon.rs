//! Summon offers (Retail `C_SummonInfo`): the server's `SummonRequest` is the open
//! offer until it is answered; answers go out as `SummonResponse` on
//! `InteractionChannel`. The `CONFIRM_SUMMON` popup is `scenes::summon_popup`.

use std::collections::VecDeque;

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use shared::protocol::{InteractionChannel, SummonRequest, SummonResponse};

use crate::network_events::{register_message_handler, register_outgoing_handler};

/// An open summon offer.
#[derive(Clone, Debug, PartialEq)]
pub struct SummonOffer {
    /// `GetSummonConfirmSummoner`.
    pub summoner: String,
    /// AreaTable id of the summoner's zone (`GetSummonConfirmAreaName`).
    pub zone_id: u32,
    /// `Time::elapsed_secs_f64` when the server drops the offer.
    pub expires_at: f64,
}

impl SummonOffer {
    /// `GetSummonConfirmTimeLeft`, in seconds.
    pub fn time_left(&self, now: f64) -> f64 {
        (self.expires_at - now).max(0.0)
    }
}

#[derive(Resource, Default)]
pub struct SummonClientState {
    pub offer: Option<SummonOffer>,
    pending_answers: VecDeque<bool>,
}

impl SummonClientState {
    /// `ConfirmSummon` (accept) or `CancelSummon`: ends the offer and queues the answer.
    pub fn answer(&mut self, accept: bool) {
        if self.offer.take().is_some() {
            self.pending_answers.push_back(accept);
        }
    }

    /// Queued answers, in order (tests read what the UI answered).
    pub fn take_answers(&mut self) -> Vec<bool> {
        self.pending_answers.drain(..).collect()
    }
}

pub struct SummonPlugin;

impl Plugin for SummonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SummonClientState>();
        register_message_handler::<SummonRequest, _>(app, receive_summon_requests, |_| true);
        register_outgoing_handler(app, send_summon_answers, |world| {
            !world
                .resource::<SummonClientState>()
                .pending_answers
                .is_empty()
        });
    }
}

/// A new offer replaces the open one, as `CONFIRM_SUMMON` fires again.
fn receive_summon_requests(
    mut receivers: MessageReceivers<SummonRequest>,
    mut state: ResMut<SummonClientState>,
    time: Res<Time>,
) {
    for receiver in receivers.iter_mut() {
        for request in receiver.receive() {
            state.offer = Some(SummonOffer {
                summoner: request.summoner,
                zone_id: request.zone_id,
                expires_at: time.elapsed_secs_f64() + f64::from(request.time_left_ms) / 1000.0,
            });
        }
    }
}

fn send_summon_answers(
    mut state: ResMut<SummonClientState>,
    mut senders: MessageSenders<SummonResponse>,
) {
    while let Some(accept) = state.pending_answers.pop_front() {
        for mut sender in senders.iter_mut() {
            sender.send::<InteractionChannel>(SummonResponse { accept });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use game_engine::network_runtime::messages::{ConnectionSender, Inbox};

    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(SummonPlugin);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(1),
        ));
        app.update();
        app
    }

    fn deliver(app: &mut App, request: SummonRequest) {
        app.insert_resource(Inbox::new(vec![request]));
        crate::network_events::dispatch_incoming(app.world_mut());
    }

    #[test]
    fn a_request_opens_an_offer_that_expires_after_its_time_left() {
        let mut app = app();
        let now = app.world().resource::<Time>().elapsed_secs_f64();
        deliver(
            &mut app,
            SummonRequest {
                summoner: "Stonecaller".into(),
                zone_id: 40,
                time_left_ms: 120_000,
            },
        );
        let offer = app
            .world()
            .resource::<SummonClientState>()
            .offer
            .clone()
            .unwrap();
        assert_eq!(offer.summoner, "Stonecaller");
        assert_eq!(offer.zone_id, 40);
        assert_eq!(offer.time_left(now), 120.0);
        assert_eq!(offer.time_left(now + 150.0), 0.0);
    }

    #[test]
    fn answering_sends_one_response_and_ends_the_offer() {
        let mut app = app();
        deliver(
            &mut app,
            SummonRequest {
                summoner: "Stonecaller".into(),
                zone_id: 40,
                time_left_ms: 120_000,
            },
        );
        let (commands, sent) = mpsc::channel();
        app.insert_resource(ConnectionSender::new(Some(commands)));
        let mut state = app.world_mut().resource_mut::<SummonClientState>();
        state.answer(true);
        state.answer(false);
        assert!(state.offer.is_none());
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert_eq!(sent.try_iter().count(), 1, "one SummonResponse");
    }
}
