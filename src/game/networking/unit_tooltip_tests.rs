use std::sync::mpsc;

use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;

use super::*;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CreatureTooltipCache>()
        .init_resource::<AccountAppearances>()
        .init_resource::<ConnectionSender>()
        .init_resource::<Inbox<CreatureTooltip>>()
        .init_resource::<Inbox<AppearanceCollectionUpdate>>();
    app
}

fn defias_thug() -> CreatureTooltip {
    CreatureTooltip {
        entry: 38,
        subname: String::new(),
        creature_type: 7,
        drops: Vec::new(),
        vendor_items: Vec::new(),
    }
}

#[test]
fn an_entry_is_asked_for_once_and_its_answer_is_cached() {
    let mut app = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    {
        let mut cache = app.world_mut().resource_mut::<CreatureTooltipCache>();
        cache.request(38);
        cache.request(38);
    }
    app.world_mut()
        .run_system_once(send_tooltip_queries)
        .unwrap();
    assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    assert!(commands.try_recv().is_err(), "one query per entry");
    app.world_mut()
        .resource_mut::<CreatureTooltipCache>()
        .request(38);
    app.world_mut()
        .run_system_once(send_tooltip_queries)
        .unwrap();
    assert!(commands.try_recv().is_err(), "never asked again");

    app.world_mut()
        .insert_resource(Inbox::new(vec![defias_thug()]));
    app.world_mut()
        .run_system_once(receive_tooltip_data)
        .unwrap();

    let cache = app.world().resource::<CreatureTooltipCache>();
    assert_eq!(cache.get(38), Some(&defias_thug()));
    assert_eq!(cache.get(54), None);
}

#[test]
fn a_collection_update_replaces_the_account_appearances() {
    let mut app = fixture();
    app.world_mut().insert_resource(Inbox::new(vec![
        AppearanceCollectionUpdate {
            appearances: vec![154],
        },
        AppearanceCollectionUpdate {
            appearances: vec![154, 646],
        },
    ]));

    app.world_mut()
        .run_system_once(receive_tooltip_data)
        .unwrap();

    let appearances = &app.world().resource::<AccountAppearances>().0;
    assert!(appearances.has(154) && appearances.has(646));
    assert_eq!(appearances.count(), 2);
}
