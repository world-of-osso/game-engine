//! Bank and guild bank frames (Wide windows): built from [`BankState`] /
//! [`GuildBankState`] and the player's money; opened and closed with the banker or
//! Guild Vault interaction (with the backpack, Retail `OpenAllBags`); clicks and edit
//! box typing turn into [`BankRequest`] / [`GuildBankRequest`]. Right-clicking a bag
//! item deposits it (`scenes::bag_frame`).

mod actions;
mod view;

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::bank_data::{BankRequest, BankState, GuildBankRequest, GuildBankState};
use game_engine::status::CharacterStatsSnapshot;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupStack};
use game_engine::ui::screens::bank_frame_component::{
    self as bank_ui, BankFrameState, bank_frame_screen,
};
use game_engine::ui::screens::guild_bank_frame_component::{
    self as guild_ui, GuildBankFrameState, guild_bank_frame_screen,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking_quests::NpcInteractionRequest;
use crate::ui_input::{mutate_editbox_from_key, walk_up_for_onclick};
use crate::ui_input_mode::focused_editbox;
use crate::window_manager::{WindowId, WindowManager};

use actions::{BUY_BANK_TAB_POPUP, BUY_GUILD_BANK_TAB_POPUP, InputTexts, Outcome};

struct FrameScreen {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for FrameScreen {}
unsafe impl Sync for FrameScreen {}

#[derive(Resource)]
struct BankFrameScreen(FrameScreen, BankFrameState);

#[derive(Resource)]
struct GuildBankFrameScreen(FrameScreen, GuildBankFrameState);

pub struct BankFramePlugin;

impl Plugin for BankFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BankState>()
            .init_resource::<GuildBankState>()
            .init_resource::<PopupStack>()
            .add_message::<BankRequest>()
            .add_message::<GuildBankRequest>()
            .add_message::<PopupResult>()
            .add_message::<NpcInteractionRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_bank_frames.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_bank_frames);
        app.add_systems(
            Update,
            (
                handle_bank_clicks,
                handle_bank_keyboard,
                confirm_tab_purchases,
                sync_bank_windows,
                sync_bank_frames,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Everything the frames show besides their own state.
#[derive(SystemParam)]
struct BankView<'w> {
    bank: Res<'w, BankState>,
    guild: Res<'w, GuildBankState>,
    manager: Res<'w, WindowManager>,
    stats: Option<Res<'w, CharacterStatsSnapshot>>,
}

impl BankView<'_> {
    fn money(&self) -> u64 {
        self.stats.as_ref().map_or(0, |stats| stats.gold)
    }

    fn bank_state(&self) -> BankFrameState {
        view::bank_frame_state(
            &self.bank,
            self.manager.is_open(WindowId::Bank),
            self.money(),
        )
    }

    fn guild_state(&self) -> GuildBankFrameState {
        view::guild_bank_frame_state(
            &self.guild,
            self.manager.is_open(WindowId::GuildBank),
            self.money(),
        )
    }
}

fn frame_screen<S: Clone + Send + Sync + 'static>(
    build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
    state: S,
    registry: &mut FrameRegistry,
) -> FrameScreen {
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut screen = Screen::new(build);
    screen.sync(&shared, registry);
    FrameScreen { screen, shared }
}

fn build_bank_frames(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    view: BankView,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let bank = view.bank_state();
    let guild = view.guild_state();
    let bank_screen = frame_screen(bank_frame_screen, bank.clone(), &mut ui.registry);
    let guild_screen = frame_screen(guild_bank_frame_screen, guild.clone(), &mut ui.registry);
    commands.insert_resource(BankFrameScreen(bank_screen, bank));
    commands.insert_resource(GuildBankFrameScreen(guild_screen, guild));
}

