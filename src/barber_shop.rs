use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::CharacterAppearance;
use shared::protocol::{
    ApplyBarberShopChanges, BarberShopChannel, BarberShopStateUpdate, QueryBarberShopStatus,
};

use crate::auction_house_data::Money;
use crate::barber_shop_data::{BarberShopState, CUSTOMIZATIONS};
use crate::ipc::{BarberOption, Request, Response};
use crate::status::BarberShopStatusSnapshot;

#[derive(Resource, Default)]
pub struct BarberShopRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    queried_inworld: bool,
    shop_state: BarberShopState,
}

enum Action {
    Apply(CharacterAppearance),
}

pub struct BarberShopPlugin;

impl Plugin for BarberShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BarberShopRuntimeState>();
        app.add_systems(Update, request_barber_shop_status_on_enter_world);
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_barber_shop_updates);
    }
}

pub fn queue_ipc_request(
    runtime: &mut BarberShopRuntimeState,
    snapshot: &mut BarberShopStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::BarberStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::BarberSet { option, value } => {
            set_barber_option(runtime, snapshot, option.clone(), *value);
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::BarberReset => {
            runtime.shop_state.reset();
            sync_pending_snapshot(runtime, snapshot);
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::BarberApply => {
            runtime
                .pending_actions
                .push_back(Action::Apply(snapshot.pending_appearance));
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

fn request_barber_shop_status_on_enter_world(
    mut runtime: ResMut<BarberShopRuntimeState>,
    snapshot: Res<BarberShopStatusSnapshot>,
    mut senders: Query<&mut MessageSender<QueryBarberShopStatus>>,
) {
    if runtime.queried_inworld
        || snapshot.gold > 0
        || snapshot.current_appearance != CharacterAppearance::default()
    {
        return;
    }
    if send_all(&mut senders, QueryBarberShopStatus) {
        runtime.queried_inworld = true;
    }
}

fn send_pending_actions(
    mut runtime: ResMut<BarberShopRuntimeState>,
    mut senders: Query<&mut MessageSender<ApplyBarberShopChanges>>,
) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Apply(appearance) => {
                send_all(&mut senders, ApplyBarberShopChanges { appearance })
            }
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "barber shop is unavailable: not connected".into(),
            ));
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<BarberShopChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_barber_shop_updates(
    mut runtime: ResMut<BarberShopRuntimeState>,
    mut snapshot: ResMut<BarberShopStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<BarberShopStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_barber_shop_state_update(&mut runtime, &mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(format_status(&snapshot))
                };
                let _ = reply.send(response);
            }
        }
    }
}

pub fn apply_barber_shop_state_update(
    runtime: &mut BarberShopRuntimeState,
    snapshot: &mut BarberShopStatusSnapshot,
    update: BarberShopStateUpdate,
) {
    if let Some(server) = update.snapshot {
        runtime.shop_state.sync_from_appearance(server.appearance);
        snapshot.current_appearance = server.appearance;
        snapshot.pending_appearance = server.appearance;
        snapshot.gold = server.gold;
        snapshot.pending_cost = 0;
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut BarberShopRuntimeState) {
    *runtime = BarberShopRuntimeState::default();
}

fn set_barber_option(
    runtime: &mut BarberShopRuntimeState,
    snapshot: &mut BarberShopStatusSnapshot,
    option: BarberOption,
    value: u8,
) {
    let choice_index = clamp_choice_index(option_index(&option), value);
    if let Some(selection) = runtime.shop_state.selections.get_mut(option_index(&option)) {
        *selection = choice_index;
    }
    sync_pending_snapshot(runtime, snapshot);
}

fn sync_pending_snapshot(
    runtime: &BarberShopRuntimeState,
    snapshot: &mut BarberShopStatusSnapshot,
) {
    let pending = runtime
        .shop_state
        .preview_appearance(snapshot.current_appearance);
    snapshot.pending_appearance = pending;
    snapshot.pending_cost = barber_cost(snapshot.current_appearance, pending);
    snapshot.last_error = None;
}

fn barber_cost(current: CharacterAppearance, pending: CharacterAppearance) -> u32 {
    let changed = [
        current.hair_style != pending.hair_style,
        current.hair_color != pending.hair_color,
        current.facial_style != pending.facial_style,
        current.skin_color != pending.skin_color,
        current.face != pending.face,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count() as u32;
    changed * 10_000
}

fn option_index(option: &BarberOption) -> usize {
    match option {
        BarberOption::HairStyle => 0,
        BarberOption::HairColor => 1,
        BarberOption::FacialHair => 2,
        BarberOption::SkinColor => 3,
        BarberOption::Face => 4,
    }
}

fn clamp_choice_index(option_index: usize, value: u8) -> usize {
    let max = CUSTOMIZATIONS
        .get(option_index)
        .map(|def| def.choices.len().saturating_sub(1))
        .unwrap_or(0);
    usize::min(value as usize, max)
}

fn format_status(snapshot: &BarberShopStatusSnapshot) -> String {
    crate::ipc::format::format_barber_shop_status(snapshot)
}

pub fn option_value(appearance: CharacterAppearance, option_index: usize) -> &'static str {
    let value_index = match option_index {
        0 => appearance.hair_style,
        1 => appearance.hair_color,
        2 => appearance.facial_style,
        3 => appearance.skin_color,
        4 => appearance.face,
        _ => 0,
    } as usize;
    CUSTOMIZATIONS
        .get(option_index)
        .and_then(|def| def.choices.get(value_index))
        .copied()
        .unwrap_or("???")
}

pub fn format_cost(copper: u32) -> String {
    if copper == 0 {
        "Free".into()
    } else {
        Money(copper as u64).display()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::BarberShopSnapshot;

    #[test]
    fn state_update_populates_snapshot() {
        let mut runtime = BarberShopRuntimeState::default();
        let mut snapshot = BarberShopStatusSnapshot::default();

        apply_barber_shop_state_update(
            &mut runtime,
            &mut snapshot,
            BarberShopStateUpdate {
                snapshot: Some(BarberShopSnapshot {
                    appearance: CharacterAppearance {
                        sex: 0,
                        skin_color: 1,
                        face: 2,
                        eye_color: 3,
                        hair_style: 4,
                        hair_color: 5,
                        facial_style: 1,
                    },
                    gold: 80_000,
                }),
                message: Some("barber shop ready".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.gold, 80_000);
        assert_eq!(snapshot.pending_cost, 0);
        assert_eq!(snapshot.pending_appearance.hair_style, 4);
    }

    #[test]
    fn local_option_change_updates_pending_cost() {
        let mut runtime = BarberShopRuntimeState::default();
        let mut snapshot = BarberShopStatusSnapshot {
            current_appearance: CharacterAppearance::default(),
            pending_appearance: CharacterAppearance::default(),
            gold: 50_000,
            pending_cost: 0,
            last_server_message: None,
            last_error: None,
        };
        runtime
            .shop_state
            .sync_from_appearance(snapshot.current_appearance);

        set_barber_option(&mut runtime, &mut snapshot, BarberOption::HairStyle, 1);

        assert_eq!(snapshot.pending_appearance.hair_style, 1);
        assert_eq!(snapshot.pending_cost, 10_000);
    }
}
