//! Native loading presentation. Progress and screen lifecycle remain caller-owned.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use game_engine::ui::native::NativeUiElement;
use game_engine::ui::screens::loading_component::{LoadingScreenLayout, LoadingScreenState};
use ui_toolkit::render::LoadedTexture;

#[path = "native_assets.rs"]
mod assets;
pub(super) use assets::LoadingViewAssets;

#[derive(Clone)]
struct TextView {
    bounds: Entity,
    text: Entity,
}

#[derive(Resource, Clone)]
pub(super) struct LoadingView {
    pub root: Entity,
    pub camera: Entity,
    top: Entity,
    bottom: Entity,
    matte: Entity,
    art: Entity,
    logo: Entity,
    bar: Entity,
    shell_left: Entity,
    shell_center: Entity,
    shell_right: Entity,
    clip: Entity,
    fill: Entity,
    zone: TextView,
    status: TextView,
    progress: TextView,
    tip: TextView,
}

struct LoadingNodes {
    top: Node,
    bottom: Node,
    matte: Node,
    art: Node,
    logo: Node,
    bar: Node,
    shell_left: Node,
    shell_center: Node,
    shell_right: Node,
    clip: Node,
    fill: Node,
    zone: Node,
    status: Node,
    progress: Node,
    tip: Node,
}

fn centered(width: f32, height: f32, x: f32, y: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: percent(50),
        top: percent(50),
        margin: UiRect {
            left: px(x - width / 2.0),
            top: px(y - height / 2.0),
            ..default()
        },
        width: px(width),
        height: px(height),
        ..default()
    }
}

fn positioned(x: f32, y: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        width: px(width),
        height: px(height),
        ..default()
    }
}

fn layout_nodes(layout: &LoadingScreenLayout, progress: u8) -> LoadingNodes {
    let art_top = 50.0 - layout.art_height / 2.0;
    let art_bottom = 50.0 + layout.art_height / 2.0;
    let bar_bottom = art_bottom - layout.bar_y;
    let bar_top = bar_bottom - layout.bar_height;
    let bar_center = bar_bottom - layout.bar_height / 2.0;
    // The original loading_bar_shell style fixes caps at 25px; its layout override is unused.
    let cap = 25.0;
    LoadingNodes {
        top: centered(
            layout.filler_width,
            layout.filler_height,
            0.0,
            art_top - layout.filler_top_y - layout.filler_height / 2.0,
        ),
        bottom: centered(
            layout.filler_width,
            layout.filler_height,
            0.0,
            art_bottom - layout.filler_bottom_y + layout.filler_height / 2.0,
        ),
        matte: centered(1328.0, 704.0, 0.0, 18.0),
        art: centered(layout.art_width, layout.art_height, 0.0, 50.0),
        logo: centered(360.0, 140.0, 0.0, art_top - layout.logo_y - 70.0),
        bar: centered(layout.bar_width, layout.bar_height, 0.0, bar_center),
        shell_left: positioned(0.0, 0.0, cap, layout.bar_height),
        shell_center: positioned(cap, 0.0, layout.bar_width - 2.0 * cap, layout.bar_height),
        shell_right: positioned(layout.bar_width - cap, 0.0, cap, layout.bar_height),
        clip: Node {
            overflow: Overflow::clip(),
            ..positioned(
                layout.bar_fill_start_x,
                (layout.bar_height - layout.bar_fill_height) / 2.0,
                layout.bar_fill_max_width,
                layout.bar_fill_height,
            )
        },
        fill: positioned(
            0.0,
            0.0,
            layout.bar_fill_max_width * f32::from(progress.min(100)) / 100.0,
            layout.bar_fill_height,
        ),
        zone: centered(560.0, 28.0, 0.0, bar_top - layout.zone_text_y - 14.0),
        status: centered(420.0, 20.0, 0.0, bar_center - layout.status_text_y),
        progress: centered(
            90.0,
            18.0,
            layout.bar_width / 2.0 + layout.progress_text_x - 45.0,
            bar_center - layout.progress_text_y,
        ),
        tip: centered(980.0, 22.0, 0.0, bar_bottom - layout.tip_text_y + 11.0),
    }
}

fn image(texture: &LoadedTexture) -> ImageNode {
    ImageNode {
        image: texture.handle.clone(),
        rect: texture.rect,
        image_mode: NodeImageMode::Stretch,
        ..default()
    }
}

