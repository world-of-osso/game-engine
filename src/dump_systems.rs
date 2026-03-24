use bevy::{camera::primitives::Aabb, picking::mesh_picking::ray_cast::MeshRayCast, prelude::*};

use crate::game_state;
use crate::{DumpSceneFlag, DumpTreeFlag, DumpUiTreeFlag};

#[allow(clippy::type_complexity)]
pub fn dump_tree_and_exit(
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    automation_queue: Option<Res<game_engine::ui::automation::UiAutomationQueue>>,
    mut exit: MessageWriter<AppExit>,
) {
    if automation_queue.is_some_and(|q| !q.0.is_empty()) {
        return;
    }
    let tree = game_engine::dump::build_tree(&tree_query, &parent_query, None);
    if tree.trim().is_empty() {
        return;
    }
    println!("{tree}");
    exit.write(AppExit::Success);
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn dump_scene_and_exit(
    tree: Option<Res<game_engine::scene_tree::SceneTree>>,
    transforms: Query<&Transform>,
    global_transforms: Query<&GlobalTransform>,
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut ray_cast: MeshRayCast,
    automation_queue: Option<Res<game_engine::ui::automation::UiAutomationQueue>>,
    state: Res<State<game_state::GameState>>,
    time: Res<Time>,
    mut entered_at: Local<Option<f64>>,
    mut exit: MessageWriter<AppExit>,
) {
    if automation_queue.is_some_and(|q| !q.0.is_empty()) {
        return;
    }
    if let Some(tree) = tree {
        println!(
            "{}",
            game_engine::dump::build_scene_tree(
                &tree,
                &transforms,
                &global_transforms,
                &parent_query,
                &aabb_query,
                &camera_query,
                &mut ray_cast,
            )
        );
        exit.write(AppExit::Success);
        return;
    }
    if *state.get() != game_state::GameState::InWorld {
        return;
    }
    let now = time.elapsed_secs_f64();
    if now - *entered_at.get_or_insert(now) < 5.0 {
        return;
    }
    println!(
        "{}",
        game_engine::dump::build_tree(&tree_query, &parent_query, None)
    );
    exit.write(AppExit::Success);
}

pub fn dump_ui_tree_and_exit(
    mut ui_state: ResMut<game_engine::ui::plugin::UiState>,
    mut spellbook_runtime: Option<
        NonSendMut<game_engine::ui::spellbook_runtime::SpellbookUiRuntime>,
    >,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(ref mut rt) = spellbook_runtime {
        rt.sync(&mut ui_state.registry);
    }
    crate::action_bar::ensure_action_bars(&mut ui_state.registry);
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

pub fn headless_dump_ui_tree_immediate(ui_state: ResMut<game_engine::ui::plugin::UiState>) {
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    std::process::exit(0);
}

#[allow(clippy::type_complexity)]
pub fn handle_automation_dump_tree_request(
    request: Option<Res<game_engine::ui::automation::UiAutomationDumpTreeRequest>>,
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if request.is_none() {
        return;
    }
    commands.remove_resource::<game_engine::ui::automation::UiAutomationDumpTreeRequest>();
    let tree = game_engine::dump::build_tree(&tree_query, &parent_query, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

pub fn handle_automation_dump_ui_tree_request(
    request: Option<Res<game_engine::ui::automation::UiAutomationDumpUiTreeRequest>>,
    mut ui_state: ResMut<game_engine::ui::plugin::UiState>,
    mut spellbook_runtime: Option<
        NonSendMut<game_engine::ui::spellbook_runtime::SpellbookUiRuntime>,
    >,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if request.is_none() {
        return;
    }
    commands.remove_resource::<game_engine::ui::automation::UiAutomationDumpUiTreeRequest>();
    if let Some(ref mut rt) = spellbook_runtime {
        rt.sync(&mut ui_state.registry);
    }
    crate::action_bar::ensure_action_bars(&mut ui_state.registry);
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

/// Insert dump/screenshot flags and register their systems.
pub fn configure_dump_systems(
    app: &mut App,
    dump_tree: bool,
    dump_ui_tree: bool,
    dump_scene: bool,
    screenshot: Option<crate::ScreenshotRequest>,
) {
    app.add_systems(Update, handle_automation_dump_tree_request);
    app.add_systems(Update, handle_automation_dump_ui_tree_request);
    if dump_tree {
        app.insert_resource(DumpTreeFlag);
        app.add_systems(Update, dump_tree_and_exit);
    }
    if dump_ui_tree {
        app.insert_resource(DumpUiTreeFlag);
        app.add_systems(PostStartup, dump_ui_tree_and_exit);
    }
    if dump_scene {
        app.insert_resource(DumpSceneFlag);
        app.add_systems(Update, dump_scene_and_exit);
    }
    if let Some(req) = screenshot {
        app.insert_resource(req);
        app.add_systems(Update, crate::take_screenshot);
    }
}
