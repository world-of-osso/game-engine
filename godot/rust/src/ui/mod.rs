pub(crate) mod assets;
mod auction_icon_preview;
#[cfg(debug_assertions)]
mod audit_probe;
mod aura_preview;
#[cfg(test)]
mod buffcancel_tests;
// New preview APIs belong in their own *_preview.rs #[godot_api(secondary)] block, never here.
mod auction_preview;
pub(crate) mod castbar_fx;
mod castbar_preview;
mod dungeon_preview;
mod flight_map_preview;
mod forevergaps_preview;
mod guild_preview;
pub(crate) mod hud_edit_layout;
mod hud_edit_preview;
#[cfg(test)]
mod hud_edit_tests;
mod icon_masks;
pub(crate) mod input_queue;
mod launcher_preview;
mod layout;
mod meter_preview;
mod minimap_preview;
mod options_keybindings;
mod parts;
mod party_preview;
mod professions_preview;
mod projection;
mod registration_preview;
mod scroll_lists;
mod sidebarbinds_preview;
mod spellbook_preview;
pub(crate) mod ui_parent;

use std::collections::VecDeque;

use game_engine_core::ui_layout_data::LayoutSettings;
use game_engine_ui_model::bag_frame_component::BagFrameState;
use game_engine_ui_model::buff_frame_component::{BuffFrameState, buff_frame_screen};
use game_engine_ui_model::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use game_engine_ui_model::char_create_component::CharCreateUiState;
use game_engine_ui_model::char_select_component::{CharSelectAction, apply_char_select_postsetup};
use game_engine_ui_model::chat_frame_component::{ChatFrameView, chat_frame_screen};
use game_engine_ui_model::entrance_difficulty_component::{
    EntranceBarState, apply_entrance_bar_postsetup, entrance_difficulty_screen,
};
use game_engine_ui_model::game_menu_component::GameMenuViewModel;
use game_engine_ui_model::game_tooltip::{GameTooltipView, game_tooltip_screen};
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, inworld_unit_frames_screen,
};
use game_engine_ui_model::main_action_bar_component::{MainActionBarState, main_action_bar_screen};
use game_engine_ui_model::merchant_frame_component::MerchantFrameState;
use game_engine_ui_model::minimap::{
    MinimapClusterState, apply_minimap_postsetup, minimap_cluster_screen,
};
use game_engine_ui_model::mirror_timer_component::{MIRROR_TIMER_CONTAINER, mirror_timer_screen};
use game_engine_ui_model::mirror_timer_data::MirrorTimersData;
use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use game_engine_ui_model::pet_action_bar_component::PetActionBarState;
use game_engine_ui_model::spellbook_frame_component::{
    SpellbookFrameState, apply_spellbook_postsetup, spellbook_frame_screen,
};
use game_engine_ui_model::stack_split_frame_component::StackSplitFrameState;
use game_engine_ui_model::world_map_frame_component::{
    WORLD_MAP_QUEST_AREAS, WorldMapFrameState, apply_world_map_postsetup, world_map_frame_screen,
};
use game_engine_ui_model::{
    CharacterCreateModel, CharacterSelectModel, GameMenuModel, LoadingModel, LoginModel,
    UiErrorsModel, apply_character_create_postsetup,
    loading_component::{LoadingScreenState, LoadingViewportHeight, advance_displayed_progress},
    login,
    ui_errors_data::UiErrorsData,
};
use godot::classes::{CanvasLayer, Control, ICanvasLayer};
use godot::prelude::*;
use ui_toolkit::frame::{NineSlice, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

use projection::{UiInput, UiProjection};
use ui_parent::UiParent;

/// Registry-authoritative UI host; the canvas is a projection, not a second model.
#[derive(GodotClass)]
#[class(base = CanvasLayer)]
pub struct RegistryUi {
    base: Base<CanvasLayer>,
    model: Option<RegistryModel>,
    projection: Option<UiProjection>,
    actions: VecDeque<String>,
    /// Right-clicks and Shift-left-clicks: `(action, right, shift)`.
    alt_clicks: VecDeque<(String, bool, bool)>,
    /// Cursor-item input with its arrival stamp; `None` for canvases without item slots.
    bag_inputs: Option<VecDeque<(u64, crate::bag_cursor::BagInput)>>,
    pointer_clicks: u32,
    /// Retail `toplevel`: a press inside raises this canvas over the other windows.
    toplevel: bool,
    /// Arrival stamp of the latest press inside a toplevel canvas, until the host raises it.
    raise_request: Option<u64>,
    slider_events: VecDeque<SliderInput>,
    login_fade: Option<f32>,
    loading_displayed_percent: f32,
    /// HUDs use UIParent scaling until an explicit options scale is set.
    ui_parent: bool,
    ui_scale: Option<f32>,
    scroll_stepper: Option<scroll_lists::StepperHold>,
}

/// Raw authored-slider input; the host applies its own policy and passes back a view.
#[derive(Debug, Clone, PartialEq)]
pub struct SliderInput {
    pub action: String,
    pub value: f32,
}

struct RegistryModel {
    screen: Screen,
    shared: SharedContext,
    registry: FrameRegistry,
    postsetup: ScreenPostsetup,
    icon_masks: icon_masks::IconMasks,
}

#[derive(Clone, Copy)]
enum ScreenPostsetup {
    None,
    Login,
    CharacterSelect,
    CharacterCreate,
    Loading,
    /// The world map, with the dynamic texture its objective area overlay shows.
    WorldMap(DynamicTextureId),
    EntranceBar,
    Merchant,
    Auction,
    /// Enter in a TradeFrame money box submits the amount.
    Trade,
    Spellbook,
    Minimap,
    CharacterFrame,
    /// Bag bar icons and container portraits get their round `CircleMask`.
    Bags,
    /// Header glyphs become tinted masks (`IconMasks`).
    ChatFrame,
    /// Autocast Shines draw the host's composite.
    PetActionBar,
    PortraitParty,
    CastingBar,
}

impl RegistryModel {
    /// A view of another Options page shows it from its top, as Retail's settings list
    /// scrolls to the beginning when it gets a category's elements
    /// (`Blizzard_SettingsList.lua:140`, `ScrollBoxListMixin:SetDataProvider`,
    /// `Blizzard_SharedXML/Shared/Scroll/ScrollBox.lua:703-713`).
    fn reset_options_scroll_for(&mut self, view: &GameMenuViewModel) {
        let other_page = self
            .shared
            .get::<GameMenuViewModel>()
            .is_some_and(|shown| shown.options_page() != view.options_page());
        if other_page {
            self.registry.scroll_lists.scroll_to(
                game_engine_ui_model::options_menu_component::OPTIONS_CONTENT_SCROLL,
                0,
            );
        }
    }

    /// A QuestFrame panel shown anew starts at its top: every panel's `OnShow`, and a
    /// `QUEST_ITEM_UPDATE` that refills it, calls `ScrollBar:ScrollToBegin`
    /// (`Mainline/QuestFrame.lua:73-83,128,299,558`).
    fn reset_quest_scroll_for(
        &mut self,
        state: &game_engine_ui_model::quest_frame_component::QuestFrameState,
    ) {
        use game_engine_ui_model::quest_frame_component::{QUEST_SCROLL_FRAMES, QuestFrameState};
        let other_page = self
            .shared
            .get::<QuestFrameState>()
            .is_some_and(|shown| shown.page != state.page);
        if other_page {
            for list in QUEST_SCROLL_FRAMES {
                self.registry.scroll_lists.scroll_to(list, 0);
            }
        }
    }

    /// Retail sizes the scroll child after laying out QuestInfo. Read the native flow height
    /// before rebuilding its range and scrollbar; the child height never depends on the range.
    fn update_quest_scroll_extent(&mut self) -> bool {
        use game_engine_ui_model::quest_scroll::{QuestScrollExtent, quest_scroll_child};
        let mut lists = game_engine_ui_model::quest_frame_component::QUEST_SCROLL_FRAMES.to_vec();
        lists.push(game_engine_ui_model::quest_log_frame_component::QUEST_LOG_DETAILS_SCROLL);
        let extent = lists.into_iter().find_map(|list| {
            let child = self.registry.get_by_name(&quest_scroll_child(list))?;
            let height = self.registry.get(child)?.layout_rect.as_ref()?.height;
            Some(QuestScrollExtent {
                list: list.into(),
                height,
            })
        });
        let Some(extent) = extent else { return false };
        if self.shared.get::<QuestScrollExtent>() == Some(&extent) {
            return false;
        }
        self.shared.insert(extent);
        true
    }

    fn project_quest_extent(&mut self, projection: &mut UiProjection) -> Result<(), String> {
        projection.sync(&mut self.registry)?;
        if self.update_quest_scroll_extent() {
            self.sync();
            projection.sync(&mut self.registry)?;
        }
        Ok(())
    }

    /// Mirror the active layout's settings, then its skin: a change of either rebuilds the
    /// Screens that read the HUD layout.
    fn sync(&mut self) {
        let edit = game_engine_ui_model::hud_edit::EditModeActive(hud_edit_layout::editor_active());
        if self
            .shared
            .get::<game_engine_ui_model::hud_edit::EditModeActive>()
            != Some(&edit)
        {
            self.shared.insert(edit);
        }
        let settings = game_engine_ui_model::hud_layout::active_layout_settings();
        if self.shared.get::<LayoutSettings>() != Some(&settings) {
            self.shared.insert(settings);
        }
        self.sync_skin(ui_toolkit::atlas::thread_skin());
    }

    /// Mirror `skin` into this canvas before syncing: a change advances the `ActiveSkin`
    /// generation, so every Screen that read it rebuilds, and re-applies panel styles.
    fn sync_skin(&mut self, skin: ui_toolkit::atlas::ActiveSkin) {
        if self.shared.get::<ui_toolkit::atlas::ActiveSkin>() != Some(&skin) {
            self.shared.insert(skin);
            self.registry.refresh_panel_styles();
        }
        self.screen.sync(&self.shared, &mut self.registry);
        self.apply_postsetup();
    }

    fn apply_postsetup(&mut self) {
        match self.postsetup {
            ScreenPostsetup::None | ScreenPostsetup::Loading | ScreenPostsetup::Trade => {}
            ScreenPostsetup::Auction => self.icon_masks.apply(&mut self.registry),
            ScreenPostsetup::CastingBar => {
                game_engine_ui_model::casting_bar_frame_component::apply_casting_bar_feedback_postsetup(&mut self.registry);
            }
            ScreenPostsetup::WorldMap(texture) => {
                if let Some(state) = self.shared.get::<WorldMapFrameState>() {
                    apply_world_map_postsetup(state, &mut self.registry);
                }
                set_dynamic_texture(&mut self.registry, WORLD_MAP_QUEST_AREAS.0, texture);
            }
            ScreenPostsetup::Minimap => {
                if let Some(state) = self.shared.get::<MinimapClusterState>() {
                    apply_minimap_postsetup(state, &mut self.registry);
                }
            }
            ScreenPostsetup::PortraitParty => {
                use game_engine_ui_model::group_frames_component::GroupFramesState;
                use game_engine_ui_model::portrait_party_frame_component::{
                    PortraitPartyFrameState, apply_portrait_party_postsetup,
                };
                if let Some(state) = self.shared.get::<GroupFramesState>() {
                    let settings = self
                        .shared
                        .get::<game_engine_core::ui_layout_data::LayoutSettings>()
                        .map(|settings| *settings)
                        .unwrap_or_default();
                    let state = game_engine_ui_model::group_frames_component::sorted_party_state(
                        &state,
                        settings.party.sort.unwrap_or_default(),
                    );
                    apply_portrait_party_postsetup(&state.portrait_party, &mut self.registry);
                } else if let Some(state) = self.shared.get::<PortraitPartyFrameState>() {
                    apply_portrait_party_postsetup(state, &mut self.registry);
                }
            }
            ScreenPostsetup::Spellbook => {
                if let Some(state) = self.shared.get::<SpellbookFrameState>() {
                    apply_spellbook_postsetup(state, &mut self.registry);
                }
                self.icon_masks.apply(&mut self.registry);
            }
            ScreenPostsetup::EntranceBar => {
                if let Some(state) = self.shared.get::<EntranceBarState>() {
                    apply_entrance_bar_postsetup(state, &mut self.registry);
                }
            }
            ScreenPostsetup::Merchant => {
                game_engine_ui_model::merchant::place_merchant_windows(&mut self.registry);
                game_engine_ui_model::mail_frame_component::apply_mail_body_postsetup(
                    &mut self.registry,
                );
                // The NPC canvas's own backpack portrait.
                self.icon_masks.apply(&mut self.registry);
            }
            ScreenPostsetup::CharacterFrame => {
                if let Some(view) =
                    self.shared
                        .get::<game_engine_ui_model::character_frame::CharacterFrameView>()
                {
                    game_engine_ui_model::character_frame::apply_character_frame_postsetup(
                        &mut self.registry,
                        view.tab,
                    );
                }
            }
            ScreenPostsetup::Login => apply_login_focus_visual(&mut self.registry),
            ScreenPostsetup::CharacterSelect => apply_char_select_postsetup(&mut self.registry),
            ScreenPostsetup::CharacterCreate => {
                apply_character_create_postsetup(&self.shared, &mut self.registry);
                self.icon_masks.apply(&mut self.registry);
            }
            ScreenPostsetup::Bags | ScreenPostsetup::ChatFrame => {
                self.icon_masks.apply(&mut self.registry)
            }
            ScreenPostsetup::PetActionBar => {
                if let Some(state) = self.shared.get::<PetActionBarState>() {
                    game_engine_ui_model::pet_action_bar_component::apply_pet_action_bar_postsetup(
                        state,
                        &mut self.registry,
                    );
                }
            }
        }
        if let Some(view) = self
            .shared
            .get::<game_engine_ui_model::trainer_frame::TrainerView>()
        {
            game_engine_ui_model::trainer_frame::apply_trainer_art(view, &mut self.registry);
        }
        // FlightMap uses quest-window mounting, which has no Bags icon-mask postsetup.
        if self
            .shared
            .get::<game_engine_ui_model::flight_map_component::FlightMapView>()
            .is_some()
        {
            self.icon_masks.apply(&mut self.registry);
        }
    }

    fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.registry.screen_width = width;
        self.registry.screen_height = height;
        self.registry.ui_scale = scale;
        match self.postsetup {
            ScreenPostsetup::CharacterCreate => {
                if let Some(state) = self.shared.get::<CharCreateUiState>() {
                    let mut state = state.clone();
                    state.viewport_width = width as u32;
                    state.viewport_height = height as u32;
                    self.shared.insert(state);
                }
                self.sync();
            }
            ScreenPostsetup::Loading => {
                self.shared.insert(LoadingViewportHeight(height));
                self.sync();
            }
            _ => self.apply_postsetup(),
        }
        self.registry.mark_all_rects_dirty();
    }

    fn slider_input(&self, id: u64, percent: f32) -> Option<SliderInput> {
        let frame = self.registry.get(id)?;
        let WidgetData::Slider(data) = frame.widget_data.as_ref()? else {
            return None;
        };
        Some(SliderInput {
            action: frame.onclick.clone()?,
            value: (data.min + (data.max - data.min) * f64::from(percent)) as f32,
        })
    }

    fn pointer_click_eligible(&self, mut id: u64) -> bool {
        loop {
            let Some(frame) = self.registry.get(id) else {
                return false;
            };
            if frame.widget_data.as_ref().is_some_and(|data| {
                matches!(data, WidgetData::Button(button)
                    if !button.enabled || button.state == ButtonState::Disabled)
            }) {
                return false;
            }
            if frame.onclick.is_some() {
                return true;
            }
            let Some(parent) = frame.parent_id else {
                return false;
            };
            id = parent;
        }
    }

    fn click_action(&mut self, id: u64) -> Option<String> {
        let disabled = self
            .registry
            .get(id)
            .and_then(|frame| frame.widget_data.as_ref())
            .is_some_and(|data| {
                matches!(data, WidgetData::Button(button)
                    if !button.enabled || button.state == ButtonState::Disabled)
            });
        if disabled {
            return None;
        }
        let action = self.registry.click_frame(id)?;
        if action == game_engine_ui_model::bag_frame_component::ACTION_SEARCH_CLEAR {
            let search = self
                .registry
                .get_by_name(game_engine_ui_model::bag_frame_component::SEARCH_BOX)?;
            self.edit_text(search, String::new());
            self.blur_frame(search);
            return None;
        }
        Some(action)
    }

    fn queue_click_action(&mut self, actions: &mut VecDeque<String>, id: u64) {
        if let Some(action) = self.click_action(id) {
            actions.push_back(action);
        }
    }

    fn bag_input(&mut self, owner: i64, input: &UiInput) -> Option<crate::bag_cursor::BagInput> {
        if let UiInput::PointerUp(at) = input {
            return Some(crate::bag_cursor::BagInput::Release {
                owner,
                at: *at / self.registry.ui_scale,
                physical_at: *at,
            });
        }
        self.bag_click_input(owner, input)
    }

    fn bag_click_input(
        &mut self,
        owner: i64,
        input: &UiInput,
    ) -> Option<crate::bag_cursor::BagInput> {
        use game_engine_ui_model::merchant::Click;
        let (action, click, at) = match input {
            UiInput::FrameClick { id, at } => (
                self.click_action(*id)?,
                Click::LEFT,
                Some(*at / self.registry.ui_scale),
            ),
            UiInput::Click(id) => (self.click_action(*id)?, Click::LEFT, None),
            UiInput::AltClick { id, right, shift } => {
                let click = Click {
                    right: *right,
                    shift: *shift,
                };
                (self.registry.click_frame(*id)?, click, None)
            }
            _ => return None,
        };
        Some(crate::bag_cursor::BagInput::Click {
            owner,
            action,
            click,
            at,
        })
    }

    fn update_input_widgets(&mut self, event: UiInput, sliders: &mut VecDeque<SliderInput>) {
        match event {
            UiInput::Focus(id) => self.focus_frame(id),
            UiInput::Blur(id) => self.blur_frame(id),
            UiInput::Text(id, text) => self.edit_text(id, text),
            UiInput::Hover(id, hovered) => self.set_button(id, |button| button.hovered = hovered),
            UiInput::Press(id) => self.press_button(id, true),
            UiInput::Release(id) => self.press_button(id, false),
            UiInput::Slider(id, percent) => {
                if let Some(input) = self.slider_input(id, percent) {
                    sliders.push_back(input);
                }
            }
            _ => {}
        }
    }

    fn set_button(
        &mut self,
        id: u64,
        update: impl FnOnce(&mut ui_toolkit::widgets::button::ButtonData),
    ) {
        if let Some(WidgetData::Button(button)) = self
            .registry
            .get_mut(id)
            .and_then(|frame| frame.widget_data.as_mut())
        {
            update(button);
        }
    }

    /// Original `sync_button_input`: pressing pushes an enabled button; release restores it.
    fn press_button(&mut self, id: u64, pushed: bool) {
        self.set_button(id, |button| {
            if button.state != ButtonState::Disabled {
                button.state = if pushed {
                    ButtonState::Pushed
                } else {
                    ButtonState::Normal
                };
            }
        });
    }

    /// Original Enter in an edit box: login submits unless in flight; deletion confirms.
    fn submit(&self, actions: &mut VecDeque<String>) {
        let connecting = self
            .shared
            .get::<login::SharedConnecting>()
            .is_some_and(|connecting| connecting.0);
        match self.postsetup {
            ScreenPostsetup::Login if !connecting => {
                actions.push_back(login::LoginAction::Connect.to_string());
            }
            ScreenPostsetup::Auction
                if self
                    .registry
                    .focused_frame
                    .and_then(|id| self.registry.get(id))
                    .is_some_and(|frame| {
                        frame.name.as_deref()
                            == Some(game_engine_ui_model::auction_house_frame_component::SEARCH_BOX)
                    }) =>
            {
                actions.push_back("auction_search".into())
            }
            ScreenPostsetup::Trade => actions.push_back(crate::trade::ACTION_MONEY_SUBMIT.into()),
            // Original: Enter confirms a pending deletion once its gate is ready.
            ScreenPostsetup::CharacterSelect => {
                actions.push_back(CharSelectAction::ConfirmDeleteChar.to_string());
            }
            _ => {}
        }
    }

    fn focus_frame(&mut self, id: u64) {
        self.registry.focused_frame = Some(id);
    }

    fn blur_frame(&mut self, id: u64) {
        if self.registry.focused_frame == Some(id) {
            self.registry.focused_frame = None;
        }
    }

    fn edit_text(&mut self, id: u64, text: String) {
        if let Some(frame) = self.registry.get_mut(id)
            && let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut()
        {
            edit.cursor_position = text.len();
            edit.text = text;
        }
    }

    fn credentials(&self) -> Option<(String, String)> {
        let text = |name| {
            let frame = self.registry.get(self.registry.get_by_name(name)?)?;
            match frame.widget_data.as_ref()? {
                WidgetData::EditBox(edit) => Some(edit.text.clone()),
                _ => None,
            }
        };
        Some((
            text(login::USERNAME_INPUT.0)?,
            text(login::PASSWORD_INPUT.0)?,
        ))
    }
}

