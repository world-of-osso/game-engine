use shared::protocol::GossipMenu;

use super::*;

const WILLEM: u64 = 823;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<NpcInteractionRequest>()
        .init_resource::<QuestRuntime>()
        .init_resource::<WindowManager>();
    app
}

/// Runs the reconciler as one registered system so its `Local` persists like in the app.
fn run_window_sync(app: &mut App) {
    let id = match app.world().get_resource::<WindowSyncSystem>() {
        Some(system) => system.0,
        None => {
            let id = app.world_mut().register_system(sync_quest_giver_window);
            app.world_mut().insert_resource(WindowSyncSystem(id));
            id
        }
    };
    app.world_mut().run_system(id).unwrap();
}

#[derive(Resource)]
struct WindowSyncSystem(bevy::ecs::system::SystemId);

fn open_greeting(app: &mut App) {
    app.world_mut().resource_mut::<QuestRuntime>().open_gossip(
        WILLEM,
        "Deputy Willem".into(),
        GossipMenu {
            menu_id: 0,
            text: String::new(),
            options: vec![],
        },
    );
}

fn requests(app: &mut App) -> Vec<NpcInteractionRequest> {
    app.world_mut()
        .resource_mut::<Messages<NpcInteractionRequest>>()
        .drain()
        .collect()
}

#[test]
fn dialog_opens_the_quest_frame_and_escape_ends_the_interaction() {
    let mut app = app();
    open_greeting(&mut app);
    run_window_sync(&mut app);
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::QuestGiver)
    );
    assert_eq!(
        app.world()
            .resource::<WindowManager>()
            .panel_slot(WindowId::QuestGiver),
        Some(0),
        "npc-driven panel takes slot L"
    );

    // Escape closes every window.
    app.world_mut().resource_mut::<WindowManager>().close_all();
    run_window_sync(&mut app);
    assert!(app.world().resource::<QuestRuntime>().dialog.is_none());
    assert_eq!(
        requests(&mut app),
        vec![NpcInteractionRequest::Close { npc: WILLEM }]
    );
}

#[test]
fn server_closed_dialog_closes_the_window_and_a_new_dialog_reopens_it() {
    let mut app = app();
    open_greeting(&mut app);
    run_window_sync(&mut app);
    app.world_mut()
        .resource_mut::<QuestRuntime>()
        .close_dialog_for(WILLEM);
    run_window_sync(&mut app);
    assert!(
        !app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::QuestGiver)
    );
    assert!(
        requests(&mut app).is_empty(),
        "server-side close sends nothing back"
    );

    open_greeting(&mut app);
    run_window_sync(&mut app);
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::QuestGiver)
    );
}

#[test]
fn header_names_resolve_area_ids() {
    game_engine::world_db::import_zone_name_cache().expect("import zone name cache");
    assert_eq!(quest_header_name(12), "Elwynn Forest");
    assert_eq!(quest_header_name(-61), "Unknown");
}
