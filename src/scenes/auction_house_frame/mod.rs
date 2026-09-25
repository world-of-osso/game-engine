//! Auction house frame (Wide window): opened by an auctioneer interaction, driven by the
//! network [`AuctionHouseState`] and the frame's own selections ([`AuctionHouseUi`]).

mod actions;
mod view;

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::auction_house::{AuctionHouseState, AuctionRequest};
use game_engine::item_catalog::item_catalog_entry;
use game_engine::quest_runtime::NpcFrameEvent;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::auction_house_frame_component::{
    AuctionHouseFrameState, AuctionHouseTab, AuctionsSubTab, SEARCH_BOX, auction_house_frame_screen,
};
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::{AuctionDuration, NpcRole};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking_quests::NpcInteractionRequest;
use crate::ui_input::{mutate_editbox_from_key, walk_up_for_onclick};
use crate::ui_input_mode::focused_editbox;
use crate::window_manager::{WindowId, WindowManager};

use view::{InputTexts, ViewInputs, input_names};

/// The frame's own state: the auctioneer, tab and selections.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct AuctionHouseUi {
    /// Auctioneer whose interaction opened the frame (server entity bits).
    pub npc: Option<u64>,
    pub tab: AuctionHouseTab,
    pub category: Option<usize>,
    /// Item whose auctions the item buy frame lists.
    pub browse_item: Option<u32>,
    pub selected_auction: Option<u64>,
    /// Auction the buy dialog asks to buy out.
    pub dialog_auction: Option<u64>,
    pub sell_item: Option<u64>,
    pub buyout_mode: bool,
    pub duration: AuctionDuration,
    pub duration_menu_open: bool,
    pub auctions_tab: AuctionsSubTab,
    pub close_requested: bool,
}

impl Default for AuctionHouseUi {
    fn default() -> Self {
        Self {
            npc: None,
            tab: AuctionHouseTab::Buy,
            category: None,
            browse_item: None,
            selected_auction: None,
            dialog_auction: None,
            sell_item: None,
            // `AuctionHouseBuyoutModeCheckButtonMixin:OnShow` checks it.
            buyout_mode: true,
            // `AuctionHouseSellFrameMixin` defaults to the middle duration.
            duration: AuctionDuration::Medium,
            duration_menu_open: false,
            auctions_tab: AuctionsSubTab::Auctions,
            close_requested: false,
        }
    }
}

struct AuctionFrameScreen {
    screen: Screen,
    shared: SharedContext,
}

// SAFETY: the screen is only touched from main-schedule systems, like every other
// Screen resource in this crate.
unsafe impl Send for AuctionFrameScreen {}
unsafe impl Sync for AuctionFrameScreen {}

#[derive(Resource)]
struct AuctionFrameScreenRes(AuctionFrameScreen);

#[derive(Resource, Default, PartialEq)]
struct AuctionFrameModel(AuctionHouseFrameState);

pub struct AuctionHouseFramePlugin;