#[godot_api]
impl ICanvasLayer for RegistryUi {
    /// Frames drawn without art whose file was still loading show it once it arrives.
    fn process(&mut self, delta: f64) {
        if let Err(error) = self.advance_scroll_stepper(delta) {
            crate::frame_error::report_once(&format!("UI scroll repeat: {error}"));
        }
        let (Some(model), Some(projection)) = (self.model.as_mut(), self.projection.as_mut())
        else {
            return;
        };
        if let Err(error) = projection.draw_arrived_textures(&mut model.registry) {
            crate::frame_error::report_once(&format!("UI textures: {error}"));
        }
    }

    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        if let Some(projection) = self.projection.as_mut() {
            projection.handle_pointer(&event);
        }
        // A global release also ends repeat when the screen was hidden while held.
        if let Ok(button) = event.try_cast::<godot::classes::InputEventMouseButton>() {
            if button.get_button_index() == godot::global::MouseButton::LEFT && !button.is_pressed()
            {
                self.scroll_stepper = None;
            }
        }
    }

    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            base,
            model: None,
            projection: None,
            actions: VecDeque::new(),
            alt_clicks: VecDeque::new(),
            bag_inputs: None,
            pointer_clicks: 0,
            toplevel: false,
            raise_request: None,
            slider_events: VecDeque::new(),
            login_fade: None,
            loading_displayed_percent: 0.0,
            ui_parent: false,
            ui_scale: None,
            scroll_stepper: None,
        }
    }
}

