use crate::ui::anchor::Anchor;
use crate::ui::layout::LayoutRect;
use crate::ui::strata::{DrawLayer, FrameStrata};

/// WoW widget types corresponding to frame XML element names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetType {
    Frame,
    Button,
    CheckButton,
    Texture,
    FontString,
    Line,
    EditBox,
    ScrollFrame,
    Slider,
    StatusBar,
    Cooldown,
    Model,
    PlayerModel,
    ModelScene,
    ColorSelect,
    MessageFrame,
    SimpleHTML,
    GameTooltip,
    Minimap,
}

/// A UI frame in the WoW frame hierarchy.
pub struct Frame {
    pub id: u64,
    pub name: Option<String>,
    pub widget_type: WidgetType,

    // Hierarchy
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,

    // Layout
    pub width: f32,
    pub height: f32,
    pub anchors: Vec<Anchor>,
    pub layout_rect: Option<LayoutRect>,

    // Visibility
    pub shown: bool,
    pub visible: bool,

    // Alpha
    pub alpha: f32,
    pub effective_alpha: f32,

    // Scale
    pub scale: f32,
    pub effective_scale: f32,

    // Strata and layering
    pub strata: FrameStrata,
    pub frame_level: i32,
    pub raise_order: i32,
    pub draw_layer: DrawLayer,
    pub draw_sub_layer: i32,

    // Input
    pub mouse_enabled: bool,
    pub keyboard_enabled: bool,
    pub hit_rect_insets: [f32; 4],

    // Appearance
    pub background_color: Option<[f32; 4]>,

    // Behavior
    pub clamped_to_screen: bool,
    pub movable: bool,
    pub resizable: bool,
}

impl Frame {
    pub fn new(id: u64, name: Option<String>, widget_type: WidgetType) -> Self {
        Self {
            id,
            name,
            widget_type,
            parent_id: None,
            children: Vec::new(),
            width: 0.0,
            height: 0.0,
            anchors: Vec::new(),
            layout_rect: None,
            shown: true,
            visible: true,
            alpha: 1.0,
            effective_alpha: 1.0,
            scale: 1.0,
            effective_scale: 1.0,
            strata: FrameStrata::default(),
            frame_level: 0,
            raise_order: 0,
            draw_layer: DrawLayer::default(),
            draw_sub_layer: 0,
            mouse_enabled: true,
            keyboard_enabled: false,
            hit_rect_insets: [0.0; 4],
            background_color: None,
            clamped_to_screen: false,
            movable: false,
            resizable: false,
        }
    }
}