impl Plugin for AuctionHouseFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuctionHouseUi>()
            .init_resource::<UiErrors>()
            .add_message::<NpcFrameEvent>()
            .add_message::<NpcInteractionRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_auction_frame.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_auction_frame);
        app.add_systems(
            Update,
            (
                open_on_auctioneer,
                handle_auction_clicks,
                handle_auction_keyboard,
                sync_auction_window,
                show_auction_errors,
                sync_auction_screen,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_auction_frame(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let model = AuctionHouseFrameState::default();
    let mut shared = SharedContext::new();
    shared.insert(model.clone());
    let mut screen = Screen::new(auction_house_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(AuctionFrameScreenRes(AuctionFrameScreen { screen, shared }));
    commands.insert_resource(AuctionFrameModel(model));
}

fn teardown_auction_frame(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screen: Option<ResMut<AuctionFrameScreenRes>>,
    mut frame_ui: ResMut<AuctionHouseUi>,
    mut net: ResMut<AuctionHouseState>,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    *frame_ui = AuctionHouseUi::default();
    net.close();
    commands.remove_resource::<AuctionFrameScreenRes>();
    commands.remove_resource::<AuctionFrameModel>();
}

/// An auctioneer interaction asks the server to open its house; the server ending the
/// interaction closes the frame.
fn open_on_auctioneer(
    mut events: MessageReader<NpcFrameEvent>,
    mut frame_ui: ResMut<AuctionHouseUi>,
    mut net: ResMut<AuctionHouseState>,
) {
    for event in events.read() {
        match *event {
            NpcFrameEvent::Opened {
                npc,
                role: NpcRole::AuctionHouse,
            } => {
                *frame_ui = AuctionHouseUi {
                    npc: Some(npc),
                    ..Default::default()
                };
                net.request(AuctionRequest::Open);
            }
            NpcFrameEvent::Closed { npc } if frame_ui.npc == Some(npc) => {
                frame_ui.npc = None;
                net.close();
            }
            _ => {}
        }
    }
}

/// The house opening opens the Wide window; the window closing (close button, Escape,
/// another Wide window) ends the interaction; the house closing closes the window.
fn sync_auction_window(
    mut manager: ResMut<WindowManager>,
    mut net: ResMut<AuctionHouseState>,
    mut frame_ui: ResMut<AuctionHouseUi>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut was_open: Local<bool>,
) {
    if std::mem::take(&mut frame_ui.close_requested) {
        manager.close(WindowId::AuctionHouse);
    }
    let window_open = manager.is_open(WindowId::AuctionHouse);
    if net.is_open && !*was_open {
        manager.open(WindowId::AuctionHouse);
    } else if net.is_open && !window_open {
        net.close();
        if let Some(npc) = frame_ui.npc.take() {
            requests.write(NpcInteractionRequest::Close { npc });
        }
    } else if !net.is_open && window_open {
        manager.close(WindowId::AuctionHouse);
    }
    *was_open = net.is_open;
}

fn show_auction_errors(mut net: ResMut<AuctionHouseState>, mut errors: ResMut<UiErrors>) {
    if net.errors.is_empty() {
        return;
    }
    for error in std::mem::take(&mut net.errors) {
        errors.add(error);
    }
}

fn read_input_texts(registry: &FrameRegistry) -> InputTexts {
    input_names()
        .into_iter()
        .filter_map(|name| {
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

#[derive(SystemParam)]
struct AuctionPointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl AuctionPointer<'_, '_> {
    /// Frame under a fresh left click, if gameplay input is allowed.
    fn clicked_frame(self, ui: &UiState) -> Option<u64> {
        let pressed = self
            .mouse
            .as_ref()
            .is_some_and(|mouse| mouse.just_pressed(MouseButton::Left));
        if !pressed || self.modal_open.is_some() {
            return None;
        }
        if !crate::networking::gameplay_input_allowed(self.reconnect) {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        find_frame_at(&ui.registry, cursor.x, cursor.y)
    }
}

fn frame_input_name(registry: &FrameRegistry, id: u64) -> Option<&'static str> {
    input_names()
        .into_iter()
        .find(|name| registry.get_by_name(name) == Some(id))
}

fn set_focus(ui: &mut UiState, id: Option<u64>) {
    ui.focused_frame = id;
    ui.registry.focused_frame = id;
}

/// A click on an edit box focuses it; any other click in the frame runs its action.
fn handle_auction_clicks(
    pointer: AuctionPointer,
    mut ui: ResMut<UiState>,
    mut net: ResMut<AuctionHouseState>,
    mut frame_ui: ResMut<AuctionHouseUi>,
) {
    if !net.is_open {
        return;
    }
    let Some(frame_id) = pointer.clicked_frame(&ui) else {
        return;
    };
    let focused_is_ours = focused_editbox(&ui)
        .is_some_and(|focused| frame_input_name(&ui.registry, focused).is_some());
    if frame_input_name(&ui.registry, frame_id).is_some() {
        set_focus(&mut ui, Some(frame_id));
        return;
    }
    if focused_is_ours {
        set_focus(&mut ui, None);
    }
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    run_action(&action, &mut ui, &mut net, &mut frame_ui);
}

fn run_action(
    action: &str,
    ui: &mut UiState,
    net: &mut AuctionHouseState,
    frame_ui: &mut AuctionHouseUi,
) {
    let texts = read_input_texts(&ui.registry);
    for (name, value) in actions::dispatch(action, net, frame_ui, &texts) {
        set_input_text(&mut ui.registry, name, &value);
    }
}

/// Numeric boxes take digits only, at most their `letters` (MoneyInputFrame.xml:79-111:
/// gold 7, silver / copper 2; quantity 8); the search box takes 64 (`bytes="64"`).
fn input_letters(name: &str) -> usize {
    use game_engine::ui::screens::auction_house_frame_component::QUANTITY_BOX;
    if name == SEARCH_BOX {
        64
    } else if name == QUANTITY_BOX {
        8
    } else if name.ends_with("Gold") {
        7
    } else {
        2
    }
}

fn accepts_text(registry: &FrameRegistry, id: u64, name: &str, typed: &str) -> bool {
    let current = match registry
        .get(id)
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(WidgetData::EditBox(data)) => data.text.chars().count(),
        _ => return false,
    };
    let digits_only = name != SEARCH_BOX;
    current + typed.chars().count() <= input_letters(name)
        && (!digits_only || typed.chars().all(|ch| ch.is_ascii_digit()))
}

/// Keys edit the focused frame edit box; Enter in the search box searches, Enter or Tab
/// elsewhere leaves the box.
fn handle_auction_keyboard(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    mut net: ResMut<AuctionHouseState>,
    mut frame_ui: ResMut<AuctionHouseUi>,
) {
    let Some(focused) = focused_editbox(&ui) else {
        key_events.read().for_each(drop);
        return;
    };
    let Some(name) = frame_input_name(&ui.registry, focused) else {
        key_events.read().for_each(drop);
        return;
    };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => {
                set_focus(&mut ui, None);
                if name == SEARCH_BOX && event.key_code != KeyCode::Tab {
                    use game_engine::ui::screens::auction_house_frame_component::ACTION_SEARCH;
                    run_action(ACTION_SEARCH, &mut ui, &mut net, &mut frame_ui);
                }
                return;
            }
            KeyCode::Escape => {}
            _ => {
                let typed = event.text.as_deref().unwrap_or("");
                let editing_key = matches!(
                    event.key_code,
                    KeyCode::Backspace
                        | KeyCode::Delete
                        | KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::Home
                        | KeyCode::End
                );
                if editing_key || accepts_text(&ui.registry, focused, name, typed) {
                    mutate_editbox_from_key(&mut ui.registry, focused, event);
                }
            }
        }
    }
}

fn sync_auction_screen(
    mut ui: ResMut<UiState>,
    screen: Option<ResMut<AuctionFrameScreenRes>>,
    model: Option<ResMut<AuctionFrameModel>>,
    net: Res<AuctionHouseState>,
    frame_ui: Res<AuctionHouseUi>,
    manager: Res<WindowManager>,
) {
    let (Some(mut screen), Some(mut model)) = (screen, model) else {
        return;
    };
    // A closed frame shows nothing, so it builds (and loads the item catalog for) nothing.
    let visible = net.is_open && manager.is_open(WindowId::AuctionHouse);
    let state = if visible {
        let texts = read_input_texts(&ui.registry);
        view::build_view(&ViewInputs {
            net: &net,
            ui: &frame_ui,
            texts: &texts,
            catalog: &item_catalog_entry,
            visible,
        })
    } else {
        AuctionHouseFrameState::default()
    };
    if model.0 == state {
        return;
    }
    model.0 = state.clone();
    let res = &mut screen.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

#[cfg(test)]
mod tests;