pub fn create_login_ui(width: f32, height: f32) -> Result<Gd<RegistryUi>, String> {
    let mut ui = RegistryUi::new_alloc();
    let result = ui.bind_mut().initialize_login(width, height);
    if let Err(error) = result {
        ui.free();
        return Err(error);
    }
    Ok(ui)
}

impl RegistryUi {
    /// The topmost mouse-enabled frame under the physical point.
    pub(crate) fn pointer_frame_at(&self, at: Vector2) -> Option<u64> {
        let model = self.model.as_ref()?;
        let frame = self
            .projection
            .as_ref()?
            .pointer_frame_at(&model.registry, at)?;
        Some(frame.id)
    }

    /// The click action of frame `id` or its nearest ancestor with one.
    pub(crate) fn frame_click_action(&self, id: u64) -> Option<String> {
        projection::frame_click_action(&self.model.as_ref()?.registry, id)
    }

    /// The physical screen rect of frame `id` as projected.
    pub(crate) fn frame_id_rect(&self, id: u64) -> Option<Rect2> {
        let node = self.projection.as_ref()?.node(id)?;
        node.is_visible_in_tree().then(|| node.get_global_rect())
    }

    /// Route this canvas's slot clicks and pointer releases through the shared cursor
    /// queue (`GameClient::poll_window_inputs`) instead of `pop_action`.
    pub(crate) fn enable_cursor_inputs(&mut self) {
        self.bag_inputs.get_or_insert_with(VecDeque::new);
    }

    pub(crate) fn accepts_cursor_inputs(&self) -> bool {
        self.bag_inputs.is_some()
    }

    pub(crate) fn is_toplevel(&self) -> bool {
        self.toplevel
    }

    /// Arrival stamp of a press inside this toplevel canvas since the last call.
    pub(crate) fn take_raise_request(&mut self) -> Result<Option<u64>, String> {
        let error = self.sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        Ok(self.raise_request.take())
    }

