pub(crate) mod assets;
mod icon_masks;
mod layout;
mod parts;
mod projection;

use std::collections::VecDeque;

use game_engine_ui_model::bag_frame_component::BagFrameState;
use game_engine_ui_model::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use game_engine_ui_model::char_create_component::CharCreateUiState;
use game_engine_ui_model::char_select_component::{CharSelectAction, apply_char_select_postsetup};
use game_engine_ui_model::entrance_difficulty_component::{
    EntranceBarState, apply_entrance_bar_postsetup, entrance_difficulty_screen,
};
use game_engine_ui_model::game_menu_component::GameMenuViewModel;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, inworld_unit_frames_screen,
};
use game_engine_ui_model::main_action_bar_component::{MainActionBarState, main_action_bar_screen};
use game_engine_ui_model::merchant_frame_component::MerchantFrameState;
use game_engine_ui_model::mirror_timer_component::{MIRROR_TIMER_CONTAINER, mirror_timer_screen};
use game_engine_ui_model::mirror_timer_data::MirrorTimersData;
use game_engine_ui_model::spell_tooltip_component::{SpellTooltipState, spell_tooltip_screen};
use game_engine_ui_model::spellbook_frame_component::{
    SpellbookFrameState, apply_spellbook_postsetup, spellbook_frame_screen,
};
use game_engine_ui_model::stack_split_frame_component::StackSplitFrameState;
use game_engine_ui_model::world_map_frame_component::{
    WorldMapFrameState, apply_world_map_postsetup, world_map_frame_screen,
};
use game_engine_ui_model::{
    CharacterCreateModel, CharacterSelectModel, GameMenuModel, LoadingModel, LoginModel,
    UiErrorsModel, apply_character_create_postsetup,
    loading_component::{LoadingScreenState, LoadingViewportHeight, advance_displayed_progress},
    login,
    ui_errors_data::UiErrorsData,
};
use godot::classes::{CanvasLayer, ICanvasLayer};
use godot::prelude::*;
use ui_toolkit::frame::{NineSlice, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::texture::TextureSource;

use projection::{UiInput, UiProjection};

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
    pointer_clicks: u32,
    slider_events: VecDeque<SliderInput>,
    login_fade: Option<f32>,
    loading_displayed_percent: f32,
    ui_scale: f32,
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
    WorldMap,
    EntranceBar,
    Merchant,
    Spellbook,
}

