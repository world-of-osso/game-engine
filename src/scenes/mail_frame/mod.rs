//! MailFrame scene: builds the Retail frame from [`MailState`], the bags and the
//! player's money; the mailbox interaction opens and closes its window (with the
//! backpack), closing the window ends the interaction. Clicks and the Send Mail
//! edit boxes become [`MailRequest`]s; opening a mail marks it read; taking a
//! C.O.D. item asks `COD_CONFIRMATION` and deleting a mail that still holds
//! something asks `DELETE_MAIL` / `DELETE_MONEY` first (MailFrame.lua:866-921).
//! Right-clicking a bag item while Send Mail shows attaches it (`scenes::bag_frame`).

mod view;

use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::bag_data::InventoryState;
use game_engine::bank_data::money_text;
use game_engine::mail_data::{MailDraft, MailRequest, MailState, MailTab, SendMoneyMode};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::screens::mail_frame_component::{
    self as mail_ui, MailFrameState, mail_frame_screen,
};
use shared::protocol::MailAction;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking_quests::NpcInteractionRequest;
use crate::scenes::frame_input::{self, EditBoxSpec, Pointer};
use crate::scenes::static_popup::StaticPopupSystems;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

pub use view::mail_frame_state;

/// `StaticPopupDialogs["COD_CONFIRMATION"]`, `["DELETE_MAIL"]`, `["DELETE_MONEY"]`.
pub const COD_POPUP: &str = "COD_CONFIRMATION";
pub const DELETE_MAIL_POPUP: &str = "DELETE_MAIL";
pub const DELETE_MONEY_POPUP: &str = "DELETE_MONEY";

struct MailFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for MailFrameRes {}
unsafe impl Sync for MailFrameRes {}

#[derive(Resource)]
struct MailFrameWrap(MailFrameRes, MailFrameState);

/// The request a confirmation popup stands for.
#[derive(Resource, Default)]
struct PendingConfirm(Option<MailRequest>);

pub struct MailFramePlugin;

impl Plugin for MailFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>()
            .init_resource::<PopupStack>()
            .init_resource::<PendingConfirm>()
            .add_message::<MailRequest>()
            .add_message::<PopupResult>()
            .add_message::<NpcInteractionRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_mail_frame.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_mail_frame);
        app.add_systems(
            Update,
            (
                handle_mail_clicks,
                handle_mail_keyboard,
                confirm_mail_popups,
                clear_sent_form,
                sync_mail_window,
                sync_mail_frame,
            )
                .chain()
                .after(StaticPopupSystems)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Everything the frame shows.
#[derive(SystemParam)]
struct MailView<'w> {
    mail: Res<'w, MailState>,
    manager: Res<'w, WindowManager>,
    inventory: Option<Res<'w, InventoryState>>,
    stats: Option<Res<'w, game_engine::status::CharacterStatsSnapshot>>,
}

impl MailView<'_> {
    fn state(&self) -> MailFrameState {
        let empty = InventoryState::default();
        let inventory = self.inventory.as_deref().unwrap_or(&empty);
        let money = self.stats.as_ref().map_or(0, |stats| stats.gold);
        mail_frame_state(
            &self.mail,
            inventory,
            self.manager.is_open(WindowId::Mail),
            money,
        )
    }
}