    fn initialize_login(&mut self, width: f32, height: f32) -> Result<(), String> {
        let LoginModel {
            screen,
            shared,
            registry,
        } = LoginModel::new(width, height);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::Login,
        };
        model.sync();
        apply_login_art(&mut model.registry)?;
        // Original login fades in from 0.1s of LOGIN_FADE_SECS.
        self.login_fade = Some(0.1);
        set_login_alpha(&mut model.registry, 0.0);
        self.initialize_model(model, width, height)
    }

    /// Project the full authored game-menu view on the in-world HUD canvas.
    pub fn show_game_menu_view(&mut self, view: GameMenuViewModel) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let GameMenuModel {
            screen,
            shared,
            mut registry,
        } = GameMenuModel::from_view(parent.width, parent.height, view);
        registry.ui_scale = parent.scale;
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_hud_model(model, parent)
    }

    /// Reproject changed category/value/style in place without replacing the CanvasLayer.
    pub fn set_game_menu_view(&mut self, view: GameMenuViewModel) -> Result<(), String> {
        if self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<GameMenuViewModel>())
            .is_none()
        {
            return Err("Full game menu view not initialized".into());
        }
        if let Some(model) = self.model.as_mut() {
            model.reset_options_scroll_for(&view);
        }
        self.set_state(view)
    }

    /// Before QuestFrame shows `state`: back to the top of its scroll frame when its panel
    /// changed.
    pub(crate) fn reset_reputation_description_scroll(&mut self) {
        if let Some(model) = self.model.as_mut() {
            model.registry.scroll_lists.scroll_to(
                game_engine_ui_model::character_frame::REPUTATION_DESCRIPTION_SCROLL,
                0,
            );
        }
    }

    pub fn reset_quest_scroll_for(
        &mut self,
        state: &game_engine_ui_model::quest_frame_component::QuestFrameState,
    ) {
        if let Some(model) = self.model.as_mut() {
            model.reset_quest_scroll_for(state);
        }
    }

    /// Drain input after processing Godot events with `sync_input`.
    pub fn drain_slider_events(&mut self) -> Vec<SliderInput> {
        self.slider_events.drain(..).collect()
    }

    /// Initialize a dedicated RegistryUi instance for the shared world map frame.
    pub fn show_world_map(&mut self, state: WorldMapFrameState) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let [width, height] = state.viewport;
        let mut registry = FrameRegistry::new(width, height);
        let quest_areas = registry.create_dynamic_texture(1, 1, vec![0; 4])?;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(world_map_frame_screen),
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::WorldMap(quest_areas),
        };
        model.sync();
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("World map has no viewport")?;
        let size = viewport.get_visible_rect().size;
        self.initialize_model(model, size.x, size.y)
    }

    /// Initialize a dedicated RegistryUi instance for the dungeon-entrance difficulty bar.
    pub fn show_entrance_bar(&mut self, state: EntranceBarState) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(entrance_difficulty_screen),
            shared,
            registry: FrameRegistry::new(size.x, size.y),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::EntranceBar,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
    }

    /// Initialize a dedicated RegistryUi instance for a full-viewport standalone screen.
    pub(crate) fn show_standalone_screen<T: 'static>(
        &mut self,
        state: T,
        build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut registry = FrameRegistry::new(size.x, size.y);
        // The original client registers this style for every screen's plain panels.
        registry.register_panel_style(
            "default",
            game_engine_ui_model::panel_style_data::default_panel_style(),
        );
        let mut model = RegistryModel {
            screen: Screen::new(build),
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
    }

    /// Project the original cursor icon and authored stack-split picker.
    pub(crate) fn show_cursor_item(
        &mut self,
        view: crate::bag_cursor::CursorView,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            view,
            crate::bag_cursor::cursor_screen,
            ScreenPostsetup::None,
        )
    }

    /// Initialize the authored bag strip and standalone containers.
    /// Containers draw the `PortraitFrameFlatTemplate` metal border (ContainerFrame.xml:218).
    pub(crate) fn show_bags(&mut self, view: crate::bags::BagsView) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_flare_bronze_style(&mut registry, |fdid| {
            let image = assets::decode_blp(&format!("data/textures/{fdid}.blp"))?;
            Ok((image.pixels, image.width))
        })?;
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        self.show_viewport_screen_in(
            view,
            crate::bags::bags_screen,
            ScreenPostsetup::Bags,
            registry,
            parent,
        )?;
        self.enable_cursor_inputs();
        // ContainerFrame.xml:216: every bag lives in toplevel `ContainerFrameContainer`.
        self.toplevel = true;
        Ok(())
    }

    /// Cursor-item input with arrival stamps; merge canvases with `input_queue::in_arrival_order`.
    pub(crate) fn drain_bag_inputs(
        &mut self,
    ) -> Result<Vec<(u64, crate::bag_cursor::BagInput)>, String> {
        let error = self.sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let inputs = self
            .bag_inputs
            .as_mut()
            .ok_or("Bags input not initialized")?;
        Ok(inputs.drain(..).collect())
    }

    /// The Retail CharacterFrame; its clicks and releases queue as cursor inputs.
    pub(crate) fn show_character_frame(
        &mut self,
        view: game_engine_ui_model::character_frame::CharacterFrameView,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        register_auction_popup_style(&mut registry);
        self.show_viewport_screen_in(
            view,
            game_engine_ui_model::character_frame::character_frame_screen,
            ScreenPostsetup::CharacterFrame,
            registry,
            parent,
        )?;
        self.enable_cursor_inputs();
        // CharacterFrame.xml `toplevel="true"`.
        self.toplevel = true;
        Ok(())
    }

    /// The Retail micro menu row.
    pub(crate) fn show_micro_menu(
        &mut self,
        view: game_engine_ui_model::micro_menu::MicroMenuView,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            view,
            game_engine_ui_model::micro_menu::micro_menu_screen,
            ScreenPostsetup::None,
        )
    }

    pub(crate) fn show_launcher(
        &mut self,
        view: game_engine_ui_model::launcher::LauncherView,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        )?;
        self.show_viewport_screen_in(
            view,
            game_engine_ui_model::launcher::launcher_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )
    }

    /// Initialize a dedicated RegistryUi instance for the Retail main action bar.
    pub fn show_main_action_bar(&mut self, state: MainActionBarState) -> Result<(), String> {
        self.show_viewport_screen(state, main_action_bar_screen, ScreenPostsetup::None)
    }

    /// Initialize a dedicated RegistryUi instance for the Skyriding vigor widget.
    pub fn show_vigor_bar(
        &mut self,
        state: game_engine_ui_model::vigor_bar_component::VigorBarState,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            state,
            game_engine_ui_model::vigor_bar_component::vigor_bar_screen,
            ScreenPostsetup::None,
        )
    }

    /// Initialize a dedicated RegistryUi instance for the Retail pet action bar, with an empty
    /// autocast Shine composite.
    pub fn show_pet_action_bar(&mut self, mut state: PetActionBarState) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        state.shine_texture = Some(registry.create_dynamic_texture(1, 1, vec![0; 4])?);
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::pet_action_bar_component::pet_action_bar_screen,
            ScreenPostsetup::PetActionBar,
            registry,
            parent,
        )
    }

    /// Replace the pet bar state and, when given, the `size`² RGBA8 autocast Shine.
    pub fn set_pet_action_bar(
        &mut self,
        mut state: PetActionBarState,
        shine: Option<(u32, Vec<u8>)>,
    ) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Pet bar UI is not initialized")?;
        let texture = model
            .shared
            .get::<PetActionBarState>()
            .and_then(|current| current.shine_texture)
            .ok_or("Pet bar Shine texture missing")?;
        state.shine_texture = Some(texture);
        let redraw = shine.is_some();
        if let Some((size, rgba)) = shine {
            model
                .registry
                .update_dynamic_texture(texture, size, size, rgba)?;
        }
        self.set_state(state)?;
        if redraw {
            let model = self.model.as_mut().ok_or("Pet bar UI is not initialized")?;
            self.projection
                .as_mut()
                .ok_or("Native projection not initialized")?
                .sync(&mut model.registry)?;
        }
        Ok(())
    }

    /// Initialize a dedicated RegistryUi instance for the damage meter window.
    pub fn show_damage_meter(
        &mut self,
        view: game_engine_ui_model::damage_meter_data::DamageMeterView,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            view,
            game_engine_ui_model::damage_meter_component::damage_meter_screen,
            ScreenPostsetup::None,
        )
    }

    /// Initialize a dedicated RegistryUi instance for the player casting bar.
    pub fn show_casting_bar(&mut self, state: CastingBarState) -> Result<(), String> {
        self.show_viewport_screen(state, casting_bar_frame_screen, ScreenPostsetup::CastingBar)
    }

    /// Initialize a dedicated RegistryUi instance for `GameTooltip` and its comparison tooltips.
    pub(crate) fn show_game_tooltip(&mut self, view: GameTooltipView) -> Result<(), String> {
        self.show_viewport_screen(view, game_tooltip_screen, ScreenPostsetup::None)
    }

    /// Mount the shared authored Retail corpse-loot frame.
    pub fn show_loot_frame(
        &mut self,
        state: game_engine_ui_model::loot_frame_component::LootFrameState,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        )?;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::loot_frame_component::loot_frame_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )
    }

    /// Initialize a dedicated RegistryUi instance for the Retail spellbook.
    /// The spellbook in its `PlayerSpellsFrame` portrait `metal_frame` window.
    pub fn show_spellbook(&mut self, state: SpellbookFrameState) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        self.show_viewport_screen_in(
            state,
            spellbook_frame_screen,
            ScreenPostsetup::Spellbook,
            registry,
            parent,
        )
    }

    /// Initialize a dedicated RegistryUi instance for the MinimapCluster, with an empty
    /// composite registered for `MinimapDisplay` and the metal border the Forever skin's
    /// cluster draws.
    pub fn show_minimap(&mut self, mut state: MinimapClusterState) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        )?;
        state.map_texture = Some(registry.create_dynamic_texture(1, 1, vec![0; 4])?);
        self.show_viewport_screen_in(
            state,
            minimap_cluster_screen,
            ScreenPostsetup::Minimap,
            registry,
            parent,
        )
    }

    /// Replace the world map state and, when given, the RGBA8 objective area overlay
    /// (`QUEST_AREA_TEXTURE_SIZE`).
    pub fn set_world_map(
        &mut self,
        state: WorldMapFrameState,
        overlay: Option<Vec<u8>>,
    ) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("World map UI is not initialized")?;
        let ScreenPostsetup::WorldMap(texture) = model.postsetup else {
            return Err("World map quest area texture missing".into());
        };
        let redraw = overlay.is_some();
        if let Some(rgba) = overlay {
            let [width, height] =
                game_engine_ui_model::world_map_frame_component::QUEST_AREA_TEXTURE_SIZE;
            model
                .registry
                .update_dynamic_texture(texture, width, height, rgba)?;
        }
        self.set_state(state)?;
        if redraw {
            let model = self
                .model
                .as_mut()
                .ok_or("World map UI is not initialized")?;
            self.projection
                .as_mut()
                .ok_or("Native projection not initialized")?
                .sync(&mut model.registry)?;
        }
        Ok(())
    }

    /// Replace the minimap state and, when given, the `size`² RGBA8 map composite.
    pub fn set_minimap(
        &mut self,
        mut state: MinimapClusterState,
        composite: Option<(u32, Vec<u8>)>,
    ) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Minimap UI is not initialized")?;
        let texture = model
            .shared
            .get::<MinimapClusterState>()
            .and_then(|current| current.map_texture)
            .ok_or("Minimap composite texture missing")?;
        state.map_texture = Some(texture);
        let redraw = composite.is_some();
        if let Some((size, rgba)) = composite {
            model
                .registry
                .update_dynamic_texture(texture, size, size, rgba)?;
        }
        self.set_state(state)?;
        if redraw {
            let model = self.model.as_mut().ok_or("Minimap UI is not initialized")?;
            self.projection
                .as_mut()
                .ok_or("Native projection not initialized")?
                .sync(&mut model.registry)?;
        }
        Ok(())
    }

    /// Initialize a dedicated RegistryUi instance for the objective tracker.
    pub fn show_objective_tracker(&mut self, state: ObjectiveTrackerState) -> Result<(), String> {
        self.show_viewport_screen(state, objective_tracker_screen, ScreenPostsetup::None)
    }

    /// Initialize a dedicated RegistryUi instance for the experience bar.
    pub fn show_xp_bar(
        &mut self,
        state: game_engine_ui_model::xp_bar_component::XpBarState,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            state,
            game_engine_ui_model::xp_bar_component::xp_bar_screen,
            ScreenPostsetup::None,
        )
    }

    pub(crate) fn show_raid_warnings(
        &mut self,
        state: game_engine_ui_model::raid_warning::RaidWarnings,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            state,
            game_engine_ui_model::raid_warning::raid_warning_screen,
            ScreenPostsetup::None,
        )
    }

    fn show_viewport_screen<T: 'static>(
        &mut self,
        state: T,
        build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
        postsetup: ScreenPostsetup,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_flare_bronze_style(&mut registry, |fdid| {
            let image = assets::decode_blp(&format!("data/textures/{fdid}.blp"))?;
            Ok((image.pixels, image.width))
        })?;
        register_auction_popup_style(&mut registry);
        self.show_viewport_screen_in(state, build, postsetup, registry, parent)
    }

    fn show_viewport_screen_in<T: 'static>(
        &mut self,
        state: T,
        build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
        postsetup: ScreenPostsetup,
        registry: FrameRegistry,
        parent: UiParent,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(build),
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup,
        };
        model.sync();
        self.initialize_hud_model(model, parent)
    }

    /// Initialize a dedicated RegistryUi instance for one quest window (`QuestFrame`,
    /// `QuestLogFrame`) with the portrait `metal_frame` border their chrome uses.
    pub fn show_quest_window<T: 'static>(
        &mut self,
        state: T,
        build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        // QuestFrame.xml:36 and QuestMapFrame.xml:269 (QuestLogPopupDetailFrame)
        // `toplevel="true"`.
        self.toplevel = true;
        self.show_viewport_screen_in(state, build, ScreenPostsetup::None, registry, parent)
    }

    /// Initialize a dedicated RegistryUi instance for the MerchantFrame, backpack and
    /// StackSplitFrame, with the `metal_frame` window border composed from its atlases.
    pub fn show_merchant(&mut self, states: MerchantStates) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        let mut shared = SharedContext::new();
        shared.insert(states.frame);
        shared.insert(states.bags);
        shared.insert(states.split);
        let mut model = RegistryModel {
            screen: Screen::new(game_engine_ui_model::merchant::merchant_screen),
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::Merchant,
        };
        model.sync();
        self.initialize_hud_model(model, parent)?;
        self.enable_cursor_inputs();
        // MerchantFrame.xml:91 `toplevel="true"`.
        self.toplevel = true;
        Ok(())
    }

    pub fn show_mail(
        &mut self,
        state: game_engine_ui_model::mail::NativeMailView,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::mail::native_mail_screen,
            ScreenPostsetup::Merchant,
            registry,
            parent,
        )?;
        // MailFrame.xml:274 `toplevel="true"`.
        self.toplevel = true;
        self.bag_inputs = Some(VecDeque::new());
        Ok(())
    }

    /// BankFrame on the metal border; bag slots stay in the standalone Bags UI.
    pub fn show_bank(
        &mut self,
        state: game_engine_ui_model::bank_frame_component::BankFrameState,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        register_auction_popup_style(&mut registry);
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::bank_frame_component::bank_frame_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )?;
        self.enable_cursor_inputs();
        // BankFrame.xml:673 `toplevel="true"`.
        self.toplevel = true;
        Ok(())
    }

    pub fn show_achievement_window(
        &mut self,
        state: game_engine_ui_model::achievements::AchievementWindow,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        )?;
        self.toplevel = true;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::achievements::achievement_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )
    }

    pub fn show_achievement_toast(
        &mut self,
        state: shared::protocol::AchievementToastSnapshot,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            state,
            game_engine_ui_model::achievements::achievement_toast_screen,
            ScreenPostsetup::None,
        )
    }

    pub fn show_professions(
        &mut self,
        state: game_engine_ui_model::professions_frame::ProfessionView,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::professions_frame::professions_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )?;
        self.toplevel = true;
        Ok(())
    }

    pub fn show_guild_ranks(
        &mut self,
        state: game_engine_ui_model::guild_ranks::GuildRanksSession,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        )?;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::guild_rank_frame::guild_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )?;
        self.toplevel = true;
        Ok(())
    }

    /// GuildBankFrame with the backpack it deposits from.
    pub fn show_guild_bank(
        &mut self,
        state: game_engine_ui_model::guild_bank::NativeGuildBankView,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        )?;
        register_auction_popup_style(&mut registry);
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::guild_bank::native_guild_bank_screen,
            ScreenPostsetup::Merchant,
            registry,
            parent,
        )?;
        self.enable_cursor_inputs();
        // Blizzard_GuildBankUI.xml:167 `toplevel="true"`.
        self.toplevel = true;
        Ok(())
    }

    pub fn show_trade(
        &mut self,
        state: game_engine_ui_model::trade::NativeTradeView,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::trade::native_trade_screen,
            ScreenPostsetup::Trade,
            registry,
            parent,
        )?;
        // Slot clicks and drops share the cursor queue with the bags.
        self.enable_cursor_inputs();
        // TradeFrame.xml:143 `toplevel="true"`.
        self.toplevel = true;
        Ok(())
    }

    pub fn show_auction_gossip(
        &mut self,
        state: game_engine_ui_model::auction::AuctionGossipView,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(game_engine_ui_model::auction::auction_gossip_screen),
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_hud_model(model, parent)
    }
    pub fn show_auction(
        &mut self,
        state: game_engine_ui_model::auction::NativeAuctionView,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        register_auction_popup_style(&mut registry);
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(game_engine_ui_model::auction::native_auction_screen),
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::Auction,
        };
        model.sync();
        self.initialize_hud_model(model, parent)?;
        // Blizzard_AuctionHouseFrame.xml:4 `toplevel="true"`.
        self.toplevel = true;
        Ok(())
    }
    pub fn set_auction(
        &mut self,
        state: game_engine_ui_model::auction::NativeAuctionView,
    ) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Auction UI not initialized")?;
        if model
            .shared
            .get::<game_engine_ui_model::auction::NativeAuctionView>()
            == Some(&state)
        {
            return Ok(());
        }
        model.shared.insert(state);
        self.sync_model()
    }

    /// Replace the merchant screen states; unchanged states do not resync.
    pub fn set_merchant_states(&mut self, states: MerchantStates) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("Merchant UI is not initialized")?;
        let changed = model.shared.get::<MerchantFrameState>() != Some(&states.frame)
            || model.shared.get::<BagFrameState>() != Some(&states.bags)
            || model.shared.get::<StackSplitFrameState>() != Some(&states.split);
        if !changed {
            return Ok(());
        }
        model.shared.insert(states.frame);
        model.shared.insert(states.bags);
        model.shared.insert(states.split);
        self.sync_model()
    }

    /// The registry as last laid out, for anchoring frames to other frames.
    pub fn registry(&self) -> Option<&FrameRegistry> {
        self.model.as_ref().map(|model| &model.registry)
    }

    /// Initialize a dedicated RegistryUi instance for the party and raid frames.
    pub fn show_group_frames(
        &mut self,
        state: game_engine_ui_model::group_frames_component::GroupFramesState,
    ) -> Result<(), String> {
        self.show_viewport_screen(
            state,
            game_engine_ui_model::group_frames_component::group_frames_screen,
            ScreenPostsetup::PortraitParty,
        )
    }

    /// Initialize a dedicated RegistryUi instance for `StaticPopup1..3` (`PARTY_INVITE`),
    /// on the `static_popup` dialog border.
    pub fn show_static_popups(
        &mut self,
        state: game_engine_ui_model::static_popup_component::StaticPopupState,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_auction_popup_style(&mut registry);
        self.show_viewport_screen_in(
            state,
            game_engine_ui_model::static_popup_component::static_popup_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )
    }

    /// Initialize a dedicated RegistryUi instance for the player BuffFrame and DebuffFrame.
    pub fn show_buff_frame(&mut self, state: BuffFrameState) -> Result<(), String> {
        self.show_viewport_screen(state, buff_frame_screen, ScreenPostsetup::None)
    }

    /// The projected control of frame `name`.
    pub fn frame_control(&self, name: &str) -> Option<Gd<Control>> {
        let id = self.model.as_ref()?.registry.get_by_name(name)?;
        self.projection.as_ref()?.node(id)
    }

    /// The Key Bindings action whose shown binding button contains `point` (canvas UI units).
    pub fn keybinding_button_at(
        &self,
        section: game_engine_core::input_bindings_data::BindingSection,
        point: [f32; 2],
    ) -> Option<game_engine_core::input_bindings_data::InputAction> {
        let model = self.model.as_ref()?;
        options_keybindings::keybinding_button_at(&model.registry, section, point)
    }

    /// Screen rect `[x, y, w, h]` of frame `name` as last laid out (UIParent units times
    /// the UI scale), and its texture FileDataID (0 for other widgets).
    pub fn frame_rect(&self, name: &str) -> Option<([f32; 4], u32)> {
        let model = self.model.as_ref()?;
        let frame = model.registry.get(model.registry.get_by_name(name)?)?;
        let rect = frame.layout_rect.as_ref()?;
        let scale = model.registry.ui_scale;
        let fdid = match frame.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => match texture.source {
                TextureSource::FileDataId(id) => id,
                _ => 0,
            },
            _ => 0,
        };
        Some((
            [rect.x, rect.y, rect.width, rect.height].map(|value| value * scale),
            fdid,
        ))
    }

    /// Initialize a dedicated RegistryUi instance for the in-world unit frames.
    pub fn show_unit_frames(&mut self, state: InWorldUnitFramesState) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(inworld_unit_frames_screen),
            shared,
            registry: parent.registry(),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_hud_model(model, parent)
    }

    /// Initialize a dedicated RegistryUi instance for the authored error overlay.
    pub fn show_errors(&mut self) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let UiErrorsModel {
            screen,
            shared,
            mut registry,
            ..
        } = UiErrorsModel::new(parent.width, parent.height);
        registry.ui_scale = parent.scale;
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_hud_model(model, parent)
    }

    /// Initialize a dedicated RegistryUi instance for the chat frame `ChatFrame1`.
    pub fn show_chat_frame(&mut self, view: ChatFrameView) -> Result<(), String> {
        self.show_viewport_screen(view, chat_frame_screen, ScreenPostsetup::ChatFrame)
    }

    /// Replace an edit box's text with the caret at its end.
    pub fn set_editbox_text(&mut self, name: &str, text: &str) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("Registry model not initialized")?;
        let id = model
            .registry
            .get_by_name(name)
            .ok_or_else(|| format!("Missing frame {name}"))?;
        model.edit_text(id, text.to_owned());
        self.projection
            .as_mut()
            .ok_or("Native projection not initialized")?
            .sync(&mut model.registry)
    }

    /// Set a frame's alpha without rebuilding the screen (animated art).
    pub fn set_frame_alpha(&mut self, name: &str, alpha: f32) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("Registry model not initialized")?;
        let id = model
            .registry
            .get_by_name(name)
            .ok_or_else(|| format!("Missing frame {name}"))?;
        model.registry.set_alpha(id, alpha);
        self.projection
            .as_mut()
            .ok_or("Native projection not initialized")?
            .sync(&mut model.registry)
    }

    /// A frame's last laid-out rect `[x, y, w, h]` in viewport pixels.
    pub fn frame_viewport_rect(&self, name: &str) -> Option<[f32; 4]> {
        let registry = &self.model.as_ref()?.registry;
        let rect = registry
            .get(registry.get_by_name(name)?)?
            .layout_rect
            .as_ref()?;
        Some([rect.x, rect.y, rect.width, rect.height].map(|value| value * registry.ui_scale))
    }

    /// Initialize a dedicated RegistryUi instance for the retail mirror timer bars.
    pub fn show_mirror_timers(&mut self) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let parent = self.hud_parent()?;
        let mut shared = SharedContext::new();
        shared.insert(MirrorTimersData::default());
        let mut model = RegistryModel {
            screen: Screen::new(mirror_timer_screen),
            shared,
            registry: parent.registry(),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_hud_model(model, parent)
    }

    /// Apply `update` to the mirror timers; unchanged timers do not resync.
    pub fn update_mirror_timers(
        &mut self,
        update: impl FnOnce(&mut MirrorTimersData),
    ) -> Result<(), String> {
        let model = self
            .model
            .as_ref()
            .ok_or("Mirror timer UI is not initialized")?;
        if model
            .registry
            .get_by_name(MIRROR_TIMER_CONTAINER.0)
            .is_none()
        {
            return Err("RegistryUi is not a mirror timer overlay".into());
        }
        let mut timers = model
            .shared
            .get::<MirrorTimersData>()
            .ok_or("Mirror timer data is not initialized")?
            .clone();
        update(&mut timers);
        self.set_state(timers)
    }

    pub fn mirror_timer_fraction(
        &self,
        kind: game_engine_ui_model::mirror_timer_data::MirrorTimerKind,
    ) -> Option<f32> {
        let timers = self.model.as_ref()?.shared.get::<MirrorTimersData>()?;
        timers.timer(kind).map(|timer| timer.fraction())
    }

    fn update_errors(&mut self, update: impl FnOnce(&mut UiErrorsData)) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("UIErrors UI is not initialized")?;
        if model.registry.get_by_name("UIErrorsFrame").is_none() {
            return Err("RegistryUi is not an UIErrors overlay".into());
        }
        let mut errors = model
            .shared
            .get::<UiErrorsData>()
            .ok_or("UIErrors data is not initialized")?
            .clone();
        update(&mut errors);
        model.shared.insert(errors);
        self.sync_model()
    }

    pub fn add_error(&mut self, text: &str) -> Result<(), String> {
        self.update_errors(|errors| errors.add(text))
    }

    pub fn add_info(&mut self, text: &str) -> Result<(), String> {
        self.update_errors(|errors| errors.add_info(text))
    }

    pub fn tick_errors(&mut self, dt: f32) -> Result<(), String> {
        self.update_errors(|errors| errors.tick(dt))
    }

    pub fn clear_errors(&mut self) -> Result<(), String> {
        self.update_errors(|errors| *errors = UiErrorsData::default())
    }

    /// The UIParent canvas of this layer's viewport.
    fn hud_parent(&self) -> Result<UiParent, String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        Ok(UiParent::for_viewport(size.x, size.y))
    }

    /// Host an in-world HUD model on the UIParent canvas, scaled to the viewport.
    fn initialize_hud_model(
        &mut self,
        model: RegistryModel,
        parent: UiParent,
    ) -> Result<(), String> {
        self.ui_parent = true;
        self.initialize_model(
            model,
            parent.width * parent.scale,
            parent.height * parent.scale,
        )
    }

    fn initialize_model(
        &mut self,
        mut model: RegistryModel,
        width: f32,
        height: f32,
    ) -> Result<(), String> {
        let scale = self.ui_scale.unwrap_or(if self.ui_parent {
            UiParent::for_viewport(width, height).scale
        } else {
            1.0
        });
        let mut projection = UiProjection::new();
        projection.root.set_scale(Vector2::splat(scale));
        projection
            .root
            .set_size(Vector2::new(width, height) / scale);
        model.resize(width / scale, height / scale, scale);
        self.base_mut().add_child(&projection.root);
        model.project_quest_extent(&mut projection)?;
        self.projection = Some(projection);
        self.model = Some(model);
        Ok(())
    }

    /// Move the authored root of a native managed window without changing its children.
    pub fn set_window_position(&mut self, root: &str, [x, y]: [f32; 2]) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Window model not initialized")?;
        let id = model
            .registry
            .get_by_name(root)
            .ok_or_else(|| format!("Window root {root} missing"))?;
        model
            .registry
            .set_pos(id, x, y)
            .map_err(|error| format!("Window {root} position: {error:?}"))?;
        self.projection
            .as_mut()
            .ok_or("Window projection missing")?
            .sync(&mut model.registry)
    }

    /// Replace one reactive screen state; unchanged values do not resync. A resized
    /// viewport relays the screen out first.
    pub fn set_state<T: PartialEq + 'static>(&mut self, state: T) -> Result<(), String> {
        self.sync_viewport()?;
        let model = self
            .model
            .as_mut()
            .ok_or("Registry model not initialized")?;
        if model.shared.get::<T>() == Some(&state) {
            return Ok(());
        }
        if let Some(next) = (&state as &dyn std::any::Any)
            .downcast_ref::<game_engine_ui_model::quest_log_frame_component::QuestLogFrameState>(
        ) {
            use game_engine_ui_model::quest_log_frame_component::{
                QUEST_LOG_DETAILS_SCROLL, QuestLogFrameState,
            };
            let previous = model
                .shared
                .get::<QuestLogFrameState>()
                .and_then(|state| state.details.as_ref())
                .map(|details| details.quest_id);
            let selected = next.details.as_ref().map(|details| details.quest_id);
            if previous != selected {
                model
                    .registry
                    .scroll_lists
                    .scroll_to(QUEST_LOG_DETAILS_SCROLL, 0);
            }
        }
        model.shared.insert(state);
        self.sync_model()
    }

    /// Name and screen rect `[x, y, w, h]` of the button under the pointer.
    pub fn hovered_button(&self) -> Option<(String, [f32; 4])> {
        let model = self.model.as_ref()?;
        model.registry.frames_iter().find_map(|frame| {
            let WidgetData::Button(button) = frame.widget_data.as_ref()? else {
                return None;
            };
            let rect = frame.layout_rect.as_ref()?;
            let name = frame.name.clone()?;
            (button.hovered && frame.visible)
                .then_some((name, [rect.x, rect.y, rect.width, rect.height]))
        })
    }

    /// Name of the button held down (`ButtonState::Pushed` from a press).
    pub fn pushed_button(&self) -> Option<String> {
        let model = self.model.as_ref()?;
        model.registry.frames_iter().find_map(|frame| {
            let WidgetData::Button(button) = frame.widget_data.as_ref()? else {
                return None;
            };
            (button.state == ButtonState::Pushed && frame.visible)
                .then(|| frame.name.clone())
                .flatten()
        })
    }

    pub fn has_frame(&self, name: &str) -> bool {
        self.model
            .as_ref()
            .is_some_and(|model| model.registry.get_by_name(name).is_some())
    }

    pub fn is_frame_focused(&self, name: &str) -> bool {
        self.model.as_ref().is_some_and(|model| {
            model.registry.focused_frame.is_some()
                && model.registry.focused_frame == model.registry.get_by_name(name)
        })
    }

    pub fn focus_frame_named(&mut self, name: &str) -> Result<(), String> {
        let model = self
            .model
            .as_ref()
            .ok_or("Registry model not initialized")?;
        let id = model
            .registry
            .get_by_name(name)
            .ok_or_else(|| format!("Missing frame {name}"))?;
        self.projection
            .as_ref()
            .ok_or("Native projection not initialized")?
            .grab_focus(id);
        Ok(())
    }

    /// Take keyboard focus away from a frame's native control, if it has it.
    pub fn release_focus_named(&mut self, name: &str) {
        let Some(id) = self
            .model
            .as_ref()
            .and_then(|model| model.registry.get_by_name(name))
        else {
            return;
        };
        if let Some(projection) = self.projection.as_ref() {
            projection.release_focus(id);
        }
    }

    /// Apply the effective camera-equivalent scale to both layout and projected pixels.
    pub fn set_ui_scale(&mut self, scale: f32) -> Result<(), String> {
        self.ui_scale = Some(scale);
        if self.model.is_some() {
            self.sync_viewport()?;
        }
        Ok(())
    }

    fn sync_viewport(&mut self) -> Result<(), String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let scale = self.ui_scale.unwrap_or(if self.ui_parent {
            UiParent::for_viewport(size.x, size.y).scale
        } else {
            1.0
        });
        let logical = size / scale;
        let Some(model) = self.model.as_mut() else {
            return Err("Registry model not initialized".into());
        };
        if model.registry.screen_width == logical.x
            && model.registry.screen_height == logical.y
            && model.registry.ui_scale == scale
        {
            return Ok(());
        }
        model.resize(logical.x, logical.y, scale);
        let Some(projection) = self.projection.as_mut() else {
            return Err("Native projection not initialized".into());
        };
        projection.root.set_scale(Vector2::splat(scale));
        projection.root.set_size(logical);
        projection.sync(&mut model.registry)
    }

    /// Redraw this canvas under the active skin; a canvas without a screen has nothing drawn.
    pub(crate) fn sync_skin(&mut self) -> Result<(), String> {
        if self.model.is_none() {
            return Ok(());
        }
        self.sync_model()
    }

    fn sync_model(&mut self) -> Result<(), String> {
        let Some(model) = self.model.as_mut() else {
            return Err("Login model not initialized".into());
        };
        let span = crate::profile::span(|| "ui.model_sync".to_owned());
        model.sync();
        drop(span);
        let Some(projection) = self.projection.as_mut() else {
            return Err("Native projection not initialized".into());
        };
        model.project_quest_extent(projection)
    }
}