/// All assets load before entity creation; errors leave no partial native hierarchy.
pub(super) fn spawn_loading_view(
    commands: &mut Commands,
    assets: &mut LoadingViewAssets,
    state: &LoadingScreenState,
    layout: &LoadingScreenLayout,
    camera_order: isize,
) -> Result<LoadingView, String> {
    let art = assets.load()?;
    let nodes = layout_nodes(layout, state.progress_percent);
    let camera = commands
        .spawn((
            Name::new("NativeLoadingCamera"),
            Camera2d,
            Camera {
                order: camera_order,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            RenderLayers::none(),
        ))
        .id();
    Ok(ui_toolkit::rsx! {
        @native(commands, None) {
            node {
                id: root, name: "LoadingRoot", width: percent(100), height: percent(100),
                components: (NativeUiElement, UiTargetCamera(camera), BackgroundColor(Color::BLACK)),
                node { id: top, name: "LoadingTopFiller", layout: nodes.top,
                    components: (NativeUiElement, image(&art.top), ZIndex(0)), }
                node { id: bottom, name: "LoadingBottomFiller", layout: nodes.bottom,
                    components: (NativeUiElement, image(&art.bottom), ZIndex(0)), }
                node { id: matte, name: "LoadingArtworkMatte", layout: nodes.matte,
                    components: (NativeUiElement, BackgroundColor(Color::srgba(0.0,0.0,0.0,0.82)), ZIndex(1)), }
                node { id: artwork, name: "LoadingArtwork", layout: nodes.art,
                    components: (NativeUiElement, image(&art.art), ZIndex(2)), }
                node { id: logo, name: "LoadingLogo", layout: nodes.logo,
                    components: (NativeUiElement, image(&art.logo), ZIndex(3)), }
                node {
                    id: bar, name: "LoadingBarBackground", layout: nodes.bar,
                    components: (NativeUiElement, ZIndex(4)),
                    node { id: shell_left, name: "LoadingBarShellLeft", layout: nodes.shell_left,
                        components: (NativeUiElement, image(&art.left)), }
                    node { id: shell_center, name: "LoadingBarShellCenter", layout: nodes.shell_center,
                        components: (NativeUiElement, image(&art.center)), }
                    node { id: shell_right, name: "LoadingBarShellRight", layout: nodes.shell_right,
                        components: (NativeUiElement, image(&art.right)), }
                    node {
                        id: clip, name: "LoadingBarFillClip", layout: nodes.clip,
                        components: (NativeUiElement, ZIndex(1)),
                        node { id: fill, name: "LoadingBarFill", layout: nodes.fill,
                            components: (NativeUiElement, image(&art.fill)), }
                    }
                }
                {
                    let gold = Color::srgb(1.0,0.82,0.0);
                    let zone = spawn_text(commands, root, "LoadingZoneText", &state.zone_text,
                        nodes.zone, &art.font, 22.0, gold);
                    let status = spawn_text(commands, root, "LoadingStatusText", &state.status_text,
                        nodes.status, &art.font, 13.0, Color::srgb(0.95,0.9,0.78));
                    // Legacy status is Medium and precedes the shell; retain its occlusion.
                    commands.entity(status.bounds).insert(ZIndex(3));
                    let progress = spawn_text(commands, root, "LoadingProgressText", &format!("{}%",state.progress_percent),
                        nodes.progress, &art.font, 15.0, gold);
                    let tip = spawn_text(commands, root, "LoadingTipText", &state.tip_text,
                        nodes.tip, &art.font, 14.0, Color::srgb(0.78,0.74,0.66));
                }
            }
        } => LoadingView { root, camera, top, bottom, matte, art: artwork, logo, bar,
            shell_left, shell_center, shell_right, clip, fill, zone, status, progress, tip }
    })
}

fn spawn_text(
    commands: &mut Commands,
    parent: Entity,
    name: &str,
    value: &str,
    node: Node,
    font: &Handle<Font>,
    size: f32,
    color: Color,
) -> TextView {
    ui_toolkit::rsx! {
        @native(commands, Some(parent)) {
            node {
                id: bounds, name: format!("{name}Bounds"), layout: node,
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
                components: (NativeUiElement, ZIndex(5)),
                node {
                    id: text, name: name.to_owned(),
                    components: (NativeUiElement, Text::new(value), TextColor(color),
                        TextFont { font: bevy::text::FontSource::Handle(font.clone()), font_size: FontSize::Px(size), ..default() },
                        TextLayout::new(Justify::Center, bevy::text::LineBreak::NoWrap)),
                }
            }
        } => TextView { bounds, text }
    }
}

pub(super) fn sync_loading_view(
    world: &mut World,
    view: &LoadingView,
    state: &LoadingScreenState,
    layout: &LoadingScreenLayout,
) {
    let nodes = layout_nodes(layout, state.progress_percent);
    for (entity, node) in [
        (view.top, nodes.top),
        (view.bottom, nodes.bottom),
        (view.matte, nodes.matte),
        (view.art, nodes.art),
        (view.logo, nodes.logo),
        (view.bar, nodes.bar),
        (view.shell_left, nodes.shell_left),
        (view.shell_center, nodes.shell_center),
        (view.shell_right, nodes.shell_right),
        (view.clip, nodes.clip),
        (view.fill, nodes.fill),
    ] {
        update_node(world, entity, node);
    }
    for (text_view, node, value) in [
        (&view.zone, nodes.zone, state.zone_text.clone()),
        (&view.status, nodes.status, state.status_text.clone()),
        (
            &view.progress,
            nodes.progress,
            format!("{}%", state.progress_percent),
        ),
        (&view.tip, nodes.tip, state.tip_text.clone()),
    ] {
        update_node(
            world,
            text_view.bounds,
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..node
            },
        );
        let mut text = world
            .get_mut::<Text>(text_view.text)
            .expect("loading text exists until teardown");
        if text.0 != value {
            text.0 = value;
        }
    }
}

fn update_node(world: &mut World, entity: Entity, value: Node) {
    let mut node = world
        .get_mut::<Node>(entity)
        .expect("loading node exists until teardown");
    if *node != value {
        *node = value;
    }
}

#[cfg(test)]
#[path = "native_view_tests.rs"]
mod tests;