fn build_mail_frame(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    view: MailView,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = view.state();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(mail_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(MailFrameWrap(MailFrameRes { screen, shared }, state));
}

fn teardown_mail_frame(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    wrap: Option<ResMut<MailFrameWrap>>,
) {
    if let Some(mut wrap) = wrap {
        wrap.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<MailFrameWrap>();
}

fn sync_mail_frame(mut ui: ResMut<UiState>, wrap: Option<ResMut<MailFrameWrap>>, view: MailView) {
    let Some(mut wrap) = wrap else { return };
    let state = view.state();
    if wrap.1 == state {
        return;
    }
    wrap.1 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// The Send Mail edit boxes with their Retail `letters` limits.
fn edit_boxes() -> Vec<EditBoxSpec> {
    let mut specs = vec![
        EditBoxSpec {
            name: mail_ui::TO_BOX,
            letters: 77,
            digits_only: false,
        },
        EditBoxSpec {
            name: mail_ui::SUBJECT_BOX,
            letters: shared::protocol::MAIL_SUBJECT_MAX_LETTERS,
            digits_only: false,
        },
        EditBoxSpec {
            name: mail_ui::BODY_BOX,
            letters: shared::protocol::MAIL_BODY_MAX_LETTERS,
            digits_only: false,
        },
    ];
    specs.extend(frame_input::money_specs(mail_ui::MONEY_BOXES));
    specs
}

/// What a click did besides changing [`MailState`].
#[derive(Debug, Default, PartialEq)]
pub struct MailClick {
    pub requests: Vec<MailRequest>,
    pub popup: Option<(PopupSpec, MailRequest)>,
    /// Edit box texts to set.
    pub texts: Vec<(&'static str, String)>,
    pub close: bool,
}

/// The typed Send Mail form (`SendMailFrame_SendMail`): money goes out as money or
/// as C.O.D.; an empty subject takes the first attachment's name.
pub fn draft(
    registry: &FrameRegistry,
    mail: &MailState,
    inventory: &InventoryState,
) -> Option<MailDraft> {
    let recipient = frame_input::text(registry, mail_ui::TO_BOX)
        .trim()
        .to_string();
    if recipient.is_empty() {
        return None;
    }
    let mut subject = frame_input::text(registry, mail_ui::SUBJECT_BOX);
    if subject.trim().is_empty() {
        subject = mail
            .attachments
            .first()
            .and_then(|guid| view::bag_item(inventory, *guid))
            .map(|slot| slot.name.clone())
            .unwrap_or_default();
    }
    let money = frame_input::money(registry, mail_ui::MONEY_BOXES);
    let (money, cod) = match mail.money_mode {
        SendMoneyMode::Money => (money, 0),
        SendMoneyMode::Cod => (0, money),
    };
    Some(MailDraft {
        recipient,
        subject,
        body: frame_input::text(registry, mail_ui::BODY_BOX),
        attachments: mail.attachments.clone(),
        money,
        cod,
    })
}

fn act(mail_id: u64, action: MailAction) -> MailRequest {
    MailRequest::Act { mail_id, action }
}

/// `OpenAllMail`: every mail without C.O.D. gives up its money and items.
fn open_all(mail: &MailState) -> Vec<MailRequest> {
    mail.mails()
        .iter()
        .filter(|header| header.cod == 0)
        .flat_map(|header| {
            let money = (header.money > 0).then(|| act(header.mail_id, MailAction::TakeMoney));
            let items = header.attachments.iter().map(|attached| {
                act(
                    header.mail_id,
                    MailAction::TakeAttachment {
                        slot: attached.slot,
                    },
                )
            });
            money.into_iter().chain(items)
        })
        .collect()
}

fn confirm(key: &str, text: String, request: MailRequest) -> Option<(PopupSpec, MailRequest)> {
    Some((
        PopupSpec {
            key: key.into(),
            text,
            accept_label: "Accept".into(),
            cancel_label: Some("Cancel".into()),
            timeout: None,
        },
        request,
    ))
}

/// Open-mail button clicks (MF.lua:840-921).
fn open_mail_click(action: &str, mail: &mut MailState) -> MailClick {
    let mut click = MailClick::default();
    let Some(open) = mail.opened().cloned() else {
        return click;
    };
    let id = open.mail_id;
    if action == mail_ui::ACTION_OPEN_CLOSE {
        mail.open_mail = None;
    } else if action == mail_ui::ACTION_TAKE_MONEY {
        click.requests.push(act(id, MailAction::TakeMoney));
    } else if let Some(slot) = parse(action, mail_ui::ACTION_TAKE_ITEM_PREFIX) {
        let take = act(id, MailAction::TakeAttachment { slot });
        if open.cod > 0 {
            let text = format!("Accepting this item will cost:\n{}", money_text(open.cod));
            click.popup = confirm(COD_POPUP, text, take);
        } else {
            click.requests.push(take);
        }
    } else if action == mail_ui::ACTION_REPLY && open.from_player {
        mail.tab = MailTab::Send;
        click.texts = vec![
            (mail_ui::TO_BOX, open.sender.clone()),
            (mail_ui::SUBJECT_BOX, format!("RE: {}", open.subject)),
        ];
    } else if action == mail_ui::ACTION_DELETE {
        click = delete_click(&open);
        mail.open_mail = None;
    }
    click
}

fn delete_click(open: &shared::protocol::MailHeader) -> MailClick {
    let id = open.mail_id;
    let mut click = MailClick::default();
    if !open.can_delete() {
        click.requests.push(act(id, MailAction::Return));
    } else if let Some(item) = open.attachments.first() {
        let text = format!("Deleting this mail will also destroy {}", item.name);
        click.popup = confirm(DELETE_MAIL_POPUP, text, act(id, MailAction::Delete));
    } else if open.money > 0 {
        let text = format!(
            "Deleting this mail will also destroy:\n{}",
            money_text(open.money)
        );
        click.popup = confirm(DELETE_MONEY_POPUP, text, act(id, MailAction::Delete));
    } else {
        click.requests.push(act(id, MailAction::Delete));
    }
    click
}

fn parse<T: std::str::FromStr>(action: &str, prefix: &str) -> Option<T> {
    action.strip_prefix(prefix)?.parse().ok()
}

/// One frame click: tabs, inbox rows and paging, the Send Mail form, the open mail.
pub fn mail_click(
    action: &str,
    mail: &mut MailState,
    registry: &FrameRegistry,
    inventory: &InventoryState,
) -> MailClick {
    let mut click = MailClick::default();
    match action {
        mail_ui::ACTION_CLOSE => click.close = true,
        mail_ui::ACTION_TAB_INBOX => mail.tab = MailTab::Inbox,
        mail_ui::ACTION_TAB_SEND => mail.tab = MailTab::Send,
        mail_ui::ACTION_PREV => mail.prev_page(),
        mail_ui::ACTION_NEXT => mail.next_page(),
        mail_ui::ACTION_OPEN_ALL => click.requests = open_all(mail),
        mail_ui::ACTION_MODE_MONEY => mail.money_mode = SendMoneyMode::Money,
        mail_ui::ACTION_MODE_COD => mail.money_mode = SendMoneyMode::Cod,
        mail_ui::ACTION_SEND => {
            if let Some(draft) = draft(registry, mail, inventory) {
                click.requests.push(MailRequest::Send(draft));
            }
        }
        mail_ui::ACTION_SEND_CANCEL => {
            mail.attachments.clear();
            click.texts = cleared_form();
        }
        _ => {
            if let Some(mail_id) = parse::<u64>(action, mail_ui::ACTION_OPEN_PREFIX) {
                // `InboxFrame_OnClick`: opening an unread mail reads it.
                if mail.find(mail_id).is_some_and(|header| !header.read) {
                    click.requests.push(act(mail_id, MailAction::MarkRead));
                }
                mail.open_mail = Some(mail_id);
            } else if let Some(index) = parse::<usize>(action, mail_ui::ACTION_ATTACHMENT_PREFIX) {
                mail.detach(index);
            } else {
                click = open_mail_click(action, mail);
            }
        }
    }
    click
}

fn cleared_form() -> Vec<(&'static str, String)> {
    [mail_ui::TO_BOX, mail_ui::SUBJECT_BOX, mail_ui::BODY_BOX]
        .into_iter()
        .chain(mail_ui::MONEY_BOXES.all())
        .map(|name| (name, String::new()))
        .collect()
}

#[derive(SystemParam)]
struct MailInputs<'w> {
    mail: ResMut<'w, MailState>,
    manager: ResMut<'w, WindowManager>,
    popups: ResMut<'w, PopupStack>,
    pending: ResMut<'w, PendingConfirm>,
    inventory: Option<Res<'w, InventoryState>>,
}

fn handle_mail_clicks(
    pointer: Pointer,
    mut ui: ResMut<UiState>,
    mut inputs: MailInputs,
    mut requests: MessageWriter<MailRequest>,
) {
    if !inputs.mail.is_open() {
        return;
    }
    let Some((_, frame_id)) = pointer.click(&ui) else {
        return;
    };
    let specs = edit_boxes();
    if frame_input::spec_of(&ui.registry, &specs, frame_id).is_some() {
        frame_input::set_focus(&mut ui, Some(frame_id));
        return;
    }
    if frame_input::focused_spec(&ui, &specs).is_some() {
        frame_input::set_focus(&mut ui, None);
    }
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    if !action.starts_with("mail_") {
        return;
    }
    let empty = InventoryState::default();
    let inventory = inputs.inventory.as_deref().unwrap_or(&empty);
    let click = mail_click(&action, &mut inputs.mail, &ui.registry, inventory);
    for (name, text) in &click.texts {
        frame_input::set_text(&mut ui.registry, name, text);
    }
    if let Some((popup, request)) = click.popup {
        inputs.popups.push(popup);
        inputs.pending.0 = Some(request);
    }
    if click.close {
        inputs.manager.close(WindowId::Mail);
    }
    for request in click.requests {
        requests.write(request);
    }
}

fn handle_mail_keyboard(mut key_events: MessageReader<KeyboardInput>, mut ui: ResMut<UiState>) {
    frame_input::type_into(&mut key_events, &mut ui, &edit_boxes());
}

fn confirm_mail_popups(
    mut results: MessageReader<PopupResult>,
    mut pending: ResMut<PendingConfirm>,
    mut requests: MessageWriter<MailRequest>,
) {
    for result in results.read() {
        if ![COD_POPUP, DELETE_MAIL_POPUP, DELETE_MONEY_POPUP].contains(&result.key.as_str()) {
            continue;
        }
        let request = pending.0.take();
        if result.outcome == PopupOutcome::Accepted
            && let Some(request) = request
        {
            requests.write(request);
        }
    }
}

/// `MAIL_SEND_SUCCESS` clears the Send Mail form (`SendMailFrame_Reset`).
fn clear_sent_form(mail: Res<MailState>, mut ui: ResMut<UiState>, mut seen: Local<u32>) {
    if mail.sent == *seen {
        return;
    }
    *seen = mail.sent;
    for (name, text) in cleared_form() {
        frame_input::set_text(&mut ui.registry, name, &text);
    }
}

/// The mailbox opening opens the window and the backpack; closing the window ends
/// the interaction (`CloseInteraction`); the interaction ending closes both.
fn sync_mail_window(
    mut manager: ResMut<WindowManager>,
    mut mail: ResMut<MailState>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut open: Local<Option<u64>>,
) {
    let window_open = manager.is_open(WindowId::Mail);
    let object = mail.object;
    if object.is_some() && *open != object {
        manager.open(WindowId::Mail);
        manager.open(WindowId::Bag(0));
        *open = object;
    } else if let Some(npc) = object.filter(|_| !window_open) {
        requests.write(NpcInteractionRequest::Close { npc });
        manager.close(WindowId::Bag(0));
        mail.close();
        *open = None;
    } else if object.is_none() && open.is_some() {
        manager.close(WindowId::Mail);
        manager.close(WindowId::Bag(0));
        *open = None;
    }
}

#[cfg(test)]
mod tests;