impl RegistryModel {
    fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        self.apply_postsetup();
    }

    fn apply_postsetup(&mut self) {
        match self.postsetup {
            ScreenPostsetup::None | ScreenPostsetup::Loading => {}
            ScreenPostsetup::WorldMap => {
                if let Some(state) = self.shared.get::<WorldMapFrameState>() {
                    apply_world_map_postsetup(state, &mut self.registry);
                }
            }
            ScreenPostsetup::Spellbook => {
                if let Some(state) = self.shared.get::<SpellbookFrameState>() {
                    apply_spellbook_postsetup(state, &mut self.registry);
                }
            }
            ScreenPostsetup::EntranceBar => {
                if let Some(state) = self.shared.get::<EntranceBarState>() {
                    apply_entrance_bar_postsetup(state, &mut self.registry);
                }
            }
            ScreenPostsetup::Merchant => {
                game_engine_ui_model::merchant::place_merchant_windows(&mut self.registry)
            }
            ScreenPostsetup::Login => apply_login_focus_visual(&mut self.registry),
            ScreenPostsetup::CharacterSelect => apply_char_select_postsetup(&mut self.registry),
            ScreenPostsetup::CharacterCreate => {
                apply_character_create_postsetup(&self.shared, &mut self.registry);
                self.icon_masks.apply(&mut self.registry);
            }
        }
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.registry.screen_width = width;
        self.registry.screen_height = height;
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

    fn queue_click_action(&mut self, actions: &mut VecDeque<String>, id: u64) {
        let disabled = self
            .registry
            .get(id)
            .and_then(|frame| frame.widget_data.as_ref())
            .is_some_and(|data| {
                matches!(data, WidgetData::Button(button)
                    if !button.enabled || button.state == ButtonState::Disabled)
            });
        if !disabled && let Some(action) = self.registry.click_frame(id) {
            actions.push_back(action);
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
    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        if let Some(projection) = self.projection.as_mut() {
            projection.handle_pointer(&event);
        }
    }

    fn init(base: Base<CanvasLayer>) -> Self {
        Self {
            base,
            model: None,
            projection: None,
            actions: VecDeque::new(),
            alt_clicks: VecDeque::new(),
            pointer_clicks: 0,
            slider_events: VecDeque::new(),
            login_fade: None,
            loading_displayed_percent: 0.0,
            ui_scale: 1.0,
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

    /// Project the full authored game-menu view.
    pub fn show_game_menu_view(&mut self, view: GameMenuViewModel) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Game menu has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let GameMenuModel {
            screen,
            shared,
            registry,
        } = GameMenuModel::from_view(size.x, size.y, view);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
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
        self.set_state(view)
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
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(world_map_frame_screen),
            shared,
            registry: FrameRegistry::new(width, height),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::WorldMap,
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

    /// Initialize a dedicated RegistryUi instance for the Retail main action bar.
    pub fn show_main_action_bar(&mut self, state: MainActionBarState) -> Result<(), String> {
        self.show_viewport_screen(state, main_action_bar_screen, ScreenPostsetup::None)
    }

    /// Initialize a dedicated RegistryUi instance for the player casting bar.
    pub fn show_casting_bar(&mut self, state: CastingBarState) -> Result<(), String> {
        self.show_viewport_screen(state, casting_bar_frame_screen, ScreenPostsetup::None)
    }

    /// Initialize a dedicated RegistryUi instance for the spell tooltip.
    pub fn show_spell_tooltip(&mut self, state: SpellTooltipState) -> Result<(), String> {
        self.show_viewport_screen(state, spell_tooltip_screen, ScreenPostsetup::None)
    }

    /// Initialize a dedicated RegistryUi instance for the Retail spellbook.
    pub fn show_spellbook(&mut self, state: SpellbookFrameState) -> Result<(), String> {
        self.show_viewport_screen(state, spellbook_frame_screen, ScreenPostsetup::Spellbook)
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
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut model = RegistryModel {
            screen: Screen::new(build),
            shared,
            registry: FrameRegistry::new(size.x, size.y),
            icon_masks: Default::default(),
            postsetup,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
    }

    /// Initialize a dedicated RegistryUi instance for the MerchantFrame, backpack and
    /// StackSplitFrame, with the `metal_frame` window border composed from its atlases.
    pub fn show_merchant(&mut self, states: MerchantStates) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut registry = FrameRegistry::new(size.x, size.y);
        register_metal_frame_style(&mut registry)?;
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
        self.initialize_model(model, size.x, size.y)
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

    /// Initialize a dedicated RegistryUi instance for the in-world unit frames.
    pub fn show_unit_frames(&mut self, state: InWorldUnitFramesState) -> Result<(), String> {
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
            screen: Screen::new(inworld_unit_frames_screen),
            shared,
            registry: FrameRegistry::new(size.x, size.y),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
    }

    /// Initialize a dedicated RegistryUi instance for the authored error overlay.
    pub fn show_errors(&mut self) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let UiErrorsModel {
            screen,
            shared,
            registry,
            ..
        } = UiErrorsModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
    }

    /// Initialize a dedicated RegistryUi instance for the retail mirror timer bars.
    pub fn show_mirror_timers(&mut self) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut shared = SharedContext::new();
        shared.insert(MirrorTimersData::default());
        let mut model = RegistryModel {
            screen: Screen::new(mirror_timer_screen),
            shared,
            registry: FrameRegistry::new(size.x, size.y),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync();
        self.initialize_model(model, size.x, size.y)
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

    pub fn tick_errors(&mut self, dt: f32) -> Result<(), String> {
        self.update_errors(|errors| errors.tick(dt))
    }

    pub fn clear_errors(&mut self) -> Result<(), String> {
        self.update_errors(|errors| *errors = UiErrorsData::default())
    }

    fn initialize_model(
        &mut self,
        mut model: RegistryModel,
        width: f32,
        height: f32,
    ) -> Result<(), String> {
        let mut projection = UiProjection::new();
        projection.root.set_scale(Vector2::ONE * self.ui_scale);
        projection
            .root
            .set_size(Vector2::new(width, height) / self.ui_scale);
        model.registry.ui_scale = self.ui_scale;
        model.resize(width / self.ui_scale, height / self.ui_scale);
        self.base_mut().add_child(&projection.root);
        projection.sync(&mut model.registry)?;
        self.projection = Some(projection);
        self.model = Some(model);
        Ok(())
    }

    /// Replace one reactive screen state; unchanged values do not resync.
    pub fn set_state<T: PartialEq + 'static>(&mut self, state: T) -> Result<(), String> {
        let model = self
            .model
            .as_mut()
            .ok_or("Registry model not initialized")?;
        if model.shared.get::<T>() == Some(&state) {
            return Ok(());
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

    /// Apply the effective camera-equivalent scale to both layout and projected pixels.
    pub fn set_ui_scale(&mut self, scale: f32) -> Result<(), String> {
        if self.ui_scale != scale {
            self.ui_scale = scale;
            if self.model.is_some() {
                self.sync_viewport()?;
            }
        }
        Ok(())
    }

    fn sync_viewport(&mut self) -> Result<(), String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let logical = size / self.ui_scale;
        let Some(model) = self.model.as_mut() else {
            return Err("Registry model not initialized".into());
        };
        if model.registry.screen_width == logical.x
            && model.registry.screen_height == logical.y
            && model.registry.ui_scale == self.ui_scale
        {
            return Ok(());
        }
        model.registry.ui_scale = self.ui_scale;
        model.resize(logical.x, logical.y);
        let Some(projection) = self.projection.as_mut() else {
            return Err("Native projection not initialized".into());
        };
        projection.root.set_scale(Vector2::ONE * self.ui_scale);
        projection.root.set_size(logical);
        projection.sync(&mut model.registry)
    }

    fn sync_model(&mut self) -> Result<(), String> {
        let Some(model) = self.model.as_mut() else {
            return Err("Login model not initialized".into());
        };
        model.sync();
        let Some(projection) = self.projection.as_mut() else {
            return Err("Native projection not initialized".into());
        };
        projection.sync(&mut model.registry)
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
        } = CharacterSelectModel::new(size.x, size.y);
        let mut model = RegistryModel {
            screen,
            shared,
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::CharacterSelect,
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
        if let Err(error) = self.sync_viewport() {
            return GString::from(error.as_str());
        }
        let Some(projection) = self.projection.as_mut() else {
            return "Login UI is not initialized".into();
        };
        let inputs = projection.drain_input();
        if inputs.is_empty() {
            return GString::new();
        }
        let Some(model) = self.model.as_mut() else {
            return "Login model is not initialized".into();
        };
        for event in inputs {
            match event {
                UiInput::Click(id) => model.queue_click_action(&mut self.actions, id),
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
                UiInput::Focus(id) => model.focus_frame(id),
                UiInput::Blur(id) => model.blur_frame(id),
                UiInput::Text(id, text) => model.edit_text(id, text),
                UiInput::Submit => model.submit(&mut self.actions),
                UiInput::Hover(id, hovered) => {
                    model.set_button(id, |button| button.hovered = hovered)
                }
                UiInput::Press(id) => model.press_button(id, true),
                UiInput::Release(id) => model.press_button(id, false),
                UiInput::Slider(id, percent) => {
                    if let Some(input) = model.slider_input(id, percent) {
                        self.slider_events.push_back(input);
                    }
                }
            }
        }
        GString::from(self.sync_model().err().unwrap_or_default().as_str())
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
        let Some(model) = self.model.as_mut() else {
            return "Login UI is not initialized".into();
        };
        model
            .shared
            .insert(login::SharedStatusText(status.to_string()));
        GString::from(self.sync_model().err().unwrap_or_default().as_str())
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

/// Register `metal_frame` (`PortraitFrameTemplate` border) from the composed atlas sheet.
fn register_metal_frame_style(registry: &mut FrameRegistry) -> Result<(), String> {
    use game_engine_ui_model::panel_style_data::{
        METAL_FRAME_PANEL_STYLE, METAL_SHEET, MetalTopLeft, compose_metal_sheet, metal_frame_style,
    };
    let pixels = compose_metal_sheet(MetalTopLeft::Portrait, |fdid| {
        let image = assets::decode_blp(&format!("data/textures/{fdid}.blp"))?;
        Ok((image.pixels, image.width))
    })?;
    let (width, height) = METAL_SHEET;
    let sheet = registry
        .create_dynamic_texture(width, height, pixels)
        .map_err(|error| format!("Metal frame sheet: {error}"))?;
    registry.register_panel_style(
        METAL_FRAME_PANEL_STYLE,
        metal_frame_style(TextureSource::Dynamic(sheet)),
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
