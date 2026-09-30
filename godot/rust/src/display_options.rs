//! Native display settings shared by startup and live Options edits.

use game_engine_core::client_options_data::{
    GraphicsOptionsFile, MAX_FRAME_RATE_LIMIT, MIN_FRAME_RATE_LIMIT,
};
use godot::classes::{
    CanvasLayer, ColorRect, DisplayServer, Engine, Node, PackedScene, ResourceLoader, Shader,
    ShaderMaterial, Viewport,
    control::{LayoutPreset, MouseFilter},
    display_server::VSyncMode,
};
use godot::prelude::*;

pub(crate) fn apply_graphics_display_options(
    graphics: &GraphicsOptionsFile,
    viewport: &mut Gd<Viewport>,
) {
    let scale = graphics.clone().clamped().render_scale;
    viewport.set_scaling_3d_scale(scale);
    update_rcas_layer(viewport, scale);
    update_bloom(viewport, graphics);

    let vsync_mode = if graphics.vsync_enabled {
        VSyncMode::MAILBOX
    } else {
        VSyncMode::DISABLED
    };
    DisplayServer::singleton().window_set_vsync_mode(vsync_mode);

    let max_fps = if graphics.frame_rate_limit_enabled {
        i32::from(
            graphics
                .frame_rate_limit
                .clamp(MIN_FRAME_RATE_LIMIT, MAX_FRAME_RATE_LIMIT),
        )
    } else {
        0
    };
    Engine::singleton().set_max_fps(max_fps);
}

const RCAS_LAYER_NAME: &str = "NativeRcasLayer";
const BLOOM_CONTROLLER_NAME: &str = "NativeBloom";

fn update_bloom(viewport: &mut Gd<Viewport>, graphics: &GraphicsOptionsFile) {
    let existing = viewport.try_get_node_as::<Node>(BLOOM_CONTROLLER_NAME);
    if existing.is_none() && !graphics.bloom_enabled {
        return;
    }
    let mut controller = existing.unwrap_or_else(|| spawn_bloom_controller(viewport));
    controller.call(
        "configure",
        &[
            graphics.bloom_enabled.to_variant(),
            graphics.bloom_intensity.clamp(0.0, 1.0).to_variant(),
        ],
    );
}

fn spawn_bloom_controller(viewport: &mut Gd<Viewport>) -> Gd<Node> {
    let scene = ResourceLoader::singleton()
        .load("res://scenes/bloom.tscn")
        .expect("Cannot load native bloom controller scene")
        .try_cast::<PackedScene>()
        .expect("Native bloom controller is not a PackedScene");
    let mut controller = scene
        .instantiate()
        .expect("Cannot instantiate native bloom controller");
    controller.set_name(BLOOM_CONTROLLER_NAME);
    viewport.add_child(&controller);
    controller
}

fn update_rcas_layer(viewport: &mut Gd<Viewport>, scale: f32) {
    let existing = viewport.try_get_node_as::<CanvasLayer>(RCAS_LAYER_NAME);
    if scale >= 0.999 {
        if let Some(mut layer) = existing {
            layer.hide();
        }
        return;
    }
    if let Some(mut layer) = existing {
        layer.show();
        return;
    }

    let shader = ResourceLoader::singleton()
        .load("res://shaders/rcas.gdshader")
        .expect("Cannot load native RCAS shader")
        .try_cast::<Shader>()
        .expect("Native RCAS shader has wrong resource type");
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);

    let mut layer = CanvasLayer::new_alloc();
    layer.set_name(RCAS_LAYER_NAME);
    layer.set_layer(-100);
    let mut rect = ColorRect::new_alloc();
    rect.set_anchors_preset(LayoutPreset::FULL_RECT);
    rect.set_mouse_filter(MouseFilter::IGNORE);
    rect.set_material(&material);
    layer.add_child(&rect);
    viewport.add_child(&layer);
}