fn teardown_bank_frames(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    bank: Option<ResMut<BankFrameScreen>>,
    guild: Option<ResMut<GuildBankFrameScreen>>,
) {
    if let Some(mut bank) = bank {
        bank.0.screen.teardown(&mut ui.registry);
    }
    if let Some(mut guild) = guild {
        guild.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<BankFrameScreen>();
    commands.remove_resource::<GuildBankFrameScreen>();
}

fn sync_bank_frames(
    mut ui: ResMut<UiState>,
    bank_screen: Option<ResMut<BankFrameScreen>>,
    guild_screen: Option<ResMut<GuildBankFrameScreen>>,
    view: BankView,
) {
    if let Some(mut bank_screen) = bank_screen {
        let state = view.bank_state();
        if bank_screen.1 != state {
            bank_screen.1 = state.clone();
            let screen = &mut bank_screen.0;
            screen.shared.insert(state);
            screen.screen.sync(&screen.shared, &mut ui.registry);
        }
    }
    if let Some(mut guild_screen) = guild_screen {
        let state = view.guild_state();
        if guild_screen.1 != state {
            guild_screen.1 = state.clone();
            let screen = &mut guild_screen.0;
            screen.shared.insert(state);
            screen.screen.sync(&screen.shared, &mut ui.registry);
        }
    }
}

/// An opened frame opens its window and the backpack; the window manager closing it
/// ends the interaction with `CloseInteraction`; the server ending it closes the
/// window. Either way the backpack closes (Retail `CloseAllBags`).
fn sync_bank_windows(
    mut manager: ResMut<WindowManager>,
    mut bank: ResMut<BankState>,
    mut guild: ResMut<GuildBankState>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut open: Local<(Option<u64>, Option<u64>)>,
) {
    sync_window(
        &mut manager,
        WindowId::Bank,
        bank.npc,
        &mut open.0,
        &mut requests,
    );
    if bank.npc.is_some() && open.0.is_none() {
        bank.close();
    }
    sync_window(
        &mut manager,
        WindowId::GuildBank,
        guild.object,
        &mut open.1,
        &mut requests,
    );
    if guild.object.is_some() && open.1.is_none() {
        guild.close();
    }
}

fn sync_window(
    manager: &mut WindowManager,
    window: WindowId,
    target: Option<u64>,
    open: &mut Option<u64>,
    requests: &mut MessageWriter<NpcInteractionRequest>,
) {
    let window_open = manager.is_open(window);
    if target.is_some() && *open != target {
        manager.open(window);
        manager.open(WindowId::Bag(0));
        *open = target;
    } else if let Some(npc) = target.filter(|_| !window_open) {
        requests.write(NpcInteractionRequest::Close { npc });
        manager.close(WindowId::Bag(0));
        *open = None;
    } else if target.is_none() && open.is_some() {
        manager.close(window);
        manager.close(WindowId::Bag(0));
        *open = None;
    }
}

#[derive(SystemParam)]
struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    /// The button pressed this frame and the frame under the cursor.
    fn click(self, ui: &UiState) -> Option<(MouseButton, u64)> {
        let mouse = self.mouse.as_ref()?;
        let button = [MouseButton::Left, MouseButton::Right]
            .into_iter()
            .find(|button| mouse.just_pressed(*button))?;
        if self.modal_open.is_some() || !crate::networking::gameplay_input_allowed(self.reconnect) {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        Some((button, find_frame_at(&ui.registry, cursor.x, cursor.y)?))
    }
}

/// Edit boxes of both frames with their Retail `letters` limit and whether they take
/// digits only (MoneyInputFrame.xml:79-111: gold 7, silver / copper 2).
fn input_boxes() -> Vec<(&'static str, usize, bool)> {
    let mut boxes = vec![
        (bank_ui::TAB_NAME_BOX, 15, false),
        (guild_ui::INFO_BOX, 500, false),
    ];
    for money in [bank_ui::MONEY_BOXES, guild_ui::MONEY_BOXES] {
        boxes.push((money.gold, 7, true));
        boxes.push((money.silver, 2, true));
        boxes.push((money.copper, 2, true));
    }
    boxes
}

fn input_box(registry: &FrameRegistry, id: u64) -> Option<(&'static str, usize, bool)> {
    input_boxes()
        .into_iter()
        .find(|(name, ..)| registry.get_by_name(name) == Some(id))
}

fn read_input_texts(registry: &FrameRegistry) -> InputTexts {
    input_boxes()
        .into_iter()
        .filter_map(|(name, ..)| {
            let id = registry.get_by_name(name)?;
            match registry.get(id)?.widget_data.as_ref()? {
                WidgetData::EditBox(data) => Some((name, data.text.clone())),
                _ => None,
            }
        })
        .collect()
}

fn set_input_text(registry: &mut FrameRegistry, name: &str, value: &str) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    if let Some(WidgetData::EditBox(data)) = registry
        .get_mut(id)
        .and_then(|frame| frame.widget_data.as_mut())
    {
        data.replace_range(0, data.text.len(), value);
        data.cursor_end();
    }
}

fn set_focus(ui: &mut UiState, id: Option<u64>) {
    ui.focused_frame = id;
    ui.registry.focused_frame = id;
}