#[godot_api]
impl RegistryUi {
    #[func]
    pub fn show_loading(&mut self) -> GString {
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        let LoadingModel {
            screen,
            shared,
            registry,
        } = LoadingModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::Loading,
        };
        model.sync();
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn show_character_select(&mut self) -> GString {
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        let CharacterSelectModel {
            screen,
            shared,
            registry,
        } = {
            let _span = crate::profile::span(|| "ui.character_select.new".to_owned());
            CharacterSelectModel::new(size.x, size.y)
        };
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::CharacterSelect,
        };
        let span = crate::profile::span(|| "ui.character_select.sync".to_owned());
        model.sync();
        drop(span);
        let _span = crate::profile::span(|| "ui.character_select.initialize".to_owned());
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn show_character_create(&mut self) -> GString {
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        let CharacterCreateModel {
            screen,
            shared,
            registry,
        } = CharacterCreateModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::CharacterCreate,
        };
        model.sync();
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn show_login(&mut self) -> GString {
        if self.model.is_some() {
            return GString::new();
        }
        let viewport = self
            .base()
            .get_viewport()
            .expect("RegistryUi must be in scene tree");
        let rect = viewport.get_visible_rect();
        GString::from(
            self.initialize_login(rect.size.x, rect.size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn sync_input(&mut self) -> GString {
        GString::from(self.sync_pending_input().err().unwrap_or_default().as_str())
    }

    fn sync_pending_input(&mut self) -> Result<(), String> {
        self.sync_viewport()?;
        let projection = self
            .projection
            .as_mut()
            .ok_or("Login UI is not initialized")?;
        let inputs = projection.drain_input();
        if inputs.is_empty() {
            return Ok(());
        }
        for (arrival, event) in inputs {
            self.dispatch_ui_input(arrival, event)?;
        }
        self.sync_model()
    }

    fn queue_bag_input(&mut self, arrival: u64, event: &UiInput) -> Result<bool, String> {
        let owner = self.to_gd().instance_id().to_i64();
        let Some(queue) = self.bag_inputs.as_mut() else {
            return Ok(false);
        };
        let model = self
            .model
            .as_mut()
            .ok_or("Login model is not initialized")?;
        let Some(input) = model.bag_input(owner, event) else {
            return Ok(false);
        };
        queue.push_back((arrival, input));
        Ok(true)
    }

    fn release_search_clear_focus(&mut self, event: &UiInput) {
        use game_engine_ui_model::bag_frame_component::{ACTION_SEARCH_CLEAR, SEARCH_BOX};
        let (UiInput::Click(id) | UiInput::FrameClick { id, .. }) = event else {
            return;
        };
        let Some(model) = self.model.as_ref() else {
            return;
        };
        let action = model
            .registry
            .get(*id)
            .and_then(|frame| frame.onclick.as_deref());
        if action == Some(ACTION_SEARCH_CLEAR) {
            self.release_focus_named(SEARCH_BOX);
        }
    }

    fn dispatch_ui_input(&mut self, arrival: u64, event: UiInput) -> Result<(), String> {
        self.release_search_clear_focus(&event);
        if self.toplevel && matches!(event, UiInput::PointerDown(_)) {
            self.raise_request = Some(arrival);
        }
        if self.queue_bag_input(arrival, &event)? {
            return Ok(());
        }
        let model = self
            .model
            .as_mut()
            .ok_or("Login model is not initialized")?;
        match event {
            UiInput::Click(id) | UiInput::FrameClick { id, .. } => {
                model.queue_click_action(&mut self.actions, id);
            }
            UiInput::AltClick { id, right, shift } => {
                if let Some(action) = model.registry.click_frame(id) {
                    self.alt_clicks.push_back((action, right, shift));
                }
            }
            UiInput::PointerDown(id) => {
                if model.pointer_click_eligible(id) {
                    self.pointer_clicks += 1;
                }
            }
            UiInput::Submit => model.submit(&mut self.actions),
            other => model.update_input_widgets(other, &mut self.slider_events),
        }
        Ok(())
    }

    pub fn sync_pointer_clicks(&mut self) -> Result<u32, String> {
        if self
            .projection
            .as_ref()
            .is_some_and(UiProjection::has_pointer_down)
        {
            let error = self.sync_input();
            if !error.is_empty() {
                return Err(error.to_string());
            }
        }
        Ok(self.pop_ui_clicks())
    }

    #[func]
    pub fn pop_ui_clicks(&mut self) -> u32 {
        let clicks = self.pointer_clicks;
        self.pointer_clicks = 0;
        clicks
    }

    /// Read-only action lookup for pointer fixtures; input still travels through Godot.
    #[func]
    fn control_for_action(&self, action: GString) -> Option<Gd<Control>> {
        let model = self.model.as_ref()?;
        let action = action.to_string();
        model
            .registry
            .frames_iter()
            .filter(|frame| frame.onclick.as_deref() == Some(action.as_str()))
            .filter_map(|frame| self.projection.as_ref()?.node(frame.id))
            .find(|node| node.is_visible_in_tree())
    }

    #[func]
    pub fn pop_action(&mut self) -> GString {
        let error = self.sync_input();
        if !error.is_empty() {
            godot_error!("Native UI input sync: {error}");
        }
        GString::from(self.actions.pop_front().unwrap_or_default().as_str())
    }

    /// The next right-click or Shift-left-click action: `(action, right, shift)`.
    pub fn pop_alt_click(&mut self) -> Option<(String, bool, bool)> {
        let error = self.sync_input();
        if !error.is_empty() {
            godot_error!("Native UI input sync: {error}");
        }
        self.alt_clicks.pop_front()
    }

    #[func]
    pub fn credentials(&self) -> VarDictionary {
        let mut credentials = VarDictionary::new();
        if let Some((username, password)) = self.model.as_ref().and_then(RegistryModel::credentials)
        {
            credentials.set("username", username);
            credentials.set("password", password);
        }
        credentials
    }

    pub fn login_submission_pending(&self) -> bool {
        self.model.as_ref().is_some_and(|model| {
            model
                .shared
                .get::<login::SharedConnecting>()
                .is_some_and(|state| state.0)
        })
    }

    pub fn registration_mode(&self) -> bool {
        self.model.as_ref().is_some_and(|model| {
            model
                .shared
                .get::<login::SharedRegistration>()
                .is_some_and(|state| state.0)
        })
    }

    pub fn toggle_registration(&mut self, server: &str) -> Result<(), String> {
        let registration = !self.registration_mode();
        let model = self.model.as_mut().ok_or("Login UI is not initialized")?;
        model.shared.insert(login::SharedRegistration(registration));
        model
            .shared
            .insert(login::SharedRealmText(server.to_owned()));
        model.shared.insert(login::SharedStatusText(String::new()));
        model.shared.insert(login::SharedStatusInformational(false));
        self.sync_model()
    }

    /// Original development-realm prefill: fill both fields, then focus the first empty one.
    pub fn prefill_login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Login UI is not initialized")?;
        let field = |name: &str| {
            model
                .registry
                .get_by_name(name)
                .ok_or_else(|| format!("Missing login frame {name}"))
        };
        let (user_id, password_id) = (
            field(login::USERNAME_INPUT.0)?,
            field(login::PASSWORD_INPUT.0)?,
        );
        model.edit_text(user_id, username.to_owned());
        model.edit_text(password_id, password.to_owned());
        self.sync_model()?;
        let focus = if username.is_empty() {
            user_id
        } else {
            password_id
        };
        self.projection
            .as_ref()
            .ok_or("Native projection not initialized")?
            .grab_focus(focus);
        Ok(())
    }

    #[func]
    pub fn frame_text(&mut self, name: GString) -> GString {
        let error = self.sync_input();
        if !error.is_empty() {
            godot_error!("Native UI input sync: {error}");
        }
        let Some(model) = &self.model else {
            return GString::new();
        };
        let Some(frame) = model
            .registry
            .get_by_name(&name.to_string())
            .and_then(|id| model.registry.get(id))
        else {
            return GString::new();
        };
        match &frame.widget_data {
            Some(WidgetData::EditBox(edit)) => edit.text.as_str().into(),
            Some(WidgetData::FontString(text)) => text.text.as_str().into(),
            Some(WidgetData::Button(button)) => button.text.as_str().into(),
            _ => GString::new(),
        }
    }

    /// Advance the original login fade-in; a no-op once fully opaque.
    pub fn advance_login_fade(&mut self, dt: f32) -> Result<(), String> {
        let Some(elapsed) = self.login_fade else {
            return Ok(());
        };
        let elapsed = (elapsed + dt).min(LOGIN_FADE_SECS);
        self.login_fade = (elapsed < LOGIN_FADE_SECS).then_some(elapsed);
        let model = self.model.as_mut().ok_or("Login UI is not initialized")?;
        set_login_alpha(&mut model.registry, elapsed / LOGIN_FADE_SECS);
        let projection = self
            .projection
            .as_mut()
            .ok_or("Native projection not initialized")?;
        projection.sync(&mut model.registry)
    }

    /// Ease the displayed bar toward readiness; zone and tip use the shared defaults.
    pub fn advance_loading(
        &mut self,
        target_percent: u8,
        status: &str,
        delta: f32,
    ) -> Result<(), String> {
        self.loading_displayed_percent = advance_displayed_progress(
            self.loading_displayed_percent,
            f32::from(target_percent),
            delta,
        );
        let state = LoadingScreenState::with_default_text(
            status,
            self.loading_displayed_percent.round() as u8,
        );
        self.set_state(state)
    }

    #[func]
    fn advance_loading_progress(
        &mut self,
        target_percent: u8,
        status: GString,
        delta: f32,
    ) -> GString {
        GString::from(
            self.advance_loading(target_percent, &status.to_string(), delta)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn set_status(&mut self, status: GString) -> GString {
        GString::from(
            self.set_login_feedback(&status.to_string(), false)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub(crate) fn set_login_feedback(
        &mut self,
        status: &str,
        informational: bool,
    ) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Login UI is not initialized")?;
        model
            .shared
            .insert(login::SharedStatusText(status.to_owned()));
        model
            .shared
            .insert(login::SharedStatusInformational(informational));
        self.sync_model()
    }

    #[func]
    pub fn set_connecting(&mut self, connecting: bool) -> GString {
        let Some(model) = self.model.as_mut() else {
            return "Login UI is not initialized".into();
        };
        model.shared.insert(login::SharedConnecting(connecting));
        GString::from(self.sync_model().err().unwrap_or_default().as_str())
    }

    #[func]
    fn remove_login(&mut self) -> GString {
        let Some(model) = self.model.as_mut() else {
            return "Login UI is not initialized".into();
        };
        if let Some(root) = model.registry.get_by_name(login::LOGIN_ROOT.0) {
            model.registry.remove_frame_tree(root);
        }
        let Some(projection) = self.projection.as_mut() else {
            return "Native projection not initialized".into();
        };
        GString::from(
            projection
                .sync(&mut model.registry)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}

/// The three screen states of the merchant overlay.
pub struct MerchantStates {
    pub frame: MerchantFrameState,
    pub bags: BagFrameState,
    pub split: StackSplitFrameState,
}

/// Point the texture frame `name` at the host-owned dynamic `texture`.
fn set_dynamic_texture(registry: &mut FrameRegistry, name: &str, texture: DynamicTextureId) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::Texture(data)) = frame.widget_data.as_mut()
    {
        data.source = TextureSource::Dynamic(texture);
    }
}

/// Register `metal_frame` (`PortraitFrameTemplate` border) from the composed atlas sheet.
fn register_metal_frame_style(
    registry: &mut FrameRegistry,
    top_left: game_engine_ui_model::panel_style_data::MetalTopLeft,
) -> Result<(), String> {
    use game_engine_ui_model::panel_style_data::{
        MetalGeometry, compose_metal_sheet, metal_frame_style,
    };
    let geometry = MetalGeometry::active()?;
    let pixels = compose_metal_sheet(top_left, geometry, |fdid| {
        let image = assets::decode_blp(&format!("data/textures/{fdid}.blp"))?;
        Ok((image.pixels, image.width))
    })?;
    let (width, height) = geometry.sheet_size();
    let sheet = registry
        .create_dynamic_texture(width, height, pixels)
        .map_err(|error| format!("Metal frame sheet: {error}"))?;
    registry.register_panel_style(
        top_left.style_name(),
        metal_frame_style(TextureSource::Dynamic(sheet), geometry),
    );
    Ok(())
}

/// The Forever skin's `flare_bronze` panel on a HUD canvas, from its sheet composed of the
/// textures `load` decodes to `(pixels, width)`.
pub(crate) fn register_flare_bronze_style(
    registry: &mut FrameRegistry,
    load: impl FnMut(u32) -> Result<(Vec<u8>, u32), String>,
) -> Result<(), String> {
    use game_engine_ui_model::flare_panel::{
        FLARE_BRONZE_PANEL_STYLE, FLARE_SHEET, compose_flare_bronze_sheet, flare_bronze_style,
    };
    let pixels = compose_flare_bronze_sheet(load)?;
    let (width, height) = FLARE_SHEET;
    let sheet = registry
        .create_dynamic_texture(width, height, pixels)
        .map_err(|error| format!("flare_bronze sheet: {error}"))?;
    registry.register_panel_style(
        FLARE_BRONZE_PANEL_STYLE,
        flare_bronze_style(TextureSource::Dynamic(sheet)),
    );
    Ok(())
}

/// Original `sync_editbox_focus_visual`: focused login fields brighten their border and fill.
fn apply_login_focus_visual(registry: &mut FrameRegistry) {
    let focused = registry.focused_frame;
    for name in [login::USERNAME_INPUT.0, login::PASSWORD_INPUT.0] {
        let Some(id) = registry.get_by_name(name) else {
            continue;
        };
        let Some(slice) = registry
            .get_mut(id)
            .and_then(|frame| frame.nine_slice.as_mut())
        else {
            continue;
        };
        (slice.bg_color, slice.border_color) = if focused == Some(id) {
            ([0.32, 0.24, 0.16, 1.0], [1.0, 0.78, 0.0, 1.0])
        } else {
            ([0.22, 0.16, 0.11, 1.0], [1.0, 1.0, 1.0, 1.0])
        };
    }
}

const LOGIN_FADE_SECS: f32 = 0.75;

fn set_login_alpha(registry: &mut FrameRegistry, alpha: f32) {
    if let Some(root) = registry.get_by_name(login::LOGIN_ROOT.0) {
        registry.set_alpha(root, alpha);
    }
}

fn apply_login_art(registry: &mut FrameRegistry) -> Result<(), String> {
    for name in [login::USERNAME_INPUT.0, login::PASSWORD_INPUT.0] {
        let id = registry
            .get_by_name(name)
            .ok_or_else(|| format!("Missing login frame {name}"))?;
        let frame = registry
            .get_mut(id)
            .ok_or_else(|| format!("Missing login frame {name}"))?;
        let base = "data/ui/Common-Input-Border-";
        frame.nine_slice = Some(NineSlice {
            edge_size: 8.0,
            part_textures: Some(
                ["TL", "T", "TR", "L", "M", "R", "BL", "B", "BR"]
                    .map(|part| TextureSource::File(format!("{base}{part}.blp"))),
            ),
            bg_color: [0.22, 0.16, 0.11, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
        if let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut() {
            edit.text_insets = [12.0, 5.0, 8.0, 8.0];
            edit.font = ui_toolkit::widgets::font_string::GameFont::ArialNarrow;
            edit.text_color = [1.0, 0.8, 0.2, 1.0];
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "button_style_tests.rs"]
mod button_style_tests;
#[cfg(test)]
#[path = "entrance_bar_tests.rs"]
mod entrance_bar_tests;
#[cfg(test)]
#[path = "flare_panel_tests.rs"]
mod flare_panel_tests;
#[cfg(test)]
#[path = "hud_layout_tests.rs"]
mod hud_layout_tests;
#[cfg(test)]
#[path = "modern_panel_snapshot_tests.rs"]
mod modern_panel_snapshot_tests;
#[cfg(test)]
#[path = "skin_sync_tests.rs"]
mod skin_sync_tests;

fn register_auction_popup_style(registry: &mut FrameRegistry) {
    const COLUMNS: [f32; 4] = [1.0 / 128.0, 17.0 / 128.0, 55.0 / 128.0, 71.0 / 128.0];
    let mut uv_rects = [[0.0; 4]; 9];
    for (part, rect) in uv_rects.iter_mut().enumerate() {
        let (col, row) = (part % 3, part / 3);
        *rect = [
            COLUMNS[col],
            COLUMNS[col + 1],
            COLUMNS[row],
            COLUMNS[row + 1],
        ];
    }
    registry.register_panel_style(
        "static_popup",
        NineSlice {
            edge_size: 16.0,
            bg_color: [1.0; 4],
            border_color: [1.0; 4],
            texture: Some(TextureSource::FileDataId(6_795_680)),
            uv_rects: Some(uv_rects),
            ..Default::default()
        },
    );
}