#[derive(SystemParam)]
struct BankInputs<'w> {
    bank: ResMut<'w, BankState>,
    guild: ResMut<'w, GuildBankState>,
    manager: ResMut<'w, WindowManager>,
    popups: ResMut<'w, PopupStack>,
}

/// A click on an edit box focuses it; other clicks on a bank frame run its action.
fn handle_bank_clicks(
    pointer: Pointer,
    mut ui: ResMut<UiState>,
    mut inputs: BankInputs,
    mut bank_requests: MessageWriter<BankRequest>,
    mut guild_requests: MessageWriter<GuildBankRequest>,
) {
    if !inputs.bank.is_open() && !inputs.guild.is_open() {
        return;
    }
    let Some((button, frame_id)) = pointer.click(&ui) else {
        return;
    };
    if input_box(&ui.registry, frame_id).is_some() {
        set_focus(&mut ui, Some(frame_id));
        return;
    }
    if focused_editbox(&ui).is_some_and(|focused| input_box(&ui.registry, focused).is_some()) {
        set_focus(&mut ui, None);
    }
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    let texts = read_input_texts(&ui.registry);
    if inputs.bank.is_open() && action.starts_with("bank_") {
        let outcome = actions::bank_action(&action, button, &mut inputs.bank, &texts);
        apply_outcome(
            outcome,
            WindowId::Bank,
            &mut ui,
            &mut inputs,
            &mut bank_requests,
        );
    } else if inputs.guild.is_open() && action.starts_with("guild_bank_") {
        let outcome = actions::guild_bank_action(&action, button, &mut inputs.guild, &texts);
        apply_outcome(
            outcome,
            WindowId::GuildBank,
            &mut ui,
            &mut inputs,
            &mut guild_requests,
        );
    }
}

fn apply_outcome<R: Message>(
    outcome: Outcome<R>,
    window: WindowId,
    ui: &mut UiState,
    inputs: &mut BankInputs,
    requests: &mut MessageWriter<R>,
) {
    for (name, text) in &outcome.texts {
        set_input_text(&mut ui.registry, name, text);
    }
    if let Some(popup) = outcome.popup {
        inputs.popups.push(popup);
    }
    if outcome.close {
        inputs.manager.close(window);
    }
    for request in outcome.requests {
        requests.write(request);
    }
}

/// `CONFIRM_BUY_*_TAB` accepted: buy the tab of the frame that asked.
fn confirm_tab_purchases(
    mut results: MessageReader<PopupResult>,
    bank: Res<BankState>,
    guild: Res<GuildBankState>,
    mut bank_requests: MessageWriter<BankRequest>,
    mut guild_requests: MessageWriter<GuildBankRequest>,
) {
    for result in results.read() {
        if result.outcome != PopupOutcome::Accepted {
            continue;
        }
        if result.key == BUY_BANK_TAB_POPUP && bank.is_open() {
            bank_requests.write(BankRequest::PurchaseTab { bank: bank.shown });
        } else if result.key == BUY_GUILD_BANK_TAB_POPUP && guild.is_open() {
            guild_requests.write(GuildBankRequest::BuyTab);
        }
    }
}

/// Keys edit the focused bank edit box; Enter, Tab and Escape leave it.
fn handle_bank_keyboard(mut key_events: MessageReader<KeyboardInput>, mut ui: ResMut<UiState>) {
    let Some(focused) = focused_editbox(&ui) else {
        key_events.read().for_each(drop);
        return;
    };
    let Some((_, letters, digits_only)) = input_box(&ui.registry, focused) else {
        return;
    };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab | KeyCode::Escape => {
                set_focus(&mut ui, None);
                return;
            }
            _ => {
                let typed = event.text.as_deref().unwrap_or("");
                let editing = matches!(
                    event.key_code,
                    KeyCode::Backspace
                        | KeyCode::Delete
                        | KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::Home
                        | KeyCode::End
                );
                if editing || accepts_text(&ui.registry, focused, letters, digits_only, typed) {
                    mutate_editbox_from_key(&mut ui.registry, focused, event);
                }
            }
        }
    }
}

fn accepts_text(
    registry: &FrameRegistry,
    id: u64,
    letters: usize,
    digits_only: bool,
    typed: &str,
) -> bool {
    let current = match registry
        .get(id)
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(WidgetData::EditBox(data)) => data.text.chars().count(),
        _ => return false,
    };
    current + typed.chars().count() <= letters
        && (!digits_only || typed.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests;
